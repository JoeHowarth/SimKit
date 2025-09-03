use std::collections::VecDeque;

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use simkit_core::{
    fixed_point::Q40p24,
    grid::{index::TileMapIndex, TileId},
    ids::{IdIndex, SimId},
};

use crate::{
    model::{components::*, ids::*},
    toils::ToilKind,
};

#[derive(PartialEq, Eq, Hash)]
enum TaskSpecKind {
    Harvest,
    Plant,
}

impl TaskSpec {
    fn kind(&self) -> TaskSpecKind {
        match self {
            TaskSpec::Harvest(_) => TaskSpecKind::Harvest,
            TaskSpec::Plant(_, _) => TaskSpecKind::Plant,
        }
    }
}

enum TaskSpec {
    Harvest(FixtureId),
    Plant(TileId, ItemKind),
}

struct Task {
    id: TaskId,
    spec: TaskSpec,
    status: TaskStatus,
}

#[derive(Component)]
struct WorkPriority(pub Vec<TaskSpecKind>);

#[derive(Component)]
struct Job {
    kind: JobKind,
    plan: VecDeque<ToilKind>,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum JobKind {
    Task(TaskId),
    Sleep,
    Eat,
    None,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum TaskStatus {
    Pending,
    Assigned(PawnId),
    Done,
    Cancelled,
}

#[derive(Resource)]
struct TaskBoard {
    index: IdIndex<TaskId>,
    tasks: HashMap<TaskId, Task>,
    pending_tasks: HashMap<TaskSpecKind, HashSet<TaskId>>,
}

impl Default for TaskBoard {
    fn default() -> Self {
        Self {
            index: IdIndex::default(),
            tasks: HashMap::new(),
            pending_tasks: HashMap::from_iter([
                (TaskSpecKind::Harvest, HashSet::new()),
                (TaskSpecKind::Plant, HashSet::new()),
            ]),
        }
    }
}

impl TaskBoard {
    /// Add a never before seen task to the pending state
    fn add_task(&mut self, spec: TaskSpec) -> TaskId {
        let id = self.index.alloc(None);
        let kind = spec.kind();
        let task = Task {
            id,
            spec,
            status: TaskStatus::Pending,
        };

        self.tasks.insert(id, task);
        self.pending_tasks.entry(kind).or_default().insert(id);
        id
    }

    /// Move a task to the assigned state
    fn set_assigned(&mut self, id: TaskId, pawn_id: PawnId) {
        let task = self.tasks.get_mut(&id).unwrap();
        task.status = TaskStatus::Assigned(pawn_id);
        self.pending_tasks
            .get_mut(&task.spec.kind())
            .unwrap()
            .remove(&id);
    }

    /// Move a task back to the pending state
    /// Must only be called on tasks that are already assigned
    fn return_to_pending(&mut self, id: TaskId) {
        let task = self.tasks.get_mut(&id).unwrap();
        task.status = TaskStatus::Pending;
        self.pending_tasks
            .get_mut(&task.spec.kind())
            .unwrap()
            .insert(id);
    }

    fn pending_tasks_by_kind(&self, kind: &TaskSpecKind) -> impl Iterator<Item = &Task> {
        self.pending_tasks
            .get(kind)
            .unwrap()
            .iter()
            .map(|id| self.tasks.get(id).unwrap())
    }
}

fn schedule_pawns(
    mut pawns: Query<(&Pawn, &TileId, &WorkPriority, &mut Job)>,
    mut task_board: ResMut<TaskBoard>,
    fixtures: Query<(&Fixture, &TileId)>,
    items: Query<(&Item, &TileId)>,
    fixture_index: Res<IdIndex<FixtureId>>,
    item_index: Res<IdIndex<ItemId>>,
) {
    // TODO: use a stable ordering of pawns

    for (pawn, pos, work_priority, mut job) in pawns.iter_mut() {
        // If job is running, check if it should be preempted
        if JobKind::None != job.kind {
            if let Some(prempt) = should_preempt(pawn, job.kind) {
                // If job should be preempted and it's not the same job, preempt it
                if let JobKind::Task(task_id) = &job.kind {
                    task_board.return_to_pending(*task_id);
                }
                *job = Job {
                    kind: prempt,
                    plan: VecDeque::new(),
                };
            }
            continue;
        }

        // If no job is running, choose a new job
        let kind = choose_next_job(
            &task_board,
            pawn,
            pos,
            work_priority,
            &fixtures,
            &items,
            &fixture_index,
            &item_index,
        );

        if let JobKind::Task(task_id) = &kind {
            task_board.set_assigned(*task_id, pawn.id);
        }

        *job = Job {
            kind,
            plan: VecDeque::new(),
        };
    }
}

impl Pawn {
    fn sleep_priority(&self) -> Q40p24 {
        Q40p24::from(1) - self.sleep
    }

    fn eat_priority(&self) -> Q40p24 {
        Q40p24::from(1) - self.hunger
    }
}

fn should_preempt(pawn: &Pawn, current_job: JobKind) -> Option<JobKind> {
    if current_job == JobKind::Eat {
        if pawn.sleep_priority() > Q40p24::from(0.6) {
            return Some(JobKind::Sleep);
        }
        return None;
    }

    // Premption is in a stable order, so it will not thrash
    if pawn.eat_priority() > Q40p24::from(0.8) {
        return Some(JobKind::Eat);
    }

    if current_job == JobKind::Sleep {
        return None;
    }

    if pawn.sleep_priority() > Q40p24::from(0.8) {
        return Some(JobKind::Sleep);
    }

    None
}

fn choose_next_job(
    //
    pending: &TaskBoard,
    pawn: &Pawn,
    pos: &TileId,
    work_priority: &WorkPriority,
    fixtures: &Query<(&Fixture, &TileId)>,
    items: &Query<(&Item, &TileId)>,
    fixture_index: &IdIndex<FixtureId>,
    item_index: &IdIndex<ItemId>,
) -> JobKind {
    // Check if needs are urgent
    // Sleep and eat threshold are lower than when we have a job
    if pawn.eat_priority() > Q40p24::from(0.6) {
        return JobKind::Eat;
    }
    if pawn.sleep_priority() > Q40p24::from(0.6) {
        return JobKind::Sleep;
    }

    for kind in work_priority.0.iter() {
        // Find highest priority task for this kind
        let max = pending
            .pending_tasks_by_kind(kind)
            .filter_map(|task| {
                let priority =
                    task.spec
                        .priority(pawn, pos, fixtures, items, fixture_index, item_index)?;
                Some((priority, task))
            })
            .max_by_key(|(priority, task)| (*priority, -(task.id.to_u64() as i64)));

        // If a task is found, return it
        if let Some((_, task)) = max {
            return JobKind::Task(task.id);
        }
    }

    JobKind::None
}

impl TaskSpec {
    fn priority(
        &self,
        pawn: &Pawn,
        pos: &TileId,
        fixtures: &Query<(&Fixture, &TileId)>,
        items: &Query<(&Item, &TileId)>,
        fixture_index: &IdIndex<FixtureId>,
        _item_index: &IdIndex<ItemId>,
    ) -> Option<Q40p24> {
        match self {
            TaskSpec::Harvest(_) => self.harvest_priority(pos, fixtures, fixture_index),
            TaskSpec::Plant(_, _) => self.plant_priority(pawn, pos, items, fixtures),
        }
    }

    fn harvest_priority(
        &self,
        pos: &TileId,
        fixtures: &Query<(&Fixture, &TileId)>,
        fixture_index: &IdIndex<FixtureId>,
    ) -> Option<Q40p24> {
        let TaskSpec::Harvest(fixture_id) = self else {
            panic!("Harvest priority called for non-harvest task");
        };

        let (fixture, fixture_pos) = fixtures.get(fixture_index.get(fixture_id)).unwrap();
        if fixture.harvest_countdown.is_none() || fixture.harvest_countdown.unwrap() > 0 {
            return None;
        }

        let distance = manhattan(*pos, *fixture_pos);
        Some(distance_to_score(distance))
    }

    fn plant_priority(
        &self,
        pawn: &Pawn,
        pawn_pos: &TileId,
        items: &Query<(&Item, &TileId)>,
        fixtures: &Query<(&Fixture, &TileId)>,
    ) -> Option<Q40p24> {
        let TaskSpec::Plant(fixture_pos, item_kind) = self else {
            panic!("Plant priority called for non-plant task");
        };
        let (item_pos, _) =
            neartest_item_position(pawn, pawn_pos, item_kind, items, fixtures)?;
        let distance_to_get_item = manhattan(*pawn_pos, item_pos);

        let distance = manhattan(*fixture_pos, item_pos);
        let distance_score = distance_to_score(distance + distance_to_get_item);
        Some(distance_score)
    }
}

pub fn neartest_item_position(
    pawn: &Pawn,
    pawn_pos: &TileId,
    item_kind: &ItemKind,
    items: &Query<(&Item, &TileId)>,
    fixtures: &Query<(&Fixture, &TileId)>,
) -> Option<(TileId, ItemId)> {
    if let Some(item_id) = item_in_inventory(item_kind, &pawn.inventory) {
        return Some((*pawn_pos, item_id));
    }

    let on_ground = nearest_item_on_ground(item_kind, pawn_pos, items)
        .map(|(item_id, pos, d)| (item_id, pos, d));
    let fixture = nearest_fixture_with_item(item_kind, pawn_pos, fixtures)
        .map(|(_, pos, item_id, d)| (item_id, pos, d));

    match (on_ground, fixture) {
        (
            Some((on_ground_item_id, item_pos, on_ground_d)),
            Some((fixture_item_id, fixture_pos, fixture_d)),
        ) => {
            if on_ground_d < fixture_d {
                return Some((item_pos, on_ground_item_id));
            }
            return Some((fixture_pos, fixture_item_id));
        }
        (Some((item_id, item_pos, _)), None) => {
            return Some((item_pos, item_id));
        }
        (None, Some((item_id, fixture_pos, _))) => {
            return Some((fixture_pos, item_id));
        }
        (None, None) => {
            return None;
        }
    }
}

fn distance_to_score(distance: impl Into<Q40p24>) -> Q40p24 {
    let distance = distance.into();
    Q40p24::ONE / (Q40p24::ONE + distance)
}

pub fn manhattan(a: TileId, b: TileId) -> u32 {
    ((a.x - b.x).abs() + (a.y - b.y).abs()) as u32
}

pub fn nearest_item_on_ground(
    target_kind: &ItemKind,
    current_pos: &TileId,
    items: &Query<(&Item, &TileId)>,
) -> Option<(ItemId, TileId, u32)> {
    // find nearest item on ground that matches item
    let mut nearest = None;
    for (item, item_pos) in items.iter() {
        if item.kind == *target_kind {
            let distance = manhattan(*current_pos, *item_pos);
            if distance > nearest.map(|(_, _, d)| d).unwrap_or(u32::MAX) {
                continue;
            }
            nearest = Some((item.id, *item_pos, distance));
        }
    }
    nearest
}

pub fn nearest_fixture_with_item(
    target_kind: &ItemKind,
    current_pos: &TileId,
    fixtures: &Query<(&Fixture, &TileId)>,
) -> Option<(FixtureId, TileId, ItemId, u32)> {
    // find nearest fixture that contains item
    let mut nearest = None;
    for (fixture, fixture_pos) in fixtures.iter() {
        let item_id = item_in_inventory(target_kind, &fixture.inventory);
        let Some(item_id) = item_id else {
            continue;
        };

        let distance = manhattan(*current_pos, *fixture_pos);
        if distance > nearest.map(|(_, _, _, d)| d).unwrap_or(u32::MAX) {
            continue;
        }

        nearest = Some((fixture.id, *fixture_pos, item_id, distance));
    }
    nearest
}

pub fn item_in_inventory(item: &ItemKind, inventory: &Vec<(ItemId, ItemKind)>) -> Option<ItemId> {
    inventory
        .iter()
        .find(|(_, kind)| kind == item)
        .map(|(id, _)| *id)
}
