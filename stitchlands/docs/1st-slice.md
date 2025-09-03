# Tasks

This is a brain dump. Don't respond in detail except for one paragraph summarizing what I've said. Okay, so we can simplify the job scoring system to first look at work item priority and compare that against needs. We can then bucket all of the potential tasks by their work type or their task type. And the pawn only needs to consider the highest priority non-empty bucket. From there, we get to choose which specific task or need in that bucket we actually perform. For needs, there's only ever one, so that's simple. For tasks, we'll need to get a little bit more granular, probably creating a plan and estimating how difficult it would be to complete that plan. Or perhaps a simpler heuristic of what's the minimum amount of work to get to a checkpoint for that task. For constructing a building, this might be hauling one component. Or if we've already gotten all the required building materials, this might be getting to the building and the time it takes to complete one build stage of the total construction. From there, we can select which job, and a job is either a task or a need.
Tasks are provided by the player. Needs are automatic, like hunger and sleep, etc. We also need to account for thrashing. Possibly, what we can do is every task will have checkpoints, like we already talked about, which is one unit of progress, whatever that means for each task type. So we could either only re-evaluate at checkpoints, or possibly we could just give any current job that we're on a bonus, such that it usually makes sense to complete that task, but can always be pre-empted. Maybe the bonus gets reduced significantly at a checkpoint or something. Or maybe once you reach a checkpoint, there's no bonus until you begin at the next thing. That might be a fairly simple way of doing it, so I'm leaning towards that.

Summary:
You propose a simpler scheduler: compare needs vs work-item priority, bucket all candidates by work or task type, then only consider the highest-priority non-empty bucket. Needs are trivial since there is only one; for tasks, choose a specific item via a quick plan or a heuristic like minimum work to the next checkpoint. A job is either a player-provided task or an automatic need. To limit thrashing, give the current job a continuation bonus so it usually finishes, allow preemption when something clearly outranks it, and mainly re-evaluate at checkpoints where the bonus drops or resets. Checkpoints are the atomic progress units per task (e.g., haul one component or complete one build stage).

Next section:
Okay, another round. When we complete the task system, this pretty much gets us to the core of our first vertical slice. We're going to need to update the snapshot system and the scenario loading system to reflect this. And then we should write some tests. Once this is done, we can start working on the rendering layer. For this, we'll use a tile map, represent all the tiles, and simple sprites to represent fixtures and placed items, as well as ponds. We'll need a UI to be able to get more detailed views of ponds, for example, what they're carrying, and their current level of needs, hunger, sleep, etc. We'll also want to have some level of information actually displayed above each thing. Maybe just for ponds, it's just their name or something like that. Possibly some sort of visual indicator, like how tired they are, how hungry they are, or something like that. Eventually we can incorporate that into the sprites themselves, but maybe for now we can just have a little icon that floats near the sprite on the 2D map. We'll also need some sort of a UI to understand the task system.
I think this is going to be pretty important at first, because we'll need to debug this. We'll also want a matrix or a table that shows RimWorld-style priorities, where one axis is pond name, and a second axis is work type. We don't have skill or proficiency yet, but eventually we'll probably want to copy RimWorld's methodology of color coding and shading to indicate how skilled a pond is at each given task. We'll want a similar UI, possibly a pop-up, probably a pop-up, for being able to click on fixtures, buildings, or stockpiles, or fields, or any possibly multi-tile thing that has been placed on the map. This also includes trees, etc. We'll want some sort of visual indicator that there is a task associated with a specific location.
Eventually we'll probably have icons or little sprites per task type to denote this, but we could start with something simpler. I mentioned this earlier, but we also will want some sort of a debug, not even a debug, some sort of way of visualizing all of the outstanding tasks. Once we have all that, that's mostly about visualizing information, we'll also need to be able to create inputs into the game. The primary inputs will be... Let's take a step back. There's going to be three types of inputs. One is read-only, changing the state of the viewer, so panning the map, zooming in and out, clicking on UI windows, all that kind of stuff. That can stay purely client-side. Next are going to be... ...meta commands to the server, so pause, increase speed, decrease speed, save a game, load a game, exit to the menu, exit the application, all that kind of stuff. The third will be actual gameplay interactions.
For now, we're just going to have assign or update pan work type priorities, so the client is going to send to the server for one pan the complete list of its priorities for work type. The other input type will be creating a task, so this would be anything like saying build this type of building in this spot or harvest this field or anything kind of like that. It will also include builds at fixtures, so certain fixtures can be interacted with. So, for example, produce 200 flour at a mill, and this then creates a task, and that particular task has inputs, like grain in this case, and produces outputs. There are sort of auto-tasks as well, like hauling. There also would be builds at stockpiles, like keep X amount of good or item in the stockpile, and then people that get assigned hauling tasks will complete those. For now, we're not going to have any priorities or sort of complex interactions with builds or other tasks. They'll just be sort of flat, and then everything's handled per type. The last gameplay input that I can think of right now is removing a task. We're not going to allow mutation. So, for example...

Summary:
You’re outlining the first vertical slice: finish the task system, update snapshot and scenario loading, add tests, then build a simple rendering layer using a tile map with sprites for tiles, fixtures, items, and pawns. Add UI to inspect pawns (what they carry, hunger and sleep), lightweight overlays for names and basic need icons, and early task-system visibility tools: a RimWorld-style priorities matrix (pawns x work types, skills later with color), pop-up inspectors for fixtures/buildings/stockpiles/fields/trees, location markers for tasks, and a global outstanding-tasks view. Inputs split into three tiers - client-only viewing controls, server meta commands (pause, speed, save/load, exit), and gameplay actions that set full work-type priority maps per pawn, create tasks like build/harvest/produce with inputs-outputs plus auto hauling and stockpile targets, and remove tasks without in-place mutation. This gets you to a playable core you can iterate on visually and interactively.

- **Scheduler** - compare needs vs work-item priority, then bucket all candidates by work type or task type; only consider the highest-priority non-empty bucket.
- **Within the chosen bucket, select a job:**
  - If a need - execute it directly (there is only one).
  - If a task - make a quick plan or use a heuristic that chooses the minimum work to reach the next checkpoint.
  - Examples: for construction, haul one required component if missing; if materials are ready, walk to the site and complete one build stage.
- **Define job** = either a player-provided task or an automatic need (hunger, sleep, etc.).
- **Checkpoints** - specify atomic progress units per task type.
- **Anti-thrashing** - give the current job a continuation bonus so it usually finishes, but allow preemption if something clearly outranks it.
- **Re-evaluation** - primarily at checkpoints; drop or reset the continuation bonus at each checkpoint, reapply when starting the next segment.
- **Finish the task system** aligned with the above rules.
- **Update the snapshot system** to capture scheduler state, current job, checkpoints, and bonuses.
- **Update the scenario loading system** to initialize buckets, priorities, tasks, and needs consistently.
- **Write tests** covering: bucket selection, need vs task choice, checkpoint semantics, continuation bonus behavior, and preemption at checkpoints.
- **Build a simple rendering layer:**
  - Tile map for terrain.
  - Simple sprites for fixtures, placed items, and pawns.
  - Lightweight overlays: pawn names; small icons near sprites for hunger, sleep, etc.
- **UI for inspection and visibility:**
  - Pawn inspector showing what they carry and current needs levels.
  - RimWorld-style priorities matrix - rows pawns, columns work types; plan for later skill-based color coding.
  - Pop-up inspectors for fixtures, buildings, stockpiles, fields, trees, and other multi-tile entities.
  - Visual markers at locations with associated tasks.
  - A global view to visualize all outstanding tasks.
- **Inputs** - three categories:
  - Client-only viewing controls - pan, zoom, click UI windows.
  - Server meta commands - pause, speed up or down, save, load, exit to menu, exit application.
  - Gameplay actions:
    - Set or update a pawn’s full work-type priority map - client sends complete list per pawn.
    - Create tasks - place buildings, harvest fields, operate fixtures (e.g., produce 200 flour at a mill with specified inputs and outputs).
    - Stockpile targets - keep X of an item in a stockpile; auto hauling fulfills these.
    - Remove tasks - allow deletion but no in-place mutation.

Okay, now let's talk about how we can train the model to be able to play from a headless mode. I think we'll start with purely text-based representations. Like we've talked about previously, where we encode each entity on the map as just a list of entity key-value pairs. So, for example, we'd have the pawn ID and then its position, what it's carrying, the current task that it's working on. That would be denormalized as opposed to a reference into a separate record. Additionally, any other attributes that we think are important later on can be serialized in that same way. We'll do that for pawns, fixtures, and items on the ground. Fixtures containing items will work very similar to pawns, where there will be an inventory attribute. And the items will appear there instead of as separate entities. I think we may want to actually duplicate the task information now that I think about it. So each pawn will include a full copy of the task itself, as well as its progress and its job metadata. Then we'll also have a separate section of records that are the tasks themselves. This will be sorted by the type of task and possibly broken into multiple groups per status. So, pending, assigned, completed, and recent, and cancelled just in the last step of the simulation, or since the last observation. We'll also, per pawn, definitely want to include the work priorities. I think we'll just embed that into the same record, though we can experiment with doing that in different ways. We'll also include a simple 2D ASCII art rendering of the map. So that the model can see the spatial dimension of all of the entities that are in the record table. Each table. So they'll both have a Cartesian attribute as numbers in the table, and then also have a 2D spatial representation. A lot of models have been trained with ASCII art and some limited type of spatial reasoning like that, so hopefully providing it in two ways will be helpful. Now that we have a basic idea of how to tokenize our state, I think we should do some supervised learning of asking questions about the state. Like, give a state, and a question about it, provided the answer. So that the model then learns how to reason about the state. First keep it very simple, and then ask more compound things. We can also give the record table and ask it to produce the ASCII representation, or the ASCII art 2D grid, and vice versa. We can also ask it to predict, given the current state of the board, what the next state of the board will be. We can ask it to summarize the state of the board. Basically just get it to work with the state a lot. And also maybe some limited dynamic models of what does the state look like. Give one state, what does the next state look like? Cool. So that's our warm-up there. Once we've done that, I think we'll want to do the main task, which is going to be given an objective that's in the form of text. Typically a few sentences or a paragraph. We're going to try to work towards that goal by, in a loop, getting an observation, which is that player's view of the state, which is then tokenized. And then it can perform a series of actions. Those actions are either going to be adjusting upon work-type priorities, placing new tasks, or canceling tasks. It can do a limited number of these per step, so it can't change things drastically. There might be some other query-style actions or tools that we'll give it access to, to be able to compute things on top of the state. This feels like it should be helpful. Not exactly sure what those will be. But we'll also want to include some basic training, like how to use those tools, when's a good time to use the tools, etc. Getting back to the main task, it'll get an observation, produce an action, do this however many times as part of that training run, or that rollout. And then at the end of the rollout, we will basically score the end state based off of the objective. At first, we'll use very concrete objectives, so we'll actually procedurally generate them. We'll have a very quantified objective that we'll tokenize and convert to text, which is fed into the model. And then on the other side, we can deterministically look at the state and see how well we did against the objective. For each goal plus starting state tuple, we will do multiple rollouts, so that then we can calculate the advantage of each sequence of actions against each other, and try to learn from that. Thank you.

Love this direction. Here is a compact, end-to-end plan to make the headless, text-only loop trainable and robust without overfitting to formatting quirks.

1.  **Observation format** - make it canonical, small, and redundant on purpose

    - Canonicalization rules: single schema version, include tick, rng seed, map size, and a monotonically increasing state_id. Sort all records deterministically: by kind, then y, then x, then id. No hidden defaults.
    - Denormalize smartly: each pawn record embeds its full current task snapshot and progress, plus its work priorities. Keep a global task table too. Tie them with task_id and a short task_digest so you can assert consistency and let the model cross-check.
    - Spatial dual-view: keep numeric coords in every record and include a fixed-size ASCII map. Use strict layering rules so symbols are unambiguous. Consider a cropped local map around each pawn when the global map is large, alongside the global overview every N steps.
    - Deltas: alongside the full snapshot, surface a compact event log since last obs - created, destroyed, moved, task status changes, inventory diffs. This trains temporal reasoning cheaply and helps credit assignment.
    - Budget guards: cap entities serialized per class with a stable priority heuristic (distance to objectives, recency, involvement in tasks). Emit truncation counts so the model knows it is looking at a partial list.

2.  **Action space** - small, typed, and verifiable

    - Primitive actions per step: `set_priority(work_type, weight)`, `add_task(kind, args)`, `cancel_task(task_id)`, optional `query_tool(name, args)`, `noop`, `stop`. Hard cap K actions per step.
    - Arguments are ids and enums, never free text. Keep a tiny closed vocabulary for kinds and work types to stabilize tokens.
    - Validator in the loop: engine evaluates each proposed action and returns ok or error with a terse reason. Feed those results back in the next observation so the policy learns feasibility.
    - Tool calls: begin with a few deterministic queries that are hard for LMs and cheap for the engine - path length between A and B, nearest X to Y with constraint, connected-components of tillable fields, projected haul time for a plan. The observation should contain recent tool results and their inputs.

3.  **Curriculum and pretraining tasks** - build the model’s “state literacy”

    Start with pure supervised objectives on generated data before any control:

    - State QA: count, filter, nearest, set membership, “which pawn is idle”, “who is blocking whom”, simple path length comparisons.
    - Cross-view translation: records to ASCII and ASCII to selected record slices with coordinates. This forces spatial grounding.
    - Forward model micro-steps: predict next-state deltas for deterministic sub-dynamics like progress increments, inventory transfers, task status flips, but not full physics. Keep errors local.
    - Summaries: short, structured summaries per pawn and per task bucket. This helps the model compress attention over large states.

4.  **Behavior cloning and data aggregation**

    - Teacher runs: log trajectories from your heuristic scheduler across many seeds and objectives. Serialize obs, action list, and post-action validator outcomes.
    - BC pass: train to reproduce teacher actions exactly, with label smoothing on priorities. This gives a competent baseline quickly.
    - DAgger-lite: roll the cloned policy in easy scenarios, let the teacher correct only when the policy deviates on critical decisions, append to the dataset. A few rounds reduce covariate shift without full online RL.

5.  **Main objective training** - relative, low-variance credit

    Your plan to sample multiple rollouts per (start, goal) is perfect for preference-style losses:

    - Grouped ranking loss: for each group of rollouts from the same tuple, compute final scores and optimize a listwise or pairwise ranking loss so better trajectories score higher. This is Group Relative Preference Optimization style and is much stabler than token-level REINFORCE.
    - Per-step shaping signals to stabilize: add small dense terms that correlate with competence but are hard to game - idle seconds penalty, distance walked penalty, bonus for completing any designation, bonus for reducing outstanding urgent needs, penalty for invalid actions. Keep these small so they guide but do not dominate the final objective.
    - Advantage normalization within group: center and scale rewards per group to cut variance from task difficulty.

6.  **Observation cadence and context window**

    - Full snapshot every M steps, diffs otherwise. Include the last H action lists and last H tool results, not the whole history.
    - If you exceed token budgets, first reduce entity caps via relevance, then reduce ASCII detail (switch to tiles-only, drop entity glyphs), then increase M or reduce H.

7.  **Invariants and self-checks the model can exploit**

    - Every `pawn.task_id` that is `Some` must appear exactly once in `tasks` with matching `task_digest`. If not, set a `consistency_flag` that the model sees.
    - Sum of items in all inventories plus ground equals world totals per item type. Emit short tallies. This lets the model sanity-check economy reasoning.

8.  **Metrics and eval**

    - Task success rate by objective family, mean time to completion, invalid action rate, idle fraction, path-efficiency proxy.
    - Generalization splits: unseen maps, unseen objective phrasings that share the same underlying metric, larger entity counts.
    - Ablations: remove ASCII view, remove per-pawn embedded tasks, remove deltas. Keep the simplest subset that preserves performance.

9.  **Practicalities that save pain**

    - Use a human-readable but lean grammar for text, not JSON. One record per line, `key=value`, comma-separated, fixed key order. This trims tokens and is easy to diff.
    - Freeze the schema early and bump a version field for any change. Write one canonicalizer in the engine and reuse it everywhere.
    - Deterministic seeds everywhere in the dataset generator. Log the engine build hash with the data.
    - Keep writey actions out of large custom params in Bevy when you later wire the loop - it reduces scheduler conflicts during data capture.

10. **Roadmap**
    1.  Implement the canonicalizer and validator, plus 3-5 tools.
    2.  Generate a small pretraining set for state QA and cross-view tasks and train until ≥98 percent exact-match.
    3.  Log teacher trajectories and run a short BC.
    4.  DAgger on simple maps.
    5.  Switch to grouped ranking on multi-rollout objectives with light shaping.
    6.  Scale map sizes and entity counts, enforce caps, and revisit cropping.

Reflection

Two open design tensions to watch: duplication vs contradiction, and token budget vs observability. The digest plus consistency flags mitigate the first. Relevance-capped entity lists and diff cadence mitigate the second. If this feels good, I can draft the precise line grammar for records, the initial tool list, and the grouped-ranking training loop spec next.
