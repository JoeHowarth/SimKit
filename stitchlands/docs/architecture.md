# Stitchlands Technical Architecture & High‑Level Plan

This document assumes familiarity with `stitchlands/vision.md` and describes the architecture, determinism strategy, and implementation boundaries between the reusable `simkit-core` library and the `stitchlands` game crate.

## Crates and Responsibilities

- simkit-core: Reusable simulation utilities and primitives.
  - Grid and coordinates (`Grid2D`, `TileId`), occupancy and reservations, deterministic A\* pathfinding (8-way), optional tilemap rendering helpers, typed ID utilities, snapshot extraction helpers.
- stitchlands (bin): Game-specific plugins, content, CLI, scenarios, and tests.
  - Scenario loading/cleanup, task board and needs daemon, scheduler, telemetry, snapshot hashing, and headless/live orchestration.

## Plugin Stack

- KitCorePlugin (from simkit-core): Provides app states, playback, and core system sets.
- StitchlandsCorePlugin (new, in stitchlands): Adds world, tasks, scheduler, telemetry, scenario integration.

## Schedules and State Gating

- AppState: All simulation systems run only in `AppState::InGame`.
- Schedules:
  - FixedUpdate: All simulation mutations; subdivided into KitSystemSet phases:
    - PreStep: gather facts, reset edit budget, spawn tasks from designations/needs, prune/prepare.
    - Step: core logic (assignment, job progression, needs interrupts).
    - PostStep: commit results, release stale reservations, sample telemetry, emit events.
  - Update: UI/visuals only (no simulation changes).

System labels (proposed)

- PreStep: `reset_edit_budget`, `needs_daemon_emit`, `designation_spawner`, `task_prune`, `path_cache_prepare`.
- Step: `hard_need_interrupts`, `scheduler_assign`, `job_tick`.
- PostStep: `release_stale_reservations`, `telemetry_sample`.

## Determinism Strategy

- RNG discipline: `SmallRng` seeded from CLI/scenario; used only in FixedUpdate.
- Stable iteration: Stable ordering for pawns, tasks, tiles during scheduling and extraction; explicit tie-breakers by ID.
- Typed IDs: Newtype identifiers provide stable cross-run identity and scenario authoring:
  - PawnId, TaskId, ItemId, BlueprintId, ZoneId, BedId, etc. (u64 newtypes).
  - Entities carry their typed ID as a component.
  - IdAllocator<T> (monotonic) and IdIndex<T> (Id→Entity map) resources.
  - Scenarios may auto-assign IDs when omitted; allocator makes this deterministic.
- Snapshot extraction:
  - Extract relevant components/resources into canonical containers.
  - Sort by typed IDs (or coordinates for tiles), serialize in a canonical form, and hash (SHA‑256).
  - Paired with golden hashes stored next to scenarios for deterministic integration tests and future save/load.
  - Round-trip support: the same serializable snapshot struct is used for extraction and for loading back into a world.

Typed ID specifics

- Newtypes (examples): `struct PawnId(pub u64); struct TaskId(pub u64); struct ItemId(pub u64); struct BlueprintId(pub u64); struct ZoneId(pub u64); struct BedId(pub u64);`
- Per-type allocator: `struct IdAllocator<T> { next: u64 }` with deterministic monotonic assignment (no reuse).
- Per-type index: `struct IdIndex<T>(HashMap<Id<T>, Entity>)` kept in sync on spawn/despawn.
- Components: entities carry exactly one typed ID that matches their domain (e.g., `PawnId` on pawns).
- Policy: allocator defaults start at 1000 to avoid clashing with authored IDs in scenarios. Completion of scenarios also assigns missing IDs from 1000 upward.

## Coordinates, Grid, and Pathfinding (in simkit)

- Coordinates: Bevy-style origin with (0,0) at bottom-left; +X right, +Y up.
- TileId: Newtype with `{ x: u32, y: u32 }`; independent of width; suitable for dynamic map sizes.
- Grid2D<T>: Generic 2D container with `GridConfig { size: UVec2 }` and index helpers; internal linearization for performance.
- Occupancy/Reservations:
  - Occupancy bitflags (pawn/item/building) and `EntityTileLink { tile: TileId }` components.
  - Simple reservation helpers for unique targets and item quantities.
- Pathfinding:
  - Deterministic A\* with 8-way neighbors (fixed), move costs via `Passable`/`MoveCost` traits.
  - Edge costs: cardinal = 10, diagonal = 14 (integer weights to avoid floats).
  - Heuristic: octile metric `h = 10*(dx + dy) + (14 - 20) * min(dx, dy)`.
  - Tie-breaking: open-set ordered by `(f, h, tile_id)` to stabilize exploration; reconstruct path deterministically.
  - No flow fields in Phase 0; future extension path remains open.
- Rendering helpers:
  - Optional bevy_ecs_tilemap scaffolding utilities, used only by stitchlands in live mode.

## Agents and Needs

- Pawn: `{ PawnId, position: TileId, needs, skills, inventory, job? }`.
- Needs: minimal set for Phase 0+ (hunger, rest; mood/health introduced later).
- WorkPriorities: per-work-type weights; safety pins for critical work types later.

Components (initial)

- `struct Pawn { id: PawnId }`
- `TileId { x, y }`
- `struct Needs { hunger: f32, rest: f32 }` // 0..1 where 0 = empty, 1 = full
- `struct Skills { /* placeholder for phase 0 */ }`
- `struct Inventory { /* simple carried stack(s) */ }`
- `enum Job { /* per-task driver states, see Tasks */ }`

## Tasks, Designations, and Jobs (per task-system-design)

- Single authoritative task list: `Task { TaskId, kind, args, work_type, priority, status, owner? }`.
- Designations and bills spawn tasks idempotently; a needs daemon emits Eat/Sleep tasks with urgency scaling.
- Reservations:
  - Item quantity reservations and unique target locks (e.g., BlueprintId, BedId); released on suspend/cancel/complete.
  - Invariants: reserved+available ≤ actual; unique targets have ≤1 owner; mid-step preemption prohibited except hard needs.
- Job drivers: Small state machines with explicit checkpoints; soft preemption at checkpoints with hysteresis.

Types (initial)

- `enum TaskKind { Harvest { target: TileId }, Haul { from: TileId, to: TileId, item: ItemKind, qty: u32 }, BuildBasic { blueprint: BlueprintId }, Clean { area: RectI32 }, EatAuto { owner: PawnId }, SleepAuto { owner: PawnId } }`
- `enum TaskStatus { Pending, Running, Suspended, Completed, Failed { reason: String }, Cancelled }`
- `enum WorkType { Hauling, Construction, Cooking, Cleaning, Needs }`
- `struct WorkPriorities(HashMap<WorkType, i32>)` where higher = more preferred (sane defaults).
- `struct Task { id: TaskId, kind: TaskKind, work_type: WorkType, priority_label: i32, status: TaskStatus, owner: Option<PawnId>, issued_at_tick: i64 }`

Reservations and inventory (initial)

- Unique target reservations: `HashMap<TargetKey, PawnId>` where `TargetKey` can wrap `BlueprintId`, `BedId`, or `(TaskId, TileId)`.
- Item reservations: maintained per stockpile or global store as `{ item: ItemKind => reserved_qty }` and invariant checks.
- Invariants enforced at PostStep cleanup; all reservations released on suspend/cancel/complete.

## Scheduler (Greedy, Single-Pass)

- Assignable filter: Pending/Suspended, unassigned, and minimal gates satisfied for the pawn.
- Scoring: `score = wp + tp + u - α*d - β*s` where:
  - `wp` = per-pawn `WorkPriorities[work_type]` (default 0..3 range initially),
  - `tp` = task priority label (default 0 for Normal; Urgent may be +3),
  - `u` = type urgency (Eat: 1 - hunger, Sleep: 1 - rest; Build: +δ when materials ready),
  - `d` = Chebyshev distance in tiles to first step (matches 8-way A\*),
  - `s` = setup penalty (e.g., +2 for tasks likely to need hauling).
- Initial constants: `α = 1.0`, `β = 2.0`, hysteresis margin `γ = 2.0` (tune later; keep numeric and logged).
- Tie-breaking and stability:
  - Deterministic ordering of pawns and tasks; ties break by distance then TaskId.
  - Soft-lock current jobs unless the new candidate clears a margin; hard needs interrupt immediately.
- Edit budget: `EditBudget { per_tick, remaining }` caps assignments/rewrites to avoid thrash.

## Telemetry and Observability

- Metrics resource sampled in PostStep: food buffer days, avg job wait time, idle ratio, backlog by type; extended in later phases.
- Events/logging for assignments and preemptions with score terms for debugging.

Metric definitions (initial)

- `food_buffer_days`: estimated days = `total_calories_available / (consumption_rate_per_day * population)`.
- `avg_job_wait_time`: rolling average ticks between `issued_at_tick` and first assignment.
- `idle_ratio`: fraction of pawns without a job during the last tick window.
- `backlog_by_type`: counts of `TaskStatus::Pending` by `TaskKind`/`WorkType`.

## Scenarios (RON) and CLI

- Scenario model: dynamic map size; explicit tiles/entities/zones/inventories/priorities; optional seeds; auto-ID assignment allowed. On disk the type name is `Scenario` (serde-renamed), which maps to the editable `ScenarioDef` in code; completion of defaults happens at load/spawn time (no separate completed struct).
- Loader: `OnEnter(InGame)` spawns world and indexes; `OnExit(InGame)` fully despawns tagged runtime entities and clears resources.
- CLI:
  - Flags: `--mode {live|headless}` (default: live), `--scenario <path.ron>`, `--ticks <N>` (required for headless), `--seed <u64>` (default 1).
  - Examples: `stitchlands --mode headless --scenario assets/scenarios/p0/basic.ron --ticks 200 --seed 1`.
  - Headless runs FixedUpdate only; live adds rendering and UI.
  - Always use 8-way A\*; no CLI override.

Scenario RON schema (initial, illustrative)

```
Scenario(
  sim_seed: Some(1),
  map: (
    size: (x: 64, y: 64),
    tiles: [ (pos: (x: 10, y: 10), terrain: Grass, walkable: true) ],
  ),
  pawns: [
    (
      id: Some(1), name: "Ada", pos: (x: 12, y: 10),
      needs: (hunger: 0.6, rest: 0.9),
      priorities: { Cooking: 3, Hauling: 2 },
    ),
  ],
  items: [ (id: None, kind: Grain, qty: 50, pos: (x: 5, y: 5)) ],
  zones: [ (id: None, kind: Stockpile, rect: ((x: 4, y: 4), (x: 8, y: 8)), filters: [Grain]) ],
  designations: [ Harvest((x: 20, y: 21)) ],
)
```

## Testing Strategy

- Unit tests (simkit): A\* determinism, occupancy/reservations, ID allocation/index, coordinate conversions.
- Integration tests (stitchlands): scenario → run N ticks headless → extract snapshot → hash and compare against golden; oracle checks on telemetry.

Golden meta format (RON)

```
GoldenMeta(
  sha256: "<64-hex-digits>",
  notes: Some("why this scenario matters / phase ref"),
)
```

Stored next to each scenario as `*.meta.ron`.

## Extensibility

- Data-driven content via RON for recipes, items, blueprints.
- Gradual layering per vision phases without changing core interfaces.
