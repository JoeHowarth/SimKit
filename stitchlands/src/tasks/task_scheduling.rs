use std::collections::VecDeque;

use bevy::prelude::*;
use simkit_core::{
    fixed_point::Q40p24,
    grid::{TileId, index::TileMapIndex},
    ids::SimId,
};

use super::*;

pub(super) fn schedule_pawns(
    mut pawns: Query<(&Pawn, &TileId, &WorkPriority, &mut Job)>,
    mut task_board: ResMut<TaskBoard>,
    fixtures: FixtureQuery,
    items: ItemQuery<&ItemRelation>,
    fixture_tile_index: Res<TileMapIndex<FixtureId>>,
    reservations: Res<Reservations>,
) {
    trace!("Begin schedule_pawns");
    // TODO: use a stable ordering of pawns
    let mut pawns = pawns.iter_mut().collect::<Vec<_>>();
    pawns.sort_by_key(|(pawn, _, _, _)| pawn.id.to_u64());

    for (pawn, pos, work_priority, mut job) in pawns {
        // If job is running, check if it should be preempted
        if job.kind != JobKind::None {
            if let Some(preempt) = should_preempt(pawn, job.kind) {
                debug!(
                    "Considering preemption: current={:?}, new={:?}",
                    job.kind, preempt
                );

                let plan = Plan::new(&reservations);
                // Build the preempt plan first; only switch if planning
                // succeeds
                match preempt.build_plan(
                    plan,
                    pawn,
                    pos,
                    &task_board,
                    &items,
                    &fixtures,
                    &fixture_tile_index,
                ) {
                    Ok(plan) => {
                        // Return the current task to pending after we know the
                        // new plan is viable
                        if let JobKind::Task(task_id, _) = &job.kind {
                            task_board.return_to_pending(*task_id);
                        }

                        *job = Job::new(preempt, plan);
                        info!("Preempted job: {:?}", job);
                    }
                    Err(_) => {
                        warn!(
                            "Preemption planning failed; keeping current job: \
                             {:?}",
                            job.kind
                        );
                    }
                }
            }
            continue;
        }

        debug!("Choosing next job for Pawn: {:?}, Tile: {:?}", pawn.id, pos);
        // If no job is running, choose a new job
        let next_job = choose_next_job(
            &task_board,
            pawn,
            pos,
            work_priority,
            &fixtures,
            &items,
            &fixture_tile_index,
            &reservations,
        );

        // If the job is a task, set the task to assigned
        if let JobKind::Task(task_id, _) = &next_job.kind {
            task_board.set_assigned(*task_id, pawn.id);
        }

        // Set the job to the new job
        *job = next_job;
    }
}

fn should_preempt(pawn: &Pawn, current_job: JobKind) -> Option<JobKind> {
    if current_job == JobKind::Eat {
        if pawn.sleep < Q40p24::from(60) {
            return Some(JobKind::Sleep);
        }
        return None;
    }

    // Preemption is in a stable order, so it will not thrash
    if pawn.hunger < Q40p24::from(80) {
        return Some(JobKind::Eat);
    }

    if current_job == JobKind::Sleep {
        return None;
    }

    if pawn.sleep < Q40p24::from(80) {
        return Some(JobKind::Sleep);
    }

    None
}

fn next_job_is_needs(
    pawn: &Pawn,
    pos: &TileId,
    fixtures: &FixtureQuery,
    items: &ItemQuery<&ItemRelation>,
    reservations: &Reservations,
) -> Option<Job> {
    // Sleep and eat threshold are lower than when we have a job
    if pawn.hunger < Q40p24::from(60) {
        let plan = Plan::new(reservations);
        match build_eat_plan(plan, pawn, pos, items, fixtures) {
            Ok(plan) => {
                return Some(Job::new(JobKind::Eat, plan));
            }
            Err(e) => {
                warn!("Eat auto job failed to plan: {e}");
            }
        }
    }

    if pawn.sleep < Q40p24::from(60) {
        let plan = Plan::new(reservations);
        match build_sleep_plan(plan, pos, fixtures) {
            Ok(plan) => {
                return Some(Job::new(JobKind::Sleep, plan));
            }
            Err(e) => {
                warn!("Sleep auto job failed to plan: {e}");
            }
        }
    }

    None
}

fn choose_next_job(
    pending: &TaskBoard,
    pawn: &Pawn,
    pos: &TileId,
    work_priority: &WorkPriority,
    fixtures: &FixtureQuery,
    items: &ItemQuery<&ItemRelation>,
    fixture_tile_index: &TileMapIndex<FixtureId>,
    reservations: &Reservations,
) -> Job {
    // Check if needs are urgent
    if let Some(job) =
        next_job_is_needs(pawn, pos, fixtures, items, reservations)
    {
        info!("Next job is needs: {:?}", job);
        return job;
    }

    debug!("Next job is not needs");

    for kind in work_priority.0.iter() {
        debug!("Next job is looking for kind: {:?}", kind);

        // Find highest priority task for this kind
        let mut tasks = pending
            .pending_tasks_by_kind(kind)
            .filter_map(|task| {
                let priority = task.spec.priority(
                    pawn,
                    pos,
                    fixtures,
                    items,
                    reservations,
                )?;
                Some((priority, task))
            })
            .collect::<Vec<_>>();
        tasks.sort_by_key(|(priority, task)| {
            (*priority, -(task.id.to_u64() as i64))
        });

        while let Some((_, task)) = tasks.pop() {
            let plan = Plan::new(reservations);
            match build_plan_for_task(
                plan,
                task,
                pawn,
                pos,
                items,
                fixtures,
                fixture_tile_index,
            ) {
                Ok(plan) => {
                    let kind = JobKind::Task(task.id, task.spec.kind());
                    info!(
                        "Built plan for task {:?} ({:?}): {:?}",
                        task.spec, task.id, plan
                    );
                    return Job::new(kind, plan);
                }
                Err(e) => {
                    info!(
                        "Failed to build plan for task {:?}={:?}: {}",
                        kind, task.id, e
                    )
                }
            }
        }
    }

    info!("No job found for pawn {:?}", pawn.id);
    Job::default()
}

impl TaskSpec {
    fn priority(
        &self,
        pawn: &Pawn,
        pos: &TileId,
        fixtures: &FixtureQuery,
        items: &ItemQuery<&ItemRelation>,
        reservations: &Reservations,
    ) -> Option<Q40p24> {
        match self {
            TaskSpec::Harvest { .. } => self.harvest_priority(pos, fixtures),
            TaskSpec::Plant(_, _) => {
                self.plant_priority(pawn, pos, items, fixtures, reservations)
            }
            TaskSpec::Build(..) => {
                self.build_priority(pawn, pos, fixtures, items)
            }
        }
    }

    fn build_priority(
        &self,
        pawn: &Pawn,
        pos: &TileId,
        fixtures: &FixtureQuery,
        items: &ItemQuery<&ItemRelation>,
    ) -> Option<Q40p24> {
        let TaskSpec::Build(build_spec) = self else {
            panic!("Build priority called for non-build task");
        };

        let distance = manhattan(*pos, build_spec.top_left);

        Some(distance_to_score(distance))
    }

    fn harvest_priority(
        &self,
        pos: &TileId,
        fixtures: &FixtureQuery,
    ) -> Option<Q40p24> {
        let TaskSpec::Harvest { to_harvest, .. } = self else {
            panic!("Harvest priority called for non-harvest task");
        };

        let (_, (fixture_pos, harvestable, _)) = fixtures.get(to_harvest);
        if harvestable.is_none() || harvestable.unwrap().countdown > 0 {
            return None;
        }

        let distance = manhattan(*pos, *fixture_pos);
        Some(distance_to_score(distance))
    }

    fn plant_priority(
        &self,
        pawn: &Pawn,
        pawn_pos: &TileId,
        items: &ItemQuery<&ItemRelation>,
        fixtures: &FixtureQuery,
        reservations: &Reservations,
    ) -> Option<Q40p24> {
        let TaskSpec::Plant(fixture_pos, item_kind) = self else {
            panic!("Plant priority called for non-plant task");
        };
        let item_pos = nearest_item_pos(
            pawn,
            pawn_pos,
            item_kind,
            items,
            fixtures,
            reservations,
        )?;
        let distance_to_get_item = manhattan(*pawn_pos, item_pos);

        let distance = manhattan(*fixture_pos, item_pos);
        let distance_score = distance_to_score(distance + distance_to_get_item);
        Some(distance_score)
    }
}

fn distance_to_score(distance: impl Into<Q40p24>) -> Q40p24 {
    let distance = distance.into();
    Q40p24::ONE / (Q40p24::ONE + distance)
}
