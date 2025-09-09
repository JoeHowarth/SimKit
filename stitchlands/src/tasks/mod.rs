use std::collections::VecDeque;

use bevy::{
    ecs::schedule::ScheduleLabel,
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use simkit_core::{
    grid::{index::TileMapIndex, TileId},
    ids::IdIndex,
    impl_hassimid,
};

use crate::model::*;

pub mod job_execution;
pub mod job_planning;
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

pub struct TaskPlugin<S = FixedUpdate> {
    pub(crate) schedule: S,
}

impl Default for TaskPlugin<FixedUpdate> {
    fn default() -> Self {
        Self {
            schedule: FixedUpdate,
        }
    }
}

impl<S: ScheduleLabel + Clone> Plugin for TaskPlugin<S> {
    fn build(&self, app: &mut App) {
        app.init_resource::<TaskBoard>()
            .add_event::<CompletedTask>()
            .add_event::<NewTask>()
            .add_systems(PreUpdate, handle_new_task)
            .add_systems(
                self.schedule.clone(),
                (schedule_pawns, step_jobs, mark_tasks_as_done).chain(),
            );
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum TaskSpecKind {
    Harvest,
    Plant,
}

impl TaskSpec {
    pub fn kind(&self) -> TaskSpecKind {
        match self {
            TaskSpec::Harvest(_) => TaskSpecKind::Harvest,
            TaskSpec::Plant(_, _) => TaskSpecKind::Plant,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSpec {
    Harvest(FixtureId),
    Plant(TileId, ItemKind),
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
    pub current_toil: Option<ToilKind>,
    pub plan: VecDeque<ToilKind>,
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
    ReserveItem {
        item: ItemId,
    },
    MoveTo {
        target: TileId,
        path: VecDeque<TileId>,
    },
    PickUp {
        item_id: ItemId,
    },
    PutDown {
        // consider allowing ItemId or ItemKind here
        item_id: ItemId,
        target_tile: TileId,
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

#[derive(Debug, PartialEq, Eq)]
pub enum ToilResult {
    Done,
    Failed(String),
    Running,
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
        task_board.add_task(new_task.0);
    }
}
