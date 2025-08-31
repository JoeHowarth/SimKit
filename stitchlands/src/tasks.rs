use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::components::Pawn;
use crate::ids::{PawnId, TaskId};
use simkit_core::grid::TileId;
use simkit_core::ids::IdAllocator;

// ---------- Constants
pub const PRIO_HARVEST: i32 = 3;
pub const PRIO_EAT: i32 = 9;
pub const PRIO_SLEEP: i32 = 8;

pub const EAT_LOW: f32 = 0.30;
pub const SLEEP_LOW: f32 = 0.30;

pub const HUNGER_DECAY_PER_TICK: f32 = 0.002; // optional decay
pub const REST_DECAY_PER_TICK: f32 = 0.001; // optional decay
pub const ENABLE_NEEDS_DECAY: bool = true;
pub const EAT_TICKS: u32 = 12;
pub const SLEEP_TICKS: u32 = 16;
pub const WORK_TICKS: u32 = 5;

// ---------- Components / Resources

#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize, Reflect)]
#[reflect(Component)]
pub struct Needs {
    pub hunger: f32, // 0..1 (0 empty, 1 full)
    pub rest: f32,   // 0..1
}

impl Default for Needs {
    fn default() -> Self {
        Self { hunger: 1.0, rest: 1.0 }
    }
}

#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize, Reflect)]
#[reflect(Component)]
pub enum Designation {
    Harvest(TileId),
}

#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize, Reflect)]
#[reflect(Component)]
pub struct TaskRef(pub Option<TaskId>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum TaskStatus {
    Pending,
    Running,
    Done,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum TaskKind {
    Harvest { tile: TileId },
    EatAuto { pawn: PawnId },
    SleepAuto { pawn: PawnId },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Reflect)]
pub struct Task {
    pub id: TaskId,
    pub kind: TaskKind,
    pub status: TaskStatus,
    pub priority: i32,
    pub owner: Option<PawnId>,
}

#[derive(Resource, Debug, Default)]
pub struct TaskBoard {
    pub tasks: Vec<Task>,
}

impl TaskBoard {
    pub fn add_task(
        &mut self,
        alloc: &mut IdAllocator<TaskId>,
        kind: TaskKind,
        priority: i32,
        owner: Option<PawnId>,
    ) -> TaskId {
        let id = alloc.assign(None);
        let t = Task {
            id,
            kind,
            status: TaskStatus::Pending,
            priority,
            owner,
        };
        self.tasks.push(t);
        id
    }

    pub fn get_mut(&mut self, id: TaskId) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    pub fn contains(&self, id: TaskId) -> bool {
        self.tasks.iter().any(|t| t.id == id)
    }

    pub fn iter_sorted(&self) -> impl Iterator<Item = &Task> {
        let mut idx: Vec<usize> = (0..self.tasks.len()).collect();
        idx.sort_unstable_by_key(|&i| self.tasks[i].id);
        idx.into_iter().map(move |i| &self.tasks[i])
    }

    pub fn remove_if<F: FnMut(&Task) -> bool>(&mut self, mut f: F) {
        self.tasks.retain(|t| !f(t));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum UniqueTargetKey {
    Tile(TileId),
}

#[derive(Resource, Debug, Default)]
pub struct UniqueTargetRes {
    pub owners: HashMap<UniqueTargetKey, TaskId>,
}

impl UniqueTargetRes {
    pub fn try_reserve(&mut self, key: UniqueTargetKey, task_id: TaskId) -> bool {
        match self.owners.get(&key) {
            None => {
                self.owners.insert(key, task_id);
                true
            }
            Some(&owner) if owner == task_id => true,
            Some(_) => false,
        }
    }

    pub fn release(&mut self, key: UniqueTargetKey, task_id: TaskId) {
        if matches!(self.owners.get(&key), Some(owner) if *owner == task_id) {
            self.owners.remove(&key);
        }
    }

    pub fn is_owner(&self, key: UniqueTargetKey, task_id: TaskId) -> bool {
        matches!(self.owners.get(&key), Some(owner) if *owner == task_id)
    }
}

// ---------- Systems (PreStep)

// For each (Designation::Harvest, TaskRef(None)) create a Harvest task and set TaskRef.
pub fn designation_spawner(
    mut q: Query<(&Designation, &mut TaskRef), With<crate::WorldTag>>,
    mut board: ResMut<TaskBoard>,
    mut task_alloc: ResMut<IdAllocator<TaskId>>,
) {
    for (d, mut tref) in q.iter_mut() {
        match d {
            Designation::Harvest(tile) => {
                match tref.0 {
                    None => {
                        let id = board.add_task(
                            &mut task_alloc,
                            TaskKind::Harvest { tile: *tile },
                            PRIO_HARVEST,
                            None,
                        );
                        tref.0 = Some(id);
                    }
                    Some(id) => {
                        if !board.contains(id) {
                            let new_id = board.add_task(
                                &mut task_alloc,
                                TaskKind::Harvest { tile: *tile },
                                PRIO_HARVEST,
                                None,
                            );
                            tref.0 = Some(new_id);
                        }
                    }
                }
            }
        }
    }
}

// Emit needs tasks for pawns below thresholds; also apply optional tiny decay.
pub fn needs_daemon_emit(
    mut pawns: Query<(&Pawn, &mut Needs)>,
    mut board: ResMut<TaskBoard>,
    mut task_alloc: ResMut<IdAllocator<TaskId>>,
) {
    for (pawn, mut needs) in pawns.iter_mut() {
        if ENABLE_NEEDS_DECAY {
            needs.hunger = (needs.hunger - HUNGER_DECAY_PER_TICK).clamp(0.0, 1.0);
            needs.rest = (needs.rest - REST_DECAY_PER_TICK).clamp(0.0, 1.0);
        }

        // Eat task
        if needs.hunger < EAT_LOW {
            let exists = board
                .tasks
                .iter()
                .any(|t| matches!(t.kind, TaskKind::EatAuto { pawn: p } if p == pawn.id)
                    && !matches!(t.status, TaskStatus::Done | TaskStatus::Cancelled));
            if !exists {
                board.add_task(
                    &mut task_alloc,
                    TaskKind::EatAuto { pawn: pawn.id },
                    PRIO_EAT,
                    Some(pawn.id),
                );
            }
        }

        // Sleep task
        if needs.rest < SLEEP_LOW {
            let exists = board
                .tasks
                .iter()
                .any(|t| matches!(t.kind, TaskKind::SleepAuto { pawn: p } if p == pawn.id)
                    && !matches!(t.status, TaskStatus::Done | TaskStatus::Cancelled));
            if !exists {
                board.add_task(
                    &mut task_alloc,
                    TaskKind::SleepAuto { pawn: pawn.id },
                    PRIO_SLEEP,
                    Some(pawn.id),
                );
            }
        }
    }
}

// Remove Done/Cancelled tasks each tick.
pub fn task_prune_minimal(mut board: ResMut<TaskBoard>) {
    board.remove_if(|t| matches!(t.status, TaskStatus::Done | TaskStatus::Cancelled));
}

// ---------- Job runtime (Step/PostStep)

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct Job {
    pub task: TaskId,
    pub driver: JobDriver,
}

#[derive(Debug, Clone, Copy, Reflect)]
pub enum JobDriver {
    Harvest {
        target: TileId,
        state: HarvestState,
        reserved: Option<UniqueTargetKey>,
    },
    EatAuto { ticks_left: u32 },
    SleepAuto { ticks_left: u32 },
}

#[derive(Debug, Clone, Copy, Reflect)]
pub enum HarvestState {
    ReserveTarget,
    TravelCountdown { ticks_left: u32 },
    WorkCountdown { ticks_left: u32 },
}

#[derive(Debug, Clone)]
pub enum LogEvent {
    Assign {
        pawn: PawnId,
        task: TaskId,
        prio: i32,
        dist: i32,
        kind: TaskKind,
    },
    Preempt {
        pawn: PawnId,
        from_task: TaskId,
        kind: TaskKind,
        reason: &'static str,
        released: Option<UniqueTargetKey>,
    },
}

#[derive(Resource, Default)]
pub struct LogBuffer {
    pub events: Vec<LogEvent>,
}

fn manhattan(a: TileId, b: TileId) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

pub fn hard_need_interrupts(
    mut commands: Commands,
    mut board: ResMut<TaskBoard>,
    mut uniq: ResMut<UniqueTargetRes>,
    mut logs: ResMut<LogBuffer>,
    mut q: Query<(Entity, &Pawn, &Needs, &mut Job)>,
    mut task_alloc: ResMut<IdAllocator<TaskId>>,
) {
    for (e, pawn, needs, mut job) in q.iter_mut() {
        // Only interrupt non-needs jobs (Harvest) when needs are low
        let eat_low = needs.hunger < EAT_LOW;
        let sleep_low = needs.rest < SLEEP_LOW;
        if !(eat_low || sleep_low) {
            continue;
        }
        // If current job is Harvest, preempt
        if let JobDriver::Harvest { target: _, state: _, reserved } = job.driver {
            if let Some(t) = board.get_mut(job.task) {
                t.status = TaskStatus::Pending;
            }
            // Release reservation if held
            let mut released: Option<UniqueTargetKey> = None;
            if let Some(key) = reserved {
                uniq.release(key, job.task);
                released = Some(key);
            }
            // Log preemption
            let kind = if let Some(t) = board.get_mut(job.task) {
                t.kind
            } else {
                TaskKind::Harvest { tile: TileId::new(0, 0) }
            };
            let reason = if eat_low { "Eat" } else { "Sleep" };
            logs.events.push(LogEvent::Preempt {
                pawn: pawn.id,
                from_task: job.task,
                kind,
                reason,
                released,
            });
            // Remove job from pawn
            commands.entity(e).remove::<Job>();

            // Ensure a needs task exists (scan board)
            if eat_low {
                let exists = board
                    .tasks
                    .iter()
                    .any(|t| matches!(t.kind, TaskKind::EatAuto { pawn: p } if p == pawn.id)
                        && !matches!(t.status, TaskStatus::Done | TaskStatus::Cancelled));
                if !exists {
                    board.add_task(
                        &mut task_alloc,
                        TaskKind::EatAuto { pawn: pawn.id },
                        PRIO_EAT,
                        Some(pawn.id),
                    );
                }
            }
            if sleep_low {
                let exists = board
                    .tasks
                    .iter()
                    .any(|t| matches!(t.kind, TaskKind::SleepAuto { pawn: p } if p == pawn.id)
                        && !matches!(t.status, TaskStatus::Done | TaskStatus::Cancelled));
                if !exists {
                    board.add_task(
                        &mut task_alloc,
                        TaskKind::SleepAuto { pawn: pawn.id },
                        PRIO_SLEEP,
                        Some(pawn.id),
                    );
                }
            }
        }
    }
}

pub fn scheduler_assign(
    mut commands: Commands,
    mut board: ResMut<TaskBoard>,
    mut logs: ResMut<LogBuffer>,
    mut q: Query<(Entity, &Pawn, &TileId), Without<Job>>,
) {
    // Iterate pawns in ascending PawnId
    let mut pawns: Vec<(Entity, PawnId, TileId)> = q
        .iter_mut()
        .map(|(e, p, pos)| (e, p.id, *pos))
        .collect();
    pawns.sort_by_key(|(_, pid, _)| *pid);

    for (e, pid, pos) in pawns.into_iter() {
        // Pick best pending task for this pawn
        let mut best: Option<(TaskId, i32, i32, TaskKind, i32)> = None; // (id, prio, dist, kind, neg_tid)
        for t in board.iter_sorted() {
            if !matches!(t.status, TaskStatus::Pending) {
                continue;
            }
            // Respect ownership for needs tasks
            match t.kind {
                TaskKind::EatAuto { pawn } | TaskKind::SleepAuto { pawn } => {
                    if pawn != pid {
                        continue;
                    }
                }
                _ => {}
            }
            let (dist, kind) = match t.kind {
                TaskKind::Harvest { tile } => (manhattan(pos, tile), t.kind),
                TaskKind::EatAuto { .. } | TaskKind::SleepAuto { .. } => (0, t.kind),
            };

            let prio = t.priority;
            let neg_tid = -(t.id.0 as i32);
            let candidate = (t.id, prio, dist, kind, neg_tid);
            if let Some((_bid, bprio, bdist, _bkind, bneg)) = best {
                // Higher (prio, -dist, -tid) wins
                if (prio, -dist, neg_tid) > (bprio, -bdist, bneg) {
                    best = Some(candidate);
                }
            } else {
                best = Some(candidate);
            }
        }

        // Assign if any
        if let Some((id, prio, dist, kind, _)) = best {
            if let Some(t) = board.get_mut(id) {
                t.status = TaskStatus::Running;
            }
            let driver = match kind {
                TaskKind::Harvest { tile } => JobDriver::Harvest {
                    target: tile,
                    state: HarvestState::ReserveTarget,
                    reserved: None,
                },
                TaskKind::EatAuto { .. } => JobDriver::EatAuto { ticks_left: EAT_TICKS },
                TaskKind::SleepAuto { .. } => JobDriver::SleepAuto { ticks_left: SLEEP_TICKS },
            };
            commands.entity(e).insert(Job { task: id, driver });
            logs.events.push(LogEvent::Assign {
                pawn: pid,
                task: id,
                prio,
                dist,
                kind,
            });
        }
    }
}

pub fn job_tick(
    mut commands: Commands,
    mut board: ResMut<TaskBoard>,
    mut uniq: ResMut<UniqueTargetRes>,
    mut q: Query<(Entity, &mut Job, &mut Needs, &TileId, &Pawn)>,
    mut designations: Query<(Entity, &Designation, &TaskRef)>,
) {
    for (e, mut job, mut needs, pos, pawn) in q.iter_mut() {
        let task_id = job.task;
        match job.driver {
            JobDriver::EatAuto { ref mut ticks_left } => {
                if *ticks_left > 0 {
                    *ticks_left -= 1;
                }
                if *ticks_left == 0 {
                    needs.hunger = (needs.hunger + 0.5).clamp(0.0, 1.0);
                    if let Some(t) = board.get_mut(task_id) {
                        t.status = TaskStatus::Done;
                    }
                    commands.entity(e).remove::<Job>();
                }
            }
            JobDriver::SleepAuto { ref mut ticks_left } => {
                if *ticks_left > 0 {
                    *ticks_left -= 1;
                }
                if *ticks_left == 0 {
                    needs.rest = (needs.rest + 0.5).clamp(0.0, 1.0);
                    if let Some(t) = board.get_mut(task_id) {
                        t.status = TaskStatus::Done;
                    }
                    commands.entity(e).remove::<Job>();
                }
            }
            JobDriver::Harvest {
                target,
                ref mut state,
                ref mut reserved,
            } => {
                match *state {
                    HarvestState::ReserveTarget => {
                        let key = UniqueTargetKey::Tile(target);
                        if uniq.try_reserve(key, task_id) {
                            *reserved = Some(key);
                            let d = manhattan(*pos, target).max(1) as u32;
                            *state = HarvestState::TravelCountdown { ticks_left: d };
                        }
                    }
                    HarvestState::TravelCountdown { ref mut ticks_left } => {
                        if *ticks_left > 0 {
                            *ticks_left -= 1;
                        }
                        if *ticks_left == 0 {
                            *state = HarvestState::WorkCountdown { ticks_left: WORK_TICKS };
                        }
                    }
                    HarvestState::WorkCountdown { ref mut ticks_left } => {
                        if *ticks_left > 0 {
                            *ticks_left -= 1;
                        }
                        if *ticks_left == 0 {
                            if let Some(t) = board.get_mut(task_id) {
                                t.status = TaskStatus::Done;
                            }
                            if let Some(key) = *reserved {
                                uniq.release(key, task_id);
                            }
                            // Despawn designation entity for this task if exists
                            if let Some((de, _, _)) = designations
                                .iter_mut()
                                .find(|(_, _d, tr)| tr.0 == Some(task_id))
                            {
                                commands.entity(de).despawn_recursive();
                            }
                            commands.entity(e).remove::<Job>();
                        }
                    }
                }
            }
        }
    }
}

pub fn release_stale_reservations(board: Res<TaskBoard>, mut uniq: ResMut<UniqueTargetRes>) {
    // Collect keys to check to avoid borrow issues
    let keys: Vec<UniqueTargetKey> = uniq.owners.keys().copied().collect();
    for key in keys {
        let release = match uniq.owners.get(&key).copied() {
            None => false,
            Some(tid) => match board.tasks.iter().find(|t| t.id == tid) {
                None => true,
                Some(t) => !matches!(t.status, TaskStatus::Running),
            },
        };
        if release {
            if let Some(tid) = uniq.owners.get(&key).copied() {
                uniq.release(key, tid);
            }
        }
    }
}

pub fn print_tick_logs(
    mut logs: ResMut<LogBuffer>,
    playback: Res<simkit_core::Playback>,
) {
    if logs.events.is_empty() {
        return;
    }
    let tick = playback.tick.0;
    // Sort deterministically by (pawn_id, task_id, variant)
    logs.events.sort_by(|a, b| match (a, b) {
        (
            LogEvent::Assign { pawn: pa, task: ta, .. },
            LogEvent::Assign { pawn: pb, task: tb, .. },
        ) => (pa, ta).cmp(&(pb, tb)),
        (
            LogEvent::Preempt { pawn: pa, from_task: ta, .. },
            LogEvent::Preempt { pawn: pb, from_task: tb, .. },
        ) => (pa, ta).cmp(&(pb, tb)),
        (LogEvent::Assign { pawn: pa, task: ta, .. }, LogEvent::Preempt { pawn: pb, from_task: tb, .. }) => (pa, ta).cmp(&(pb, tb)),
        (LogEvent::Preempt { pawn: pa, from_task: ta, .. }, LogEvent::Assign { pawn: pb, task: tb, .. }) => (pa, ta).cmp(&(pb, tb)),
    });
    for ev in logs.events.drain(..) {
        match ev {
            LogEvent::Assign { pawn, task, prio, dist, kind } => {
                let kind_s = match kind {
                    TaskKind::Harvest { .. } => "Harvest",
                    TaskKind::EatAuto { .. } => "EatAuto",
                    TaskKind::SleepAuto { .. } => "SleepAuto",
                };
                println!(
                    "ASSIGN tick={} pawn={} task={} kind={} prio={} dist={} score=({},-{},-{})",
                    tick, pawn.0, task.0, kind_s, prio, dist, prio, dist, task.0
                );
            }
            LogEvent::Preempt { pawn, from_task, kind, reason, released } => {
                let kind_s = match kind {
                    TaskKind::Harvest { .. } => "Harvest",
                    TaskKind::EatAuto { .. } => "EatAuto",
                    TaskKind::SleepAuto { .. } => "SleepAuto",
                };
                let rel_s = match released {
                    None => "None".to_string(),
                    Some(UniqueTargetKey::Tile(t)) => format!("Tile({}, {})", t.x, t.y),
                };
                println!(
                    "PREEMPT tick={} pawn={} from_task={}/{} reason={} released={}",
                    tick, pawn.0, from_task.0, kind_s, reason, rel_s
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use simkit_core::ids::IdAllocator;

    #[test]
    fn unique_target_res_basic() {
        let mut res = UniqueTargetRes::default();
        let key = UniqueTargetKey::Tile(TileId::new(1, 2));
        let t1 = TaskId(1000);
        let t2 = TaskId(1001);
        assert!(res.try_reserve(key, t1));
        assert!(res.is_owner(key, t1));
        assert!(!res.try_reserve(key, t2));
        res.release(key, t1);
        assert!(!res.is_owner(key, t1));
        assert!(res.try_reserve(key, t2));
        assert!(res.is_owner(key, t2));
    }

    #[test]
    fn designation_spawner_idempotent() {
        let mut app = App::new();
        app.init_resource::<TaskBoard>()
            .init_resource::<IdAllocator<TaskId>>()
            .add_systems(Startup, designation_spawner)
            .add_systems(Update, designation_spawner);

        // Spawn one designation entity with TaskRef(None)
        let tile = TileId::new(3, 4);
        app.world_mut().spawn((
            crate::WorldTag,
            Name::new("D"),
            Designation::Harvest(tile),
            TaskRef(None),
        ));

        app.update();
        // Run once more to ensure Startup/Update ordering settles
        app.update();

        // Verify one task exists and TaskRef is set
        {
            let board = app.world().resource::<TaskBoard>();
            assert_eq!(board.tasks.len(), 1);
            assert!(matches!(board.tasks[0].kind, TaskKind::Harvest { tile: t } if t == tile));
        }

        // Run again -> still one task
        app.update();
        app.update();
        {
            let board = app.world().resource::<TaskBoard>();
            assert_eq!(board.tasks.len(), 1);
        }

        // Simulate missing task: clear board but keep TaskRef(Some(id)) on entity
        app.world_mut().resource_mut::<TaskBoard>().tasks.clear();
        app.update();
        app.update();
        // Should recreate exactly one task
        {
            let board = app.world().resource::<TaskBoard>();
            assert_eq!(board.tasks.len(), 1);
            assert!(matches!(board.tasks[0].kind, TaskKind::Harvest { tile: t } if t == tile));
        }
    }

    #[test]
    fn needs_daemon_emit_no_duplicates() {
        let mut app = App::new();
        app.init_resource::<TaskBoard>()
            .init_resource::<IdAllocator<TaskId>>()
            .add_systems(Startup, needs_daemon_emit);

        // Pawn below hunger threshold
        let pawn = Pawn { id: PawnId(1000) };
        let needs = Needs { hunger: 0.2, rest: 0.9 };
        app.world_mut().spawn((pawn, needs));

        app.update();
        {
            let board = app.world().resource::<TaskBoard>();
            let count = board
                .tasks
                .iter()
                .filter(|t| matches!(t.kind, TaskKind::EatAuto { pawn: p } if p == pawn.id))
                .count();
            assert_eq!(count, 1);
        }
        // Run again, should remain one
        app.update();
        {
            let board = app.world().resource::<TaskBoard>();
            let count = board
                .tasks
                .iter()
                .filter(|t| matches!(t.kind, TaskKind::EatAuto { pawn: p } if p == pawn.id))
                .count();
            assert_eq!(count, 1);
        }
    }

    #[test]
    fn scheduler_assign_picks_low_tid_on_tie() {
        let mut app = App::new();
        app.init_resource::<TaskBoard>()
            .init_resource::<IdAllocator<TaskId>>()
            .init_resource::<LogBuffer>()
            .add_systems(Startup, scheduler_assign);

        // One idle pawn at (0,0)
        let pawn = Pawn { id: PawnId(1000) };
        let pos = TileId::new(0, 0);
        app.world_mut().spawn((pawn, pos));

        // Two equal tasks at same distance and priority
        let mut alloc = app.world_mut().resource_mut::<IdAllocator<TaskId>>().clone();
        {
            let mut board = app.world_mut().resource_mut::<TaskBoard>();
            let id1 = board.add_task(&mut alloc, TaskKind::Harvest { tile: TileId::new(1, 0) }, PRIO_HARVEST, None);
            let id2 = board.add_task(&mut alloc, TaskKind::Harvest { tile: TileId::new(1, 0) }, PRIO_HARVEST, None);
            // Write back allocator (simulate the resource advanced by two)
            *app.world_mut().resource_mut::<IdAllocator<TaskId>>() = alloc;
            // Ensure lower id will win
            assert!(id1.0 < id2.0);
        }

        app.update();

        // Pawn should have a Job and chosen task set to Running
        {
            let world = app.world_mut();
            let mut q = world.query::<(&Job, &Pawn)>();
            let v: Vec<_> = q.iter(world).collect();
            assert_eq!(v.len(), 1);
            let (job, p) = v[0];
            assert_eq!(p.id, PawnId(1000));
            let board = world.resource::<TaskBoard>();
            let chosen = board.tasks.iter().find(|t| t.id == job.task).unwrap();
            assert_eq!(chosen.status, TaskStatus::Running);
            // The other should remain Pending
            let other = board.tasks.iter().find(|t| t.id != job.task).unwrap();
            assert_eq!(other.status, TaskStatus::Pending);
        }
    }

    #[test]
    fn hard_need_interrupts_releases_and_emits() {
        let mut app = App::new();
        app.init_resource::<TaskBoard>()
            .init_resource::<UniqueTargetRes>()
            .init_resource::<IdAllocator<TaskId>>()
            .init_resource::<LogBuffer>()
            .add_systems(Startup, hard_need_interrupts);

        // Prepare a running harvest task
        let tile = TileId::new(5, 5);
        let mut alloc = app.world_mut().resource_mut::<IdAllocator<TaskId>>().clone();
        let task_id = {
            let mut board = app.world_mut().resource_mut::<TaskBoard>();
            let id = board.add_task(&mut alloc, TaskKind::Harvest { tile }, PRIO_HARVEST, None);
            board.get_mut(id).unwrap().status = TaskStatus::Running;
            id
        };
        *app.world_mut().resource_mut::<IdAllocator<TaskId>>() = alloc;

        // Reserve the target for the task
        {
            let mut uniq = app.world_mut().resource_mut::<UniqueTargetRes>();
            uniq.try_reserve(UniqueTargetKey::Tile(tile), task_id);
        }

        // Spawn pawn with Job(Harvest) and low hunger
        let pawn = Pawn { id: PawnId(2000) };
        let needs = Needs { hunger: 0.1, rest: 0.9 };
        let job = Job {
            task: task_id,
            driver: JobDriver::Harvest {
                target: tile,
                state: HarvestState::ReserveTarget,
                reserved: Some(UniqueTargetKey::Tile(tile)),
            },
        };
        app.world_mut().spawn((pawn, needs, job));

        app.update();

        // Verify job removed, task Pending, reservation released, EatAuto emitted, and a preempt log exists
        {
            let world = app.world_mut();
            let mut q = world.query::<&Job>();
            assert_eq!(q.iter(world).count(), 0);
            let board = world.resource::<TaskBoard>();
            let t = board.tasks.iter().find(|t| t.id == task_id).unwrap();
            assert_eq!(t.status, TaskStatus::Pending);
            let uniq = world.resource::<UniqueTargetRes>();
            assert!(!uniq.is_owner(UniqueTargetKey::Tile(tile), task_id));
            assert!(board.tasks.iter().any(|t| matches!(t.kind, TaskKind::EatAuto { pawn: p } if p == PawnId(2000))));
            let logs = world.resource::<LogBuffer>();
            assert!(matches!(logs.events.last(), Some(LogEvent::Preempt { .. })));
        }
    }

    #[test]
    fn job_tick_harvest_reserve_to_travel() {
        let mut app = App::new();
        app.init_resource::<TaskBoard>()
            .init_resource::<UniqueTargetRes>()
            .add_systems(Startup, job_tick);

        // Create task and pawn job at ReserveTarget
        let tile = TileId::new(2, 2);
        let task_id = TaskId(1000);
        app.world_mut().insert_resource(TaskBoard {
            tasks: vec![Task {
                id: task_id,
                kind: TaskKind::Harvest { tile },
                status: TaskStatus::Running,
                priority: PRIO_HARVEST,
                owner: None,
            }],
        });
        let pawn = Pawn { id: PawnId(3000) };
        let needs = Needs { hunger: 1.0, rest: 1.0 };
        let pos = TileId::new(0, 0);
        let job = Job {
            task: task_id,
            driver: JobDriver::Harvest {
                target: tile,
                state: HarvestState::ReserveTarget,
                reserved: None,
            },
        };
        app.world_mut().spawn((pawn, needs, pos, job));

        app.update();

        // Should move to TravelCountdown and reserve key
        {
            let world = app.world_mut();
            let mut q = world.query::<(&Job, &TileId)>();
            let v: Vec<_> = q.iter(world).collect();
            assert_eq!(v.len(), 1);
            let (job, pos) = v[0];
            match job.driver {
                JobDriver::Harvest { target, state, reserved } => {
                    assert_eq!(target, tile);
                    assert!(matches!(state, HarvestState::TravelCountdown { .. }));
                    assert_eq!(reserved, Some(UniqueTargetKey::Tile(tile)));
                    // Distance is >= 1
                    if let HarvestState::TravelCountdown { ticks_left } = state {
                        assert!(ticks_left >= 1);
                    }
                    // Position unchanged
                    assert_eq!(pos, &TileId::new(0, 0));
                }
                _ => panic!("unexpected driver"),
            }
        }
    }

    #[test]
    fn release_stale_reservations_cleans_missing_task() {
        let mut app = App::new();
        app.insert_resource(TaskBoard::default())
            .insert_resource({
                let mut res = UniqueTargetRes::default();
                res.owners.insert(UniqueTargetKey::Tile(TileId::new(1, 1)), TaskId(9999));
                res
            })
            .add_systems(Startup, release_stale_reservations);

        app.update();

        let uniq = app.world().resource::<UniqueTargetRes>();
        assert!(uniq.owners.is_empty());
    }
}
