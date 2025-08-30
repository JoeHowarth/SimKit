What problem this solves

You have autonomous pawns in an open world that should pursue high level intents like build this, haul that, keep themselves alive, and not thrash between jobs. You want correctness and debuggability first, not cleverness. No learned planners, no heavy indices. Just a predictable system that converts agent intents into concrete, stepwise work while respecting needs like eating and sleeping, avoiding resource conflicts, and staying deterministic enough to replay.

How we solve it at a high level

We keep one authoritative list of tasks and run a greedy, single pass scheduler each tick:
	1.	A needs daemon emits Eat and Sleep tasks when thresholds trip.
	2.	For each ready pawn, we scan the task list, filter what is currently assignable, score candidates with a tiny heuristic, and assign the best.
	3.	Jobs are tiny state machines with explicit checkpoints. Soft preemption only at checkpoints with hysteresis. Hard needs can interrupt anywhere.
	4.	Reservations are minimal: count reservations for items and a simple target reservation for unique sites. Release on suspend or fail.

This yields simple code paths and transparent decisions while retaining the RimWorld feel.

More detailed description

Core data model
	•	Task: type, args, workType, priority label, status, optional owner pawn for needs. Lives in a single Vec.
	•	PawnProfile: skills or just workType weights, needs (hunger, rest), position, current job.
	•	World bits: stockpile with item counts and reserved counts, blueprints, bed.
	•	ReservationMap: implicit inside stockpile reserved counts plus a map for unique targets like blueprint B1.
	•	Job: a small driver per task type with a handful of states and a boolean at_checkpoint.

Tick pipeline
	1.	Needs decay - hunger and rest drift down each tick.
	2.	Needs daemon - if thresholds fall below soft or hard limits, enqueue Eat or Sleep for that pawn with dynamic priority.
	3.	Hard interrupts - if a pawn hits a hard threshold while not already on a needs job, suspend its current job and free reservations.
	4.	Scheduling - for each pawn that is idle or at a checkpoint:
	•	Build candidates by scanning tasks with status Pending or Suspended, unassigned, and assignable(pawn, world).
	•	Score = workTypeWeight(pawn) * priorityLabel + typeUrgency(pawn, task) - α * distance - β * setupPenalty.
	•	Respect stickiness by only switching at checkpoints when the new candidate clearly wins either in priority or urgency by a margin.
	•	On assign, flip task to Running, mark assigned_to, and spawn the job driver.
	5.	Job ticking - each pawn advances its job one step. Jobs perform prechecks, update position, convert reservations to inventory, deliver materials, or work until done. On completion or failure, update task status and clear ownership.

Assignable and urgency
	•	assignable checks the minimal gates to start step 1:
	•	Needs tasks: owner matches pawn and preconditions hold.
	•	Build: blueprint not built, and there is at least some required item available or already delivered.
	•	Sleep: bed is free or already reserved by this pawn.
	•	typeUrgency is intentionally small and explainable:
	•	Eat scales with 1 - hunger, Sleep scales with 1 - rest.
	•	Build bumps when all materials are onsite to nudge building over additional hauling.

Scoring and preemption
	•	Scoring terms you start with:
	•	workType weight from setPawnPriorities
	•	task priority label from the agent
	•	type urgency
	•	distance penalty to the first step
	•	small setup penalty for tasks that usually need a haul cycle
	•	Soft preemption only at checkpoints, with a hysteresis margin so pawns do not thrash for small gains. Hard interrupts bypass this.

Reservations
	•	Items: reserve small quantities at assignment, convert reservations to inventory at pickup, and release on suspend or failure. Eat uses unreserved take so it never steals from build reservations.
	•	Unique targets: reserve a blueprint id so only one builder works that spot. Release on suspend, cancel, or completion.
	•	Expire stale reservations when a job is suspended or a task is cancelled.

Job drivers
	•	Build: ReserveTargetAndItems - MoveToStockpile - Pickup - MoveToSite - Deliver - Build. Checkpoints before Deliver and after Deliver. Minimal retries if stock changes underfoot.
	•	Eat: MoveToStockpile - Eat. On success, increase hunger, complete. On failure due to no food, suspend.
	•	Sleep: MoveToBed - Sleep. Reserve bed while sleeping, release on wake.

Correctness and determinism
	•	Stable iteration order for pawns and tasks provides deterministic assignment. Ties break by distance then id.
	•	Invariants:
	•	reserved + available never exceeds actual item count
	•	only one owner of a unique target at a time
	•	reservations are released on suspend, cancel, or complete
	•	preemption never happens mid step unless it is a hard need
	•	The greedy matcher is O(P * T) per scheduling pass, which is acceptable for an MVP and keeps reasoning obvious.

Failure handling
	•	Temporary failures move a task to Suspended with a natural retry when preconditions are met again.
	•	Permanent failures mark Failed with a reason string visible to the agent or UI.
	•	Cancellations respect interruptible flags: stop at next checkpoint or after the current step.

Observability
	•	Every assignment logs the winning score and key terms that mattered.
	•	Preemption logs include from, to, reason, and delta urgency or priority.
	•	A simple overlay for reservations and current targets makes conflicts visible.
	•	Deterministic seeds support replay for bugs.

Agent API mapping
	•	addTask(type, args) creates a Task with default Normal priority and interruptible true.
	•	removeTask(id) marks Cancelled and either stops at next checkpoint or after the current step.
	•	setPawnPriorities(map) updates workType weights and immediately biases the scheduler without touching existing jobs.

Extensibility path
	•	Add task types by implementing assignable, typeUrgency, and a small Job driver. No change to the scheduler.
	•	Optional later upgrades:
	•	A WorkGiver layer and event driven indices if T grows.
	•	Opportunistic hauling as a bounded micro step to amortize logistics.
	•	Multi pawn jobs that atomically reserve all participants.
	•	Danger and deadline terms in scoring.

Why this shape

It is the smallest architecture that still feels like a colony sim: needs interrupt, jobs are sticky, resources are not double booked, and decisions are explainable. You can ship it, play it, and then decide where added complexity actually pays for itself.
