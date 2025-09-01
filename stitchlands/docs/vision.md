# Game Design Document: Deterministic, Phased Development

This document outlines a game design focused on short, playable phases and deterministic test cases for every feature.

## Game Mechanics and Phased Dev Plan

### Pillars

- **Macro Control:** Players set group-level work priorities and queue tasks, not per-pawn micro.
- **Autonomous Scheduler:** Assigns jobs using priorities, skills, distance, urgency, and carried items.
- **Many Neighboring Groups:** Territories update from population and dwelling center-of-mass; early interaction is barter.
- **Deterministic Tests:** Every feature ships with small seeded scenarios that verify behavior without a full playthrough.

### Player Role and Control Model

You manage priorities and queue edits. The simulation exposes the same interface as RimWorld for designations, bills, zones, blueprints, and policies. Per-pawn manual control is omitted initially. The game applies diff-only changes to the priority grid and task list to avoid thrash.

### Pawns, Needs, and Productivity

Needs tracked at start: hunger, rest, mood, health. Acute hunger can kill quickly. Low-grade malnutrition reduces health over time. A pawn’s productivity is a multiplier from needs state, skill for the task, and relevant equipment. Needs spawn auto-tasks with urgency scaling, so survival preempts low-impact work.

### Skills and Work Types

Pawns have skills and per-work-type priorities. Safety-critical work types like firefighting, patient, and bed rest are pinned to minimum safe values for everyone.

### Tasks, Edits, and Scheduler

Players issue concrete edits: place blueprint, create growing zone, set bill until X, designate harvest or haul urgently, clean room, expand stockpile with filters. A scheduler assigns work by combining per-pawn priority, skill fit, distance and path cost, currently carried items, and task urgency. Each decision cycle has a small edit budget to keep behavior stable.

### Tools, Benches, Buildings

Most tasks can be done bare-handed at poor throughput. Task-specific tools and benches provide tiered boosts or unlock recipes. Equipment degrades and is sustained by a passive maintenance bill that is present by default but can be tuned or disabled.

Buildings are prefab blueprints with bench slots to reduce micro and aid AI. Some early examples:

- **Small Hut:** Sleeping only, small storage, 0 bench slots.
- **Cabin:** Family sleeping, 1 to 2 bench slots, some storage.
- **Workshop:** Several bench slots, no sleeping, medium storage.
- **Storeroom:** High storage density, 1 to 2 bench slots for logistics or maintenance.

Players place exact tiles for zones and building footprints. Benches consume slots by size or complexity. Room bonuses for light and cleanliness are small at first and grow later.

### Supply Chains and Production

Chains start shallow but already show leverage from tools. Example: grain can be ground by hand, faster with a mortar and pestle, and much faster with a mill or windmill. Similar patterns apply to cooking throughput, basic medicine processing, and early construction.

### Territory and Trade

Each group controls a territory whose center updates periodically. Early trade is simple barter at territory borders with a short transfer delay. World generation can bias resource distributions, but this is a config toggle rather than a core mechanic.

### Progression and Failure

Progression means sustaining larger populations at higher need satisfaction by improving skills, tools, benches, buildings, and land improvements. Failure is the group dying. Cooperation with neighbors accelerates growth but is optional.

### Cadence and Session Feel

The world runs continuously with event-driven decisions and a periodic heartbeat. Early phases are meant to be quick—a few real-world minutes—for proving systems. There is no hard time limit. As content expands, natural playtime increases.

### Telemetry and Success Metrics

From Phase 1 onward, the game surfaces:

- Food buffer days and spoilage horizon.
- Average job wait time and average haul distance.
- Idle time and task backlog by type.
- Median mood and tails.
- Repair backlog and equipment condition.
- Power margin and freezer temperature once introduced.

These metrics drive both player feedback and automated tests.

### Deterministic Test Harness

Every feature ships with small, reproducible scenarios:

- Fixed RNG seed and map seed, declarative start state, and scripted time window.
- Oracle checks on telemetry and world state at checkpoints.
- Binary pass or fail with crisp reasons.
- Example checks: no starvation events for 2 in-game days, median mood stays above 60 percent, repair backlog under threshold, barter delivery occurs within transfer window.

---

## Phased Roadmap - Each Phase is Playable and Testable

### Phase 0 - Crumbs and Naps

- **Content:** Scattered food items, sleeping spots, hunger and rest only, no buildings yet, no bills, no priorities UI.
- **Player Loop:** Pawns gather nearby food, eat when hungry, sleep when tired. You can place sleeping spots and a simple stockpile.
- **Tests:**
  - Survival smoke test - survive 2 in-game days with 0 deaths under seed S.
  - Pathing sanity - average travel per job below D tiles.
- **Exit Criteria:** Both tests pass in Github Actions CI within 3 minutes wall time.

### Phase 1 - Home and Piles

- **Content:** Stockpile zones, basic hauling, clean-room-lite flag, exact tile placement for zones, tiny bottlenecks panel.
- **Player Loop:** Organize space so trips shorten and sleeping is nearby. Cleaning raises a small mood bonus.
- **Tests:**
  - Haul consolidation - 80 percent of food ends in stockpile within T game hours.
  - Travel reduction - average travel per job drops at least X percent versus Phase 0 scenario.
- **Exit Criteria:** Both tests pass, loop still under 5 minutes.

### Phase 2 - First Production

- **Content:** One shallow chain - harvest grain, grind, cook simple meals. Campfire and mortar-and-pestle improve throughput. Bills support only until X.
- **Player Loop:** Stabilize meals, place workshop corner next to storeroom, watch throughput improve.
- **Tests:**
  - Buffer target - maintain 2.5 days of meals for Y hours under seed S.
  - Tool leverage - meals per hour with tools is at least R times the bare-handed baseline.
- **Exit Criteria:** Passes in under 6 minutes, clear visual delta when tools are built.

### Phase 3 - Roles That Matter

- **Content:** Per-work-type priorities UI, skills active, scheduler uses priority + skill + distance + carried items, safety work pinned.
- **Player Loop:** Assign roles and feel a tangible step up in throughput and wait times.
- **Tests:**
  - Priority effect - toggling one pawn’s Cooking priority from 4 to 1 raises meals per hour by at least Q percent in a controlled setup.
  - Scheduler sanity - job wait time stays below W under seeded workload.
- **Exit Criteria:** Both pass quickly, UI supports diff-only edits.

### Phase 4 - Buildings With Slots

- **Content:** Small hut, cabin, workshop, storeroom. Benches consume slots. Light and cleanliness give small bonuses in kitchens and clinics.
- **Player Loop:** Layout starts to matter. Storeroom adjacent to workshop reduces travel and boosts throughput.
- **Tests:**
  - Layout effect - canonical layout beats scattered layout by at least Z percent on meals per hour.
  - Slot enforcement - benches refuse placement when slots exhausted, and accept when slots open.
- **Exit Criteria:** Passes in under 8 minutes, layout benefits are obvious.

### Phase 5 - Tools, Durability, Passive Repair

- **Content:** Durability for tools and benches, passive repair bill with threshold slider, simple material costs.
- **Player Loop:** Keep gear humming with minimal oversight, trade time and materials against breakdowns.
- **Tests:**
  - Maintenance stability - repair backlog remains under B for H game hours with default bill.
  - Throughput resilience - meals per hour drops less than K percent over time with repair on, but degrades quickly if repair is off.
- **Exit Criteria:** Both pass, maintenance creates a meaningful but light planning lever.

### Phase 6 - Two Groups and Border Barter

- **Content:** A neighboring group with a different local surplus, territories that update periodically, barter at borders with short transfer delay.
- **Player Loop:** Fix local shortfalls by swapping surpluses for deficits without caravans yet.
- **Tests:**
  - Delivery guarantee - barter transfer completes within D seconds of agreement under seed S.
  - Post-trade effect - selected deficit metric crosses target within E minutes after delivery.
- **Exit Criteria:** Passes, trade UI is minimal and deterministic.

### Phase 7 - Mood and Health Depth

- **Content:** Mood causes bucketed, malnutrition as a health condition that reduces productivity before lethal stages, cleanliness affects kitchens and clinics more strongly.
- **Player Loop:** Juggle quick wins like light, tables, and cleaning to keep median mood above 60 percent while expanding food.
- **Tests:**
  - Mood control - median mood stays above 60 percent for 3 days under scripted chores.
  - Malnutrition curve - productivity declines smoothly before deaths in a starvation stress test.
- **Exit Criteria:** Passes, no brittle spikes.

### Phase 8 - Branching Chains and Light Logistics

- **Content:** Second crop, simple power source, storage priorities, spoilage, freezer room mechanics, power margin readout.
- **Player Loop:** Place cold storage near kitchen, watch spoilage drop, keep power positive.
- **Tests:**
  - Spoilage reduction - adding freezer reduces spoilage ratio below S percent.
  - Power margin - benches shed load correctly when power deficit occurs.
- **Exit Criteria:** Passes, logistics is fun not fussy.

### Phase 9 - Stressors and Scenarios

- **Content:** Gentle raids, mild disease, weather swings. No per-pawn combat micro needed.
- **Player Loop:** Validate resilience of priorities, layout, and buffers under pressure.
- **Tests:**
  - Scenario gauntlet - pass 3 scripted stress checks without deaths.
  - Recovery window - food buffer and mood recover to targets within T minutes after each stressor.
- **Exit Criteria:** Passes, feels like a living colony cluster.

---

## Build and Testing Workflow

- Each phase ships with a minimal scenario set that runs headless in CI with fixed seeds and time limits.
- A small interactive sandbox exists for manual play, but tests do not depend on human input.
- Telemetry is exported to structured logs so oracles can check deltas and thresholds exactly.
- The planner LLM can be integrated at Phase 3 or later without changing interfaces.

## Scope Controls That Keep It Shippable

- Hard edit budget per decision from the start.
- Tiny recipe lists until Phase 8.
- Exact tile placement for player intent, with blueprint slot abstractions to keep placement simple.
- World resource bias is a config toggle, used mainly in Phase 6 to force interesting trades.
