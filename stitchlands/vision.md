Here is a cohesive, self-contained design doc that bakes in your goal of short, minimally playable phases and deterministic test cases for every feature.

Game mechanics and phased dev plan

Pillars
	•	Macro control - players set group-level work priorities and queue tasks, not per-pawn micro.
	•	Autonomous scheduler - assigns jobs using priorities, skills, distance, urgency, and carried items.
	•	Many neighboring groups - territories update from population and dwelling center-of-mass, early interaction is barter.
	•	Deterministic tests - every feature ships with small seeded scenarios that verify behavior without a full playthrough.

Player role and control model

You manage priorities and queue edits. The sim exposes the same interface as RimWorld for designations, bills, zones, blueprints, and policies. Per-pawn manual control is omitted initially. The game applies diff-only changes to the priority grid and task list to avoid thrash.

Pawns, needs, and productivity

Needs tracked at start: hunger, rest, mood, health. Acute hunger can kill quickly. Low-grade malnutrition reduces health over time. A pawn’s productivity is a multiplier from needs state, skill for the task, and relevant equipment. Needs spawn auto-tasks with urgency scaling so survival preempts low-impact work.

Skills and work types

Pawns have skills and per work-type priorities. Safety-critical work types like firefighting, patient, and bed rest are pinned to minimum safe values for everyone.

Tasks, edits, and scheduler

Players issue concrete edits: place blueprint, create growing zone, set bill until X, designate harvest or haul urgently, clean room, expand stockpile with filters. A scheduler assigns work by combining per-pawn priority, skill fit, distance and path cost, currently carried items, and task urgency. Each decision cycle has a small edit budget to keep behavior stable.

Tools, benches, buildings

Most tasks can be done bare-handed at poor throughput. Task-specific tools and benches provide tiered boosts or unlock recipes. Equipment degrades and is sustained by a passive maintenance bill that is present by default but can be tuned or disabled.

Buildings are prefab blueprints with bench slots to reduce micro and aid AI:
	•	Small hut - sleeping only, small storage, 0 bench slots.
	•	Cabin - family sleeping, 1 to 2 bench slots, some storage.
	•	Workshop - several bench slots, no sleeping, medium storage.
	•	Storeroom - high storage density, 1 to 2 bench slots for logistics or maintenance.

Players place exact tiles for zones and building footprints. Benches consume slots by size or complexity. Room bonuses for light and cleanliness are small at first and grow later.

Supply chains and production

Chains start shallow but already show leverage from tools. Example: grain can be ground by hand, faster with a mortar and pestle, and much faster with a mill or windmill. Similar patterns apply to cooking throughput, basic medicine processing, and early construction.

Territory and trade

Each group controls a territory whose center updates periodically. Early trade is simple barter at territory borders with a short transfer delay. World generation can bias resource distributions, but this is a config toggle rather than a core mechanic.

Progression and failure

Progression means sustaining larger populations at higher need satisfaction by improving skills, tools, benches, buildings, and land improvements. Failure is the group dying. Cooperation with neighbors accelerates growth but is optional.

Cadence and session feel

The world runs continuously with event-driven decisions and a periodic heartbeat. Early phases are meant to be quick - a few real-world minutes - for proving systems. There is no hard time limit. As content expands, natural playtime increases.

Telemetry and success metrics

From Phase 1 onward the game surfaces:
	•	Food buffer days and spoilage horizon.
	•	Average job wait time and average haul distance.
	•	Idle time and task backlog by type.
	•	Median mood and tails.
	•	Repair backlog and equipment condition.
	•	Power margin and freezer temperature once introduced.

These metrics drive both player feedback and automated tests.

Deterministic test harness

Every feature ships with small, reproducible scenarios.
	•	Fixed RNG seed and map seed, declarative start state, and scripted time window.
	•	Oracle checks on telemetry and world state at checkpoints.
	•	Binary pass or fail with crisp reasons.
	•	Example checks: no starvation events for 2 in-game days, median mood stays above 60 percent, repair backlog under threshold, barter delivery occurs within transfer window.

⸻

Phased roadmap - each phase is playable and testable

Phase 0 - Crumbs and naps
	•	Content: scattered food items, sleeping spots, hunger and rest only, no buildings yet, no bills, no priorities UI.
	•	Player loop: pawns gather nearby food, eat when hungry, sleep when tired. You can place sleeping spots and a simple stockpile.
	•	Tests:
	•	Survival smoke test - survive 2 in-game days with 0 deaths under seed S.
	•	Pathing sanity - average travel per job below D tiles.
	•	Exit criteria: both tests pass on CI within 3 minutes wall time.

Phase 1 - Home and piles
	•	Content: stockpile zones, basic hauling, clean-room-lite flag, exact tile placement for zones, tiny bottlenecks panel.
	•	Player loop: organize space so trips shorten and sleeping is nearby. Cleaning raises a small mood bonus.
	•	Tests:
	•	Haul consolidation - 80 percent of food ends in stockpile within T game hours.
	•	Travel reduction - average travel per job drops at least X percent versus Phase 0 scenario.
	•	Exit criteria: both tests pass, loop still under 5 minutes.

Phase 2 - First production
	•	Content: one shallow chain - harvest grain, grind, cook simple meals. Campfire and mortar-and-pestle improve throughput. Bills support only until X.
	•	Player loop: stabilize meals, place workshop corner next to storeroom, watch throughput improve.
	•	Tests:
	•	Buffer target - maintain 2.5 days of meals for Y hours under seed S.
	•	Tool leverage - meals per hour with tools is at least R times the bare-handed baseline.
	•	Exit criteria: passes in under 6 minutes, clear visual delta when tools are built.

Phase 3 - Roles that matter
	•	Content: per work-type priorities UI, skills active, scheduler uses priority + skill + distance + carried items, safety work pinned.
	•	Player loop: assign roles and feel a tangible step up in throughput and wait times.
	•	Tests:
	•	Priority effect - toggling one pawn’s Cooking priority from 4 to 1 raises meals per hour by at least Q percent in a controlled setup.
	•	Scheduler sanity - job wait time stays below W under seeded workload.
	•	Exit criteria: both pass quickly, UI supports diff-only edits.

Phase 4 - Buildings with slots
	•	Content: small hut, cabin, workshop, storeroom. Benches consume slots. Light and cleanliness give small bonuses in kitchens and clinics.
	•	Player loop: layout starts to matter. Storeroom adjacent to workshop reduces travel and boosts throughput.
	•	Tests:
	•	Layout effect - canonical layout beats scattered layout by at least Z percent on meals per hour.
	•	Slot enforcement - benches refuse placement when slots exhausted, and accept when slots open.
	•	Exit criteria: passes in under 8 minutes, layout benefits are obvious.

Phase 5 - Tools, durability, passive repair
	•	Content: durability for tools and benches, passive repair bill with threshold slider, simple material costs.
	•	Player loop: keep gear humming with minimal oversight, trade time and materials against breakdowns.
	•	Tests:
	•	Maintenance stability - repair backlog remains under B for H game hours with default bill.
	•	Throughput resilience - meals per hour drops less than K percent over time with repair on, but degrades quickly if repair is off.
	•	Exit criteria: both pass, maintenance creates a meaningful but light planning lever.

Phase 6 - Two groups and border barter
	•	Content: a neighboring group with a different local surplus, territories that update periodically, barter at borders with short transfer delay.
	•	Player loop: fix local shortfalls by swapping surpluses for deficits without caravans yet.
	•	Tests:
	•	Delivery guarantee - barter transfer completes within D seconds of agreement under seed S.
	•	Post-trade effect - selected deficit metric crosses target within E minutes after delivery.
	•	Exit criteria: passes, trade UI is minimal and deterministic.

Phase 7 - Mood and health depth
	•	Content: mood causes bucketed, malnutrition as a health condition that reduces productivity before lethal stages, cleanliness affects kitchens and clinics more strongly.
	•	Player loop: juggle quick wins like light, tables, and cleaning to keep median mood above 60 percent while expanding food.
	•	Tests:
	•	Mood control - median mood stays above 60 percent for 3 days under scripted chores.
	•	Malnutrition curve - productivity declines smoothly before deaths in a starvation stress test.
	•	Exit criteria: passes, no brittle spikes.

Phase 8 - Branching chains and light logistics
	•	Content: second crop, simple power source, storage priorities, spoilage, freezer room mechanics, power margin readout.
	•	Player loop: place cold storage near kitchen, watch spoilage drop, keep power positive.
	•	Tests:
	•	Spoilage reduction - adding freezer reduces spoilage ratio below S percent.
	•	Power margin - benches shed load correctly when power deficit occurs.
	•	Exit criteria: passes, logistics is fun not fussy.

Phase 9 - Stressors and scenarios
	•	Content: gentle raids, mild disease, weather swings. No per-pawn combat micro needed.
	•	Player loop: validate resilience of priorities, layout, and buffers under pressure.
	•	Tests:
	•	Scenario gauntlet - pass 3 scripted stress checks without deaths.
	•	Recovery window - food buffer and mood recover to targets within T minutes after each stressor.
	•	Exit criteria: passes, feels like a living colony cluster.

⸻

Build and testing workflow
	•	Each phase ships with a minimal scenario set that runs headless in CI with fixed seeds and time limits.
	•	A small interactive sandbox exists for manual play, but tests do not depend on human input.
	•	Telemetry is exported to structured logs so oracles can check deltas and thresholds exactly.
	•	The planner LLM can be integrated at Phase 3 or later without changing interfaces.

Scope controls that keep it shippable
	•	Hard edit budget per decision from the start.
	•	Tiny recipe lists until Phase 8.
	•	Exact tile placement for player intent, with blueprint slot abstractions to keep placement simple.
	•	World resource bias is a config toggle, used mainly in Phase 6 to force interesting trades.