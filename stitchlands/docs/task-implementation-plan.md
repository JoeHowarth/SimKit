Approach

- Build a thin, playable slice: one real multi‑tick job (Harvest), a minimal scheduler, needs-driven hard interrupts, and only unique-target reservations.
- Keep code simple, explicit, and deterministic; compute urgency on read, not stored.
- Choose a single idempotence mechanism for spawners to avoid double-bookkeeping.

Idempotence Choice

- TaskRef (Option A): add TaskRef(Option<TaskId>) on designation entities; do not add an origin_map on the TaskBoard.
- Needs tasks avoid duplicates by scanning the board for an existing needs task for the pawn (no extra index).

Data Structures

- TaskId: monotonic via IdAllocator<TaskId>.
- TaskKind: Harvest { tile: TileId } | EatAuto { pawn: PawnId } | SleepAuto { pawn: PawnId }.
- TaskStatus: Pending | Running | Done | Cancelled.
- Task: { id: TaskId, kind: TaskKind, status: TaskStatus, priority: i32, owner: Option<PawnId> }.
    - owner is Some for Eat/Sleep (the pawn this task belongs to); None for Harvest.
- TaskBoard (Resource): { tasks: Vec<Task> }.
    - Methods: add(task) -> TaskId, get_mut(id), iter_sorted() (by TaskId), remove_if(f).
- Needs (Component): { hunger: f32, rest: f32 } (0..1, 1 = full).
- Designation (Component): enum { Harvest(TileId) }.
- TaskRef (Component): Option<TaskId>; maintained alongside Designation.
- Job (Component on Pawn):
    - { task: TaskId, driver: JobDriver }
    - JobDriver::Harvest { target: TileId, state: HarvestState, reserved: Option<UniqueTargetKey> }
    - JobDriver::EatAuto { ticks_left: u32 }
    - JobDriver::SleepAuto { ticks_left: u32 }
- HarvestState: ReserveTarget | TravelCountdown { ticks_left: u32 } | WorkCountdown { ticks_left: u32 }.
- UniqueTargetKey: Tile(TileId) | Entity(Entity) | Zone(ZoneId) | Blueprint(BlueprintId) (we’ll use Tile only in 0.d).
- UniqueTargetRes (Resource): owners: HashMap<UniqueTargetKey, TaskId>.
    - try_reserve(key, task_id) -> bool, release(key, task_id), is_owner(key, task_id) -> bool.

Reservations

- Implement only UniqueTargetRes. For Harvest the key is Tile(tile).
- Invariants:
    - One owner per key.
    - Release on suspend, cancel, or complete.
- No ItemReservations in 0.d. Add runtime asserts/log if a driver would “consume” beyond world qty.

Systems & Order

- PreStep:
    - designation_spawner: For each (e, Designation::Harvest(tile), TaskRef(None)), create a Task { Harvest{tile}, Pending, priority=3 }, set TaskRef(Some(task_id)). If TaskRef(Some(id)) but task
missing, recreate and set.
    - needs_daemon_emit: For each pawn with Needs below low thresholds, ensure exactly one needs task exists by scanning the TaskBoard (no duplicates). Suggested priorities: EatAuto=9, SleepAuto=8.
    - task_prune_minimal: Remove tasks with Done or Cancelled. If a designation entity is despawned, its dangling task (if any) will be removed when seen as Done/Cancelled (0.e will finalize).
- Step:
    - hard_need_interrupts:
    - If pawn has `Job` and driver is non-needs (Harvest) and `Needs` under threshold, preempt:
      - Set the job’s `Task` status to `Pending`.
      - Release reservation if held.
      - Remove `Job` from pawn.
      - Ensure a needs task exists (scan board; create if absent).
      - Log preemption.
- scheduler_assign:
    - Iterate pawns in ascending `PawnId`.
    - For each idle pawn (no `Job`), scan `TaskBoard.iter_sorted()` and consider tasks with `status == Pending`.
    - Score tuple (higher wins): `(priority, -distance, -task_id)`; distance is Manhattan from pawn pos to task target; for Eat/Sleep use `0` to force needs tasks to win by priority.
    - Pick best; set task `Running`; attach `Job` with appropriate driver in initial state; log assignment with the score tuple.
- job_tick:
    - For needs drivers: decrement `ticks_left`; on complete, bump `Needs` (e.g., `Eat+0.5`, `Sleep+0.5`, clamp to 1.0), mark task `Done`, remove `Job`.
    - For Harvest driver:
      - `ReserveTarget`: attempt `UniqueTargetRes::try_reserve(Tile(tile), task_id)`. On success → `TravelCountdown`; on fail → remain, will try again next tick (checkpoint at entry).
      - `TravelCountdown`: initialize ticks to simple `max(1, manhattan_distance)`. Decrement each tick until 0; checkpoint before `Work`.
      - `WorkCountdown`: fixed small number (e.g., 5). Decrement each tick; on 0 → `Complete`:
        - Mark task `Done`; release reservation; despawn the `Designation` entity with `TaskRef(Some(task_id))` (look up by query); remove `Job`.
- PostStep:
    - release_stale_reservations: scan UniqueTargetRes.owners; if TaskId no longer exists or task is not Running, release. Defensive cleanup only.
    - print_tick_logs: collect assignment and preemption events this tick, sort deterministically, then print lines with stable prefixes:
    - `ASSIGN tick=<t> pawn=<pid> task=<tid> kind=<kind> prio=<p> dist=<d> score=(p,-d,-tid)`
    - `PREEMPT tick=<t> pawn=<pid> from_task=<tid>/<kind> reason=<Eat|Sleep> released=<key>`

Needs & Thresholds

- Thresholds:
    - EAT_LOW = 0.30, SLEEP_LOW = 0.30 in 0.d.
- Emission:
    - Only create needs tasks when below LOW; removal handled by task_prune_minimal when Done.
- Optional decay (for tests/preemption feel):
    - Integrate a tiny linear decay inside needs_daemon_emit (e.g., hunger -= 0.002, rest -= 0.001 clamped). Small, deterministic constant; easy to turn off later.

Determinism

- Pawn iteration: sort by PawnId.
- Task scanning: sort by TaskId.
- Tie-breakers: (priority, -distance, -task_id).
- Logs: buffered per tick, sorted by (pawn_id, task_id) before print.
- TaskId allocation: monotonic via IdAllocator<TaskId> resource.

Tests

- Determinism (500 ticks):
    - Scenario: 2 pawns, 1 harvest designation, seeds fixed.
    - Run twice; collect stdout lines starting with ASSIGN (and PREEMPT if decay active) → identical.
- Unique target exclusion:
    - 2 pawns, same single harvest tile; only one acquires reservation and runs.
    - After N ticks, mark task Cancelled (test harness writes into board); verify its reservation released; the other pawn acquires in subsequent assignment.
- Hard interrupt correctness:
    - Long Harvest (distance large + work countdown). Hunger decays to EAT_LOW; EatAuto is emitted.
    - Assert preemption log; reservation released; EatAuto completes; same or other pawn eventually re-acquires harvest and completes.
- Idempotent spawner:
    - Run PreStep twice in isolation with one Designation::Harvest(tile). Ensure exactly one Task exists and TaskRef set; no duplicates across ticks.

Implementation Steps

- Define types and resources:
    - Add TaskId, TaskKind, TaskStatus, Task, TaskBoard, UniqueTargetKey, UniqueTargetRes, Job, JobDriver, HarvestState, Designation, TaskRef, and Needs.
- Scenario loader:
    - Attach Needs from PawnDef; spawn Designation::Harvest entities with TaskRef(None) from DesignationDef::Harvest.
- Systems:
    - Implement designation_spawner, needs_daemon_emit (+ optional decay), task_prune_minimal.
    - Implement hard_need_interrupts with reservation release and needs task creation.
    - Implement scheduler_assign with scoring tuple and deterministic iteration.
    - Implement job_tick for Harvest and needs drivers, including reservation acquire/release and designation despawn on completion.
    - Implement release_stale_reservations and print_tick_logs.
- Wiring:
    - Register resources (TaskBoard, UniqueTargetRes, IdAllocator<TaskId>).
    - Replace stubs in stitchlands/src/lib.rs with the concrete systems in the specified order.
- Tests:
    - Add integration tests as above, using headless mode and parsing the ASSIGN/PREEMPT lines.
    - Add small unit tests for UniqueTargetRes.

Constants (initial)

- PRIO_HARVEST = 3, PRIO_EAT = 9, PRIO_SLEEP = 8.
- Travel ticks = Manhattan distance; Work ticks = 5.
- Hunger decay per tick = 0.002 (optional); Sleep decay per tick = 0.001 (optional).
- Needs gains: Eat +0.5, Sleep +0.5 (clamp to 1.0).

Notes & Simplifications

- No ItemReservations, no stored urgency, no work-type weights, no setup penalties.
- No soft preemption; only hard interrupts for needs in 0.d.
- Only Harvest designations supported.
- Maintain simplicity in TaskBoard (plain Vec<Task>); future indexing can be layered later.

0.e Preview (next)

- Add centralized assignable predicates per TaskKind.
- Add scoring: workTypeWeight + taskPriority + typeUrgency(hunger, rest, materialsReady) - α*distance - β*setupPenalty.
- Add checkpoint-only soft preemption with hysteresis γ=2.0.
- Finalize idempotence (keep TaskRef and ensure it’s maintained on create/remove).
- Introduce ItemReservations behind a feature flag for the first item-competitive driver.
- Add telemetry: backlog by kind, avg wait time, idle ratio; print periodically in headless.


