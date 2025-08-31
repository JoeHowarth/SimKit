# Phase 0 Implementation Plan

This plan breaks Phase 0 into five subphases (0.a–0.e) focusing on foundational systems, determinism, and testability, aligned with `task-system-design.md` and the architecture.

## Principles
- Deterministic simulation confined to FixedUpdate; visuals only in Update.
- Typed IDs for cross-run stability; auto-assign allowed in scenarios.
- Bevy-style coordinates; TileId is a dedicated newtype `{ x, y }`.
- 8-way A* pathfinding; no flow fields in Phase 0.
- Snapshot extraction and golden hashes (FNV‑1a 64‑bit) for deterministic integration tests.

## 0.a App, CLI, Schedules
Goals
- Run live/headless from CLI; wire system sets; seed RNG; establish labels and ordering.

Deliverables
- `stitchlands/src/lib.rs`: `pub struct StitchlandsCorePlugin;`
  - Registers systems into `KitSystemSet::{PreStep, Step, PostStep}` gated by `AppState::InGame`.
  - Labels:
    - PreStep: `reset_edit_budget`, `needs_daemon_emit`, `designation_spawner`, `task_prune`, `path_cache_prepare` (some no-op until later subphases).
    - Step: `hard_need_interrupts`, `scheduler_assign`, `job_tick` (stub in 0.a).
    - PostStep: `release_stale_reservations`, `telemetry_sample` (stub in 0.a).
- `stitchlands/src/cli.rs`: parse flags `--mode {live|headless}` (default live), `--scenario <.ron>`, `--ticks <N>` (required in headless), `--seed <u64>` (default 1).
- Resources:
  - `RngResource(pub SmallRng)` initialized from CLI/scenario seed.
  - `EditBudget { per_tick: u32, remaining: u32 }` reset at PreStep.
  - `WorldTag` marker component reserved for runtime entities.

Systems
- PreStep: reset `EditBudget` to `per_tick`.
- Step: no-op placeholder.
- PostStep: no-op placeholder (later emits telemetry).

Tests (integration)
- Headless: run with fixed seed and N ticks; process exits 0 after exactly N FixedUpdate iterations.
- Determinism: two identical runs produce identical baseline snapshot hashes (empty world snapshot).

Status
- Completed. Implemented StitchlandsCore scaffolding, CLI with headless mode and tick limit, minimal FNV‑1a snapshot hash on exit, and passing integration tests for tick exit and determinism. simkit-core consolidated to KitCoreBase used by live and headless.

## 0.b Scenario Load + Cleanup
Goals
- Deterministic world boot and complete teardown on state exit.

Deliverables
- RON structs in `stitchlands/src/scenario/model.rs`:
  - `Scenario { sim_seed: Option<u64>, map: MapDef, pawns: Vec<PawnDef>, items: Vec<ItemDef>, zones: Vec<ZoneDef>, designations: Vec<DesignationDef>, defaults: DefaultsDef }`.
  - `MapDef { size: UVec2, tiles: Vec<TileDef> }` with explicit overrides (sparse list) and default terrain. Note: `size` is a concrete struct in RON (no `Some(...)` wrapper needed).
  - `TileDef { pos: TileId, terrain: Terrain, walkable: bool }`.
  - `PawnDef { id: Option<u64>, name: Option<String>, pos: TileId, needs: NeedsDef, priorities: HashMap<WorkType, i32> }`.
  - `ItemDef { id: Option<u64>, kind: ItemKind, qty: u32, pos: TileId }`.
  - `ZoneDef { id: Option<u64>, kind: ZoneKind, rect: (TileId, TileId), filters: Vec<ItemKind> }`.
  - `DesignationDef` mirrored to `TaskKind` where applicable.
- Loader in `stitchlands/src/scenario/loader.rs`:
  - `OnEnter(AppState::InGame)`: spawn map, pawns, items, zones; attach `WorldTag`; assign typed IDs using `IdAllocator<T>` for any missing IDs; populate `IdIndex<T>`.
  - Seed RNG: `sim_seed.or(cli_seed).unwrap_or(1)`.
- Typed IDs moved up to simkit-core in 0.b (earlier than planned in 0.c): `simkit-core::ids::{PawnId, ItemId, ZoneId, BlueprintId, BedId, TaskId}` along with `IdAllocator<T>` and `IdIndex<T>` resources. Stitchlands uses these during scenario load and cleans them on exit.
- ID policy: auto-assigned IDs start at 1000 to avoid clashes with authored IDs. After load, allocators are bumped beyond the maximum present ID.
- Cleanup in `stitchlands/src/scenario/cleanup.rs`:
  - `OnExit(AppState::InGame)`: despawn all entities with `WorldTag` (recursive), clear/zero:
    - `TaskBoard`, `ReservationMaps`, `IdIndex<T>` for all ID types, `IdAllocator<T>` (reset to post-max or 0 per policy), `Grid2D`, `Occupancy`, `RngResource`, `EditBudget`, `PathCache`, `Telemetry` accumulators.

Tests (integration)
- Enter→extract snapshot; Exit→assert no `WorldTag` entities and cleared resources; Re-enter→extract; snapshots match bit-for-bit.

## 0.c World Grid + A* + Reusable Core (simkit)
Goals
- Shared, deterministic grid/occupancy/pathfinding as library code.

Deliverables (simkit-core)
- `src/grid/mod.rs`: `Grid2D<T>`, `GridConfig`, index/coord conversion; `TileId { x, y }` newtype.
- `src/occupancy/mod.rs`: bitflags `Occupancy { PAWN, ITEM, BUILDING }`, component `EntityTileLink { tile: TileId }`, helpers `occupy(tile, flags)`, `vacate(tile, flags)`.
- `src/pathfinding/astar.rs`: deterministic 8-way A*; octile heuristic; costs 10/14; tie-breakers `(f, h, tile_id)`.
- `src/ids/mod.rs`: typed IDs; `IdAllocator<T>`, `IdIndex<T>`. (Implemented in 0.b)
- `src/snapshot/mod.rs`: helpers for canonical extraction (sorting and serialization utils).
- Optional `src/tilemap_render/*.rs` (live mode only) to build `bevy_ecs_tilemap` layers.

Integration (stitchlands)
- `world` glue uses simkit `Grid2D` and `TileId`; `Position` uses `TileId`.
- Enforce Bevy-style coords; forbid mutation of sim state from Update.
- Scenario completion fills missing fields deterministically:
  - Missing pawn/item positions: random tile within bounds with uniqueness (best effort, capped retries).
  - Missing zone rect: normalized 1x1 at a deterministic tile; all rects normalized (min/max) and clamped to map bounds.

Tests
- Unit (simkit): A* determinism on fixed maps; occupancy/vacate invariants and idempotence.

## 0.d Task Board Skeleton + Needs Daemon
Goals
- Central task list with idempotent spawners; minimal needs generation.

Deliverables
- `stitchlands/src/tasks/mod.rs` with:
  - `TaskId`, `Task`, `TaskStatus`, `TaskKind` (Harvest, Haul, BuildBasic, Clean, EatAuto, SleepAuto), `WorkType`.
  - `TaskBoard { tasks: Vec<Task>, next_id: u64 }` with stable iteration.
  - `Designation` components placed on tiles/entities produce tasks idempotently.
  - Reservation maps: `UniqueTargetRes`, `ItemReservations` with invariants and release helpers.
- Needs daemon in PreStep: emits `EatAuto`/`SleepAuto` with urgency scaling (1 - hunger/rest); hard-need interrupt flagging.

Systems
- PreStep:
  - `designation_spawner`: scan `Designation` components, create tasks if missing; attach back-references for idempotence.
  - `needs_daemon_emit`: emit needs tasks per pawn thresholds.
  - `task_prune`: remove invalid or completed tasks; release reservations.
- PostStep:
  - `release_stale_reservations`: ensure reservation invariants hold.

Tests (integration)
- Stable multiset of tasks from fixed designations.
- Unique target cannot be reserved by two pawns in the same tick; item reservation counts never exceed available.

## 0.e Scheduler Skeleton + Edit Budget + Telemetry Baseline
Goals
- Deterministic greedy assignment with stability via checkpoints; budgeted edits; minimal metrics.

Deliverables
- Assignable filter and scoring: `workPriority + taskPriority + typeUrgency - α*distance - β*setupPenalty` with deterministic iteration and tie-breakers `(distance, TaskId)`.
- Constants (initial): `α = 1.0`, `β = 2.0`, hysteresis margin `γ = 2.0` (log all values used).
- Soft preemption only at job checkpoints with hysteresis; hard needs can interrupt anywhere.
- `EditBudget` enforcement for assignments/rewrites.
- Telemetry: backlog by type, avg wait time, idle ratio; sampled in PostStep; printed in headless mode.
- Stub job drivers: minimal step machines that change status and set checkpoints without A* movement yet.

Systems
- Step:
  - `hard_need_interrupts`: suspend non-needs tasks when hard thresholds trip; release reservations.
  - `scheduler_assign`: single-pass greedy over ready pawns → assign best candidate.
  - `job_tick`: advance stub drivers; honor checkpoints.

Tests (integration)
- Deterministic assignment under fixed scenario/seed; edit budget caps reassignments; telemetry matches expectations after N ticks.

## Cross-Cutting: Snapshot Extraction and Golden Hashes
Extractor
- Collect: pawns (id, pos), items (id, pos, kind, qty), zones (id, rect). Extend later with tasks/telemetry.
- Sort: by typed IDs; never depend on Entity.
- Serialize: canonical JSON, compute FNV‑1a 64‑bit.

Golden Storage
- For each scenario, store a companion `*.meta.ron` with the expected hash and brief notes.
- Integration test: run N ticks headless, extract, compare hash with the golden in `*.meta.ron`.

Locations
- Scenarios: `assets/scenarios/phase0/*.ron`
- Goldens: `assets/scenarios/phase0/*.meta.ron`

Failure signals
- If hash mismatches: print both hash values and write the current snapshot to `target/snapshots/<scenario>.<tick>.json` for inspection.

## Proposed File Layout Changes
simkit-core
- src/grid/*.rs (Grid2D, TileId, GridConfig)
- src/occupancy/*.rs (bitflags, EntityTileLink, reservations)
- src/pathfinding/*.rs (A*, traits)
- src/ids/*.rs (typed IDs, allocators, indices)
- src/snapshot/*.rs (helpers to build canonical extractions; reusable pieces)

stitchlands
- src/lib.rs (re-exports; StitchlandsCorePlugin)
- src/main.rs (CLI wiring and app startup)
- src/cli.rs (arg parsing)
- src/scenario/*.rs (RON structs, loader, cleanup)
- src/world/*.rs (world-specific glue over simkit grid)
- src/tasks/*.rs (task board, kinds, reservations, needs daemon)
- src/scheduler/*.rs (assignable filter, scoring, assignment, job states)
- src/telemetry/*.rs (metrics, sampling)
- tests/integration/*.rs (headless runs vs golden)
