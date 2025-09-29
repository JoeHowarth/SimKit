use std::{collections::VecDeque, sync::Arc};

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use simkit_core::{
    grid::{TileId, index::TileMapIndex},
    ids::IdIndex,
    impl_hassimid,
};

use crate::{
    StepSystemLabel,
    model::*,
    tasks::reservations::{HeldReservations, Reservations},
};

pub mod job_execution;
pub mod job_planning;
pub mod reservations;
pub mod task_scheduling;
#[cfg(test)]
mod tests;

use job_execution::*;
use job_planning::*;
use task_scheduling::*;

#[derive(Event)]
pub struct CompletedTask(pub TaskId);

#[derive(Event)]
pub struct NewTask(pub TaskSpec);

pub struct TaskPlugin;

impl Plugin for TaskPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TaskBoard>()
            .add_event::<CompletedTask>()
            .add_event::<ToilEvent>()
            .add_event::<NewTask>()
            .init_resource::<Reservations>()
            .add_systems(PreUpdate, handle_new_task)
            .add_systems(
                dbg!(StepSystemLabel::default()),
                (schedule_pawns, step_jobs, evaluate_job, mark_tasks_as_done)
                    .chain(),
            );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskSpec {
    /// Harvest a fixture
    /// target_seq_num should be the value after the harvest has completed
    /// NOT the value before the harvest has started
    Harvest {
        to_harvest: FixtureId,
        target_seq_num: u32,
    },
    Plant(TileId, ItemKind),
    Build(BuildingSpec),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum TaskSpecKind {
    Harvest,
    Plant,
    Build,
}

impl TaskSpec {
    pub fn kind(&self) -> TaskSpecKind {
        match self {
            TaskSpec::Harvest { .. } => TaskSpecKind::Harvest,
            TaskSpec::Plant(_, _) => TaskSpecKind::Plant,
            TaskSpec::Build(_) => TaskSpecKind::Build,
        }
    }

    pub fn completed(&self, world: &World) -> bool {
        match self {
            TaskSpec::Harvest {
                to_harvest,
                target_seq_num,
            } => {
                let harvestable: &Harvestable = world.comp(to_harvest);
                harvestable.seq_num == *target_seq_num
            }
            TaskSpec::Plant(tile_id, _item_kind) => {
                let Some(fixture_id) =
                    world.resource::<TileMapIndex<FixtureId>>().get(*tile_id)
                else {
                    return false;
                };

                let fixture: &Fixture = world.comp(&fixture_id);
                fixture.kind == FixtureKind::BerryBush
            }
            TaskSpec::Build(building_spec) => {
                let Some(fixture_id) = world
                    .resource::<TileMapIndex<FixtureId>>()
                    .get(building_spec.top_left)
                else {
                    return false;
                };

                let fixture: &Fixture = world.comp(&fixture_id);
                fixture.kind == building_spec.fixture_kind
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingSpec {
    pub top_left: TileId,
    pub dim: UVec2,
    pub fixture_kind: FixtureKind,
    pub required_items: Vec<(ItemKind, u32)>,
    pub work_units: u32,
}

// TODO: should this be a component?
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: TaskId,
    pub spec: TaskSpec,
    pub status: TaskStatus,
}

impl_hassimid!(Task, TaskId);

#[derive(Component)]
pub struct WorkPriority(pub Vec<TaskSpecKind>);

#[derive(Component, Debug, Default)]
pub struct Job {
    pub kind: JobKind,
    pub plan: Option<Plan>,
    pub retries: u8,
}

impl Job {
    pub fn new(kind: JobKind, plan: Plan) -> Self {
        Self {
            kind,
            plan: Some(plan),
            retries: 0,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    pub toils: VecDeque<ToilKind>,
    pub reservations: HeldReservations,
}

impl Plan {
    pub fn new(res: &Reservations) -> Self {
        Self {
            toils: VecDeque::new(),
            reservations: HeldReservations::new(res),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub enum JobKind {
    Task(TaskId, TaskSpecKind),
    Sleep,
    Eat,
    #[default]
    None,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TaskStatus {
    Pending,
    Assigned(PawnId),
    Done,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToilKind {
    PlaceConstructionSite {
        building_spec: BuildingSpec,
    },
    Build {
        fixture_id: FixtureId,
    },
    MoveTo {
        target: TileId,
        path: VecDeque<TileId>,
    },
    PickUpItem {
        item_id: ItemId,
    },
    PutDownItem {
        // consider allowing ItemId or ItemKind here
        item_id: ItemId,
        target_tile: TileId,
    },
    StoreItem {
        item_id: ItemId,
        target_fixture_id: FixtureId,
    },
    Plant {
        seed_id: ItemId,
        tile_id: TileId,
    },
    Consume {
        item_id: ItemId,
    },
    Sleep {
        fixture_id: FixtureId,
    },
    Harvest {
        fixture_id: FixtureId,
    },
}

impl ToilKind {
    pub fn move_to_target(&self) -> Option<TileId> {
        let ToilKind::MoveTo { target, .. } = self else {
            return None;
        };
        Some(*target)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ToilResult {
    Done,
    Failed(String),
    Running,
}

impl ToilResult {
    pub fn failure_reason(&self) -> Option<String> {
        match self {
            ToilResult::Failed(reason) => Some(reason.clone()),
            _ => None,
        }
    }
}

#[derive(Event, Debug, PartialEq, Eq)]
pub struct ToilEvent {
    pub pawn_id: PawnId,
    pub toil: ToilKind,
    pub failure_reason: Option<String>,
}

#[derive(Resource, Debug)]
pub struct TaskBoard {
    pub index: IdIndex<TaskId>,
    pub tasks: HashMap<TaskId, Task>,
    pub pending_tasks: HashMap<TaskSpecKind, HashSet<TaskId>>,
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
    pub fn add_task(&mut self, spec: TaskSpec) -> TaskId {
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

    fn pending_tasks_by_kind(
        &self,
        kind: &TaskSpecKind,
    ) -> impl Iterator<Item = &Task> {
        self.pending_tasks
            .get(kind)
            .unwrap()
            .iter()
            .map(|id| self.tasks.get(id).unwrap())
    }

    pub fn tasks_by_status(
        &self,
        status: TaskStatus,
    ) -> impl Iterator<Item = &Task> {
        self.tasks
            .values()
            .filter(move |task| task.status == status)
    }
}

fn mark_tasks_as_done(
    mut task_board: ResMut<TaskBoard>,
    mut completed_tasks: EventReader<CompletedTask>,
) {
    for completed_task in completed_tasks.read() {
        info!("Marking task as done: {:?}", completed_task.0);
        let task = task_board.tasks.get_mut(&completed_task.0).unwrap();
        task.status = TaskStatus::Done;
    }
}

fn handle_new_task(
    mut task_board: ResMut<TaskBoard>,
    mut new_tasks: EventReader<NewTask>,
) {
    for new_task in new_tasks.read() {
        debug!("Adding new task: {:?}", new_task.0);
        task_board.add_task(new_task.0.clone());
    }
}
