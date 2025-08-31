## Problem Solved:

This system addresses the challenge of managing autonomous pawns in an open world with complex, high-level intents such as building, resource gathering, and survival. The primary goal is to ensure correctness and debuggability over cleverness, avoiding complex learned planners or heavy indexing. The system aims to provide a predictable mechanism for agents to execute tasks step-by-step, while respecting fundamental needs like eating and sleeping, preventing resource conflicts, and maintaining a degree of determinism for replays.

## High-Level Solution:

The solution hinges on a single, authoritative list of tasks that is processed by a greedy, single-pass scheduler on each game tick.

1.  **Needs Daemon:** A dedicated component monitors pawn needs (hunger, rest) and generates "Eat" and "Sleep" tasks when predefined thresholds are met.
2.  **Pawn Assignment:** For each available pawn, the system scans the task list, filters for assignable tasks, scores them using a simple heuristic, and assigns the highest-scoring task.
3.  **Job Structure:** Jobs are implemented as small state machines with explicit checkpoints. Soft preemption is allowed only at these checkpoints, with hysteresis to prevent thrashing. However, critical needs can interrupt a pawn's current job at any point.
4.  **Resource Reservations:** Minimal reservations are used. Pawns reserve quantities of items and unique sites (like building blueprints). These reservations are released upon job suspension or failure.

This approach results in straightforward code, transparent decision-making, and preserves the characteristic feel of games like RimWorld.

## Detailed Description:

### Core Data Model:

- **Task:** Contains details like type, arguments, work type, priority, status, and an optional owner pawn for needs-based tasks. Tasks are stored in a single list.
- **PawnProfile:** Stores pawn-specific information such as skills or work type weights, current needs (hunger, rest), location, and their current job.
- **World State:** Includes elements like stockpiles (with item counts and reservations), blueprints, and beds.
- **ReservationMap:** Manages reservations for items within stockpiles and reservations for unique target sites (e.g., a specific blueprint).
- **Job:** A small, task-specific driver with a few states and a flag indicating if the pawn is at a checkpoint.

### Tick Pipeline:

1.  **Needs Decay:** Pawn needs like hunger and rest gradually decrease over time.
2.  **Needs Daemon:** If a pawn's needs fall below critical thresholds, "Eat" or "Sleep" tasks are created with dynamically adjusted priorities.
3.  **Hard Interrupts:** If a pawn experiences a critical need while engaged in another task, their current job is suspended, and its reservations are freed.
4.  **Scheduling:** For each idle pawn or pawn at a checkpoint:
    - **Candidate Generation:** Tasks with "Pending" or "Suspended" status that are unassigned and can be performed by the pawn (considering world state) are identified.
    - **Scoring:** A score is calculated based on work type weight, task priority, type urgency, distance to the task, and a setup penalty.
    - **Job Assignment:** Pawns will only switch tasks at checkpoints if the new candidate offers a significant improvement in priority or urgency. Upon assignment, the task status is updated to "Running," the pawn is marked as assigned, and its job driver is spawned.
5.  **Job Ticking:** Each pawn progresses its assigned job by one step. Jobs involve pre-checks, movement, resource acquisition, or work execution. Upon completion or failure, task status is updated, and pawn ownership is cleared.

### Assignability and Urgency:

- **Assignable:** Checks basic requirements for starting a task, such as matching pawn ownership for needs tasks, availability of building materials, or free beds for sleeping.
- **Type Urgency:** A minor factor that influences task selection. For example, "Eat" task urgency increases with lower hunger levels, and "Sleep" urgency increases with lower rest levels. Building tasks receive a small urgency boost when all necessary materials are present.

### Scoring and Preemption:

The scoring system incorporates:

- **WorkType Weight:** Pawn-specific preferences for certain types of work.
- **Task Priority Label:** An explicit priority assigned to the task.
- **Type Urgency:** The dynamic urgency of the task.
- **Distance Penalty:** A deduction based on the pawn's proximity to the task.
- **Setup Penalty:** A small deduction for tasks that typically require an initial hauling phase.

**Soft preemption** is limited to checkpoints, with a hysteresis margin to prevent frequent task switching for minor gains. **Hard interrupts** for critical needs can override this.

### Reservations:

- **Items:** Small quantities of items are reserved upon task assignment, converted to inventory upon pickup, and released if the job is suspended or fails. Tasks like "Eat" use unreserved items to avoid conflict with building reservations.
- **Unique Targets:** A specific site or blueprint is reserved by only one builder at a time. This reservation is released upon job suspension, cancellation, or completion.
- **Stale Reservations:** Reservations are expired if a job is suspended or a task is canceled.

### Job Drivers:

Examples of job sequences:

- **Build:** Reserve Target and Items -> Move to Stockpile -> Pickup -> Move to Site -> Deliver -> Build. Checkpoints exist before and after delivery. Minimal retries occur if resources change.
- **Eat:** Move to Stockpile -> Eat. Success increases hunger and completes the task. Failure due to lack of food suspends the task.
- **Sleep:** Move to Bed -> Sleep. The bed is reserved while sleeping and released upon waking.

### Correctness and Determinism:

- **Stable Iteration:** Consistent ordering of pawns and tasks during processing ensures deterministic assignments. Ties are broken by distance, then by ID.
- **Invariants:**
  - Reserved items + available items never exceed the actual item count.
  - Unique targets are owned by only one entity at a time.
  - Reservations are always released upon job suspension, cancellation, or completion.
  - Mid-step preemption only occurs for critical needs.
- **Performance:** The greedy matching algorithm has a time complexity of O(P \* T) per scheduling pass, which is considered acceptable for an initial implementation and maintains transparency.

### Failure Handling:

- **Temporary Failures:** Tasks are moved to "Suspended" status and will naturally retry when their preconditions are met again.
- **Permanent Failures:** Tasks are marked as "Failed" with a reason string that can be displayed to the player or agent.
- **Cancellations:** Cancellations respect an "interruptible" flag, stopping the task at the next checkpoint or after the current step.

### Observability:

- **Assignment Logging:** Every pawn assignment logs the winning score and the key factors influencing the decision.
- **Preemption Logging:** Logs detail the source, destination, reason for preemption, and changes in urgency or priority.
- **Debug Overlay:** A simple visual overlay highlights current reservations and targets, making conflicts easily visible.
- **Deterministic Seeds:** Support for seeds allows for bug reproduction through replay.

### Agent API Mapping:

- **`addTask(type, args)`:** Creates a new Task with default "Normal" priority and interruptible status set to true.
- **`removeTask(id)`:** Marks a task as "Cancelled," and it will stop at the next checkpoint or after the current step, depending on its interruptible flag.
- **`setPawnPriorities(map)`:** Updates pawn work type weights, influencing the scheduler without altering currently assigned jobs.

### Extensibility Path:

- **Adding Task Types:** New task types can be added by implementing `assignable`, `typeUrgency`, and a small `Job` driver, with no changes required for the core scheduler.
- **Future Upgrades:**
  - Implementing a `WorkGiver` layer and event-driven indices for larger task lists.
  - Introducing opportunistic hauling as a background task to optimize logistics.
  - Supporting multi-pawn jobs that atomically reserve all involved participants.
  - Incorporating "danger" and "deadline" metrics into the scoring.

### Why This Architecture:

This design represents the leanest architecture capable of delivering a satisfying colony simulation experience. It ensures that needs are prioritized, jobs have a degree of persistence, resources are managed without conflicts, and decision-making is transparent. This approach allows for a shippable product that can be played and tested, guiding future complexity additions based on actual gameplay needs.
