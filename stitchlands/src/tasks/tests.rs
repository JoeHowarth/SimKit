use std::collections::VecDeque;

use simkit_core::fixed_point::Q40p24;

use super::*;
use crate::{
    model::queries::WorldExt,
    scenario::{
        model::{FixtureDef, ItemDef, MapDef, MapSize, PawnDef, ScenarioDef},
        testutil::app_with_scenario,
    },
    tasks::{TaskSpecKind, TaskStatus, job_planning::manhattan_path},
};

#[test]
fn test_manhattan_path() {
    let start = TileId::new(0, 0);
    let end = TileId::new(3, 4);
    let path = manhattan_path(start, end);
    assert_eq!(
        path,
        VecDeque::from_iter([
            TileId::new(1, 0),
            TileId::new(2, 0),
            TileId::new(3, 0),
            TileId::new(3, 1),
            TileId::new(3, 2),
            TileId::new(3, 3),
            TileId::new(3, 4)
        ])
    );
}

#[test_log::test]
fn test_build_plan_for_task() {
    let def = ScenarioDef {
        map: MapDef {
            size: MapSize { x: 4, y: 4 },
            tiles: vec![],
            schematic: Some(
                "
                    .  P1 .  .
                    .  .  .  .
                    .  .  .  .
                    .  .  F2 .
                "
                .to_owned(),
            ),
        },
        sim_seed: Some(1),
        pawns: vec![PawnDef {
            id: Some(1),
            name: Some("Sam".into()),
            priorities: vec![TaskSpecKind::Harvest, TaskSpecKind::Plant],
            sleep: Some(100),
            hunger: Some(100),
            ..Default::default()
        }],
        fixtures: vec![FixtureDef {
            id: Some(2),
            kind: "BerryBush".into(),
            harvest_countdown: Some(0),
            ..Default::default()
        }],
        tasks: vec![],
    };

    let mut app = app_with_scenario(def);
    let task_id = app.world_mut().resource_mut::<TaskBoard>().add_task(
        TaskSpec::Harvest {
            to_harvest: FixtureId(2),
            target_seq_num: 1,
        },
    );

    // Initial state before any updates
    let pawn_id = PawnId(1);
    let (_, e) = app.world().get_simid(&pawn_id);
    let mut q = app.world_mut().query::<(&TileId, &Pawn, &Job)>();
    {
        let (tile, pawn, job) = q.get(app.world(), e).unwrap();

        assert_eq!(job.kind, JobKind::None);
        assert_eq!(job.plan, None);

        // Pawn starts at (1, 0) from the schematic
        assert_eq!(*tile, TileId::new(1, 0));
        assert!(pawn.inventory.of_kind(ItemKind::Berry).next().is_none());
    }

    // First update: scheduler assigns Harvest task and starts MoveTo
    app.update();
    {
        let (tile, _, job) = q.get(app.world(), e).unwrap();
        let plan = job.plan.as_ref().unwrap();

        // Job assigned and plan created
        assert_eq!(job.kind, JobKind::Task(task_id, TaskSpecKind::Harvest));

        // Current toil should be MoveTo towards tile (2,2)
        match plan.toils.front() {
            Some(ToilKind::MoveTo { target, path }) => {
                assert_eq!(*target, TileId::new(2, 2));
                // After one step, two steps remain: (2,1), (2,2)
                assert_eq!(path.len(), 2);
            }
            other => panic!("Expected MoveTo toil, got: {:?}", other),
        }

        // Only Harvest remains in the plan
        assert_eq!(plan.toils.len(), 2);
        match plan.toils.get(1).unwrap() {
            ToilKind::Harvest { fixture_id } => {
                assert_eq!(*fixture_id, FixtureId(2));
            }
            other => panic!("Expected Harvest in plan, got: {:?}", other),
        }

        // Pawn moved one step along x to (2,0)
        assert_eq!(*tile, TileId::new(2, 0));
    }

    // Second update: continue MoveTo, one step remains
    app.update();
    {
        let (tile, _, job) = q.get(app.world(), e).unwrap();
        let plan = job.plan.as_ref().unwrap();

        match plan.toils.front() {
            Some(ToilKind::MoveTo { target, path }) => {
                assert_eq!(*target, TileId::new(2, 2));
                assert_eq!(path.len(), 1);
            }
            other => panic!("Expected MoveTo toil, got: {:?}", other),
        }
        // Plan still has Harvest
        assert_eq!(plan.toils.len(), 2);
        assert_eq!(*tile, TileId::new(2, 1));
    }

    // Third update: finish MoveTo, next toil pending (Harvest at F2)
    app.update();
    {
        let (tile, _, job) = q.get(app.world(), e).unwrap();
        let plan = job.plan.as_ref().unwrap();

        assert_eq!(*tile, TileId::new(2, 2));
        // Harvest is still queued
        assert_eq!(plan.toils.len(), 1);
        matches!(plan.toils.front().unwrap(), ToilKind::Harvest { .. });
    }

    // Fourth update: perform Harvest; inventory gains a Berry; plan clears
    app.update();
    {
        let world = app.world();
        let (_, pawn, job) = q.get(world, e).unwrap();

        // Harvest finished this tick; job is cleared in the same tick
        assert!(matches!(job.kind, JobKind::None));
        assert!(job_plan_is_empty(job));

        // Pawn now has a Berry in inventory
        assert!(pawn.inventory.of_kind(ItemKind::Berry).next().is_some());

        // Fixture harvest cooldown reset
        let harvest_countdown: &Harvestable = world.comp(&FixtureId(2));
        assert_eq!(harvest_countdown.seq_num, 1);
        assert!(harvest_countdown.countdown > 90);
    }
}

#[test_log::test]
fn test_harvest_then_plant() {
    let def = ScenarioDef {
        map: MapDef {
            size: MapSize { x: 4, y: 4 },
            tiles: vec![],
            schematic: Some(
                "
                    .  P1 .  .
                    .  .  .  .
                    .  .  .  .
                    .  .  F2 .
                "
                .to_owned(),
            ),
        },
        sim_seed: Some(1),
        pawns: vec![PawnDef {
            id: Some(1),
            name: Some("Sam".into()),
            priorities: vec![TaskSpecKind::Plant, TaskSpecKind::Harvest],
            ..Default::default()
        }],
        fixtures: vec![FixtureDef {
            id: Some(2),
            kind: "BerryBush".into(),
            harvest_countdown: Some(0),
            ..Default::default()
        }],
        tasks: vec![],
    };

    let mut app = app_with_scenario(def);
    let harvest_task_id = app.world_mut().resource_mut::<TaskBoard>().add_task(
        TaskSpec::Harvest {
            to_harvest: FixtureId(2),
            target_seq_num: 1,
        },
    );
    let plant_task_id = app
        .world_mut()
        .resource_mut::<TaskBoard>()
        .add_task(TaskSpec::Plant(TileId::new(0, 1), ItemKind::Berry));

    let pawn_id = PawnId(1);
    let (_, e) = app.world().get_simid(&pawn_id);
    let mut q = app.world_mut().query::<(&TileId, &Pawn, &Job)>();
    {
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        assert!(matches!(job.kind, JobKind::None));
        assert!(job_plan_is_empty(job));
    }

    app.update();
    {
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        match job.kind {
            JobKind::Task(id, TaskSpecKind::Harvest) => {
                assert_eq!(id, harvest_task_id);
            }
            other => {
                panic!("expected first task Harvest, got: {:?}", other)
            }
        }
        let plan = job_plan_toils(job).unwrap();
        match (plan.front(), plan.get(1)) {
            (Some(ToilKind::MoveTo { .. }), Some(ToilKind::Harvest { .. })) => {
            }
            other => {
                panic!("expected MoveTo then Harvest plan, got: {:?}", other)
            }
        }
    }

    let mut reached_harvest_target = false;
    for _ in 0..3 {
        app.update();
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        if matches!(job_plan_front(job), Some(ToilKind::Harvest { .. })) {
            reached_harvest_target = true;
            break;
        }
    }
    assert!(
        reached_harvest_target,
        "MoveTo for Harvest should finish within 3 updates"
    );

    app.update();
    {
        let (_tile, pawn, job) = q.get(app.world(), e).unwrap();
        assert!(job_plan_is_empty(job));
        assert!(matches!(job.kind, JobKind::None));
        assert!(pawn.inventory.of_kind(ItemKind::Berry).next().is_some());

        let harvest: &Harvestable = app.world().comp(&FixtureId(2));
        assert_eq!(harvest.seq_num, 1);
    }

    app.update();
    {
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        match job.kind {
            JobKind::Task(id, TaskSpecKind::Plant) => {
                assert_eq!(id, plant_task_id);
            }
            other => panic!("expected next task Plant, got: {:?}", other),
        }
        let plan = job_plan_toils(job).unwrap();
        match (plan.front(), plan.get(1)) {
            (Some(ToilKind::MoveTo { .. }), Some(ToilKind::Plant { .. })) => {}
            other => {
                panic!("expected MoveTo then Plant plan, got: {:?}", other)
            }
        }
    }

    let mut reached_plant_target = false;
    for _ in 0..3 {
        app.update();
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        if matches!(job_plan_front(job), Some(ToilKind::Plant { .. })) {
            reached_plant_target = true;
            break;
        }
    }
    assert!(
        reached_plant_target,
        "MoveTo for Plant should finish within 3 updates"
    );

    app.update();
    {
        let (_tile, pawn, job) = q.get(app.world(), e).unwrap();
        assert!(job_plan_is_empty(job));
        assert!(matches!(job.kind, JobKind::None));
        assert!(pawn.inventory.of_kind(ItemKind::Berry).next().is_none());

        let fixture_index = app.world().resource::<TileMapIndex<FixtureId>>();
        assert!(fixture_index.get(TileId::new(0, 1)).is_some());
    }

    app.update();
    app.update();
    {
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        assert!(matches!(job.kind, JobKind::None));
        assert!(job_plan_is_empty(job));
    }
}

fn job_plan_toils(job: &Job) -> Option<&VecDeque<ToilKind>> {
    job.plan.as_ref().map(|plan| &plan.toils)
}

fn job_plan_front(job: &Job) -> Option<&ToilKind> {
    job_plan_toils(job).and_then(|toils| toils.front())
}

fn job_plan_is_empty(job: &Job) -> bool {
    job_plan_toils(job).map_or(true, |toils| toils.is_empty())
}

pub fn fixture_cd(app: &App, fid: FixtureId) -> Option<Harvestable> {
    let e = app.world().get_simid(&fid).1;
    app.world().entity(e).get::<Harvestable>().copied()
}
pub fn has_fixture_at(app: &App, pos: TileId) -> bool {
    app.world()
        .resource::<TileMapIndex<FixtureId>>()
        .get(pos)
        .is_some()
}

pub fn step_until<F: FnMut(&mut App) -> bool>(
    app: &mut App,
    mut pred: F,
    max: usize,
) {
    for _ in 0..max {
        if pred(app) {
            return;
        }
        app.update();
    }
    assert!(pred(app), "step_until timed out after {max} ticks");
}
pub fn finish_moveto(
    app: &mut App,
    q: &mut QueryState<(&TileId, &Pawn, &Job)>,
    e: Entity,
    max: usize,
) {
    step_until(
        app,
        |app| {
            let (_t, _p, j) = q.get(app.world(), e).unwrap();
            job_plan_front(j)
                .map(|t| !matches!(t, ToilKind::MoveTo { .. }))
                .unwrap_or(false)
        },
        max,
    );
}

#[test_log::test]
fn test_multi_harvest_and_plant() {
    let def = ScenarioDef {
        map: MapDef {
            size: MapSize { x: 8, y: 8 },
            tiles: vec![],
            schematic: Some(
                "
                    .  P1 .  .  .  .  .  .
                    .  .  .  .  .  .  . .
                    .  .  .  .  .  .  .  .
                    .  .  .  .  .  F3 .  .
                    .  .  .  .  .  .  .  .
                    .  .  F2 .  .  .  F1 .
                    .  .  .  .  F4 .  .  .
                    .  .  .  .  .  .  .  .
                "
                .to_owned(),
            ),
        },
        sim_seed: Some(1),
        pawns: vec![
            PawnDef {
                id: Some(1),
                name: Some("Veronica".into()),
                priorities: vec![TaskSpecKind::Plant, TaskSpecKind::Harvest],
                ..Default::default()
            },
            // PawnDef {
            //     id: Some(2),
            //     name: Some("Sam".into()),
            //     priorities: vec![
            //         TaskSpecKind::Plant,
            //         TaskSpecKind::Harvest,
            //     ],
            //     sleep: Some(1),
            //     hunger: Some(1),
            //     ..Default::default()
            // },
        ],
        fixtures: vec![
            FixtureDef {
                id: Some(1),
                kind: "BerryBush".into(),
                harvest_countdown: Some(0),
                ..Default::default()
            },
            FixtureDef {
                id: Some(2),
                kind: "BerryBush".into(),
                harvest_countdown: Some(0),
                ..Default::default()
            },
            FixtureDef {
                id: Some(3),
                kind: "BerryBush".into(),
                harvest_countdown: Some(2),
                ..Default::default()
            },
            FixtureDef {
                id: Some(4),
                kind: "BerryBush".into(),
                harvest_countdown: Some(0),
                ..Default::default()
            },
        ],
        tasks: vec![],
    };

    let mut app = app_with_scenario(def);
    let mut task_board = app.world_mut().resource_mut::<TaskBoard>();
    let task_ids = [
        task_board.add_task(TaskSpec::Harvest {
            to_harvest: FixtureId(1),
            target_seq_num: 1,
        }),
        task_board.add_task(TaskSpec::Harvest {
            to_harvest: FixtureId(2),
            target_seq_num: 1,
        }),
        task_board.add_task(TaskSpec::Harvest {
            to_harvest: FixtureId(3),
            target_seq_num: 1,
        }),
        task_board.add_task(TaskSpec::Harvest {
            to_harvest: FixtureId(4),
            target_seq_num: 1,
        }),
        task_board
            .add_task(TaskSpec::Plant(TileId::new(0, 1), ItemKind::Berry)),
        task_board
            .add_task(TaskSpec::Plant(TileId::new(3, 1), ItemKind::Berry)),
    ];

    // Helper functions (high-level, reduce boilerplate)

    let pawn_id = PawnId(1);
    let (_, e) = app.world().get_simid(&pawn_id);
    let mut q = app.world_mut().query::<(&TileId, &Pawn, &Job)>();

    // 1) First assignment must be Harvest (Plant infeasible initially)
    app.update();
    {
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        match &job.kind {
            JobKind::Task(_, TaskSpecKind::Harvest) => {}
            other => panic!("expected first job Harvest, got {other:?}"),
        }
        let plan = job_plan_toils(job).unwrap();
        match (plan.front(), plan.get(1)) {
            (
                Some(ToilKind::MoveTo { .. }),
                Some(ToilKind::Harvest { fixture_id }),
            ) => {
                // Should pick a ready bush and not F3 (countdown=2)
                assert!(
                    FixtureId(2) == *fixture_id,
                    "first Harvest should be one of F2, got {:?}",
                    fixture_id
                );
                // Prefer nearest → F2 from (1,0)
                assert_eq!(*fixture_id, FixtureId(2));
            }
            other => {
                panic!("expected MoveTo then Harvest plan, got {other:?}")
            }
        }
    }

    // Finish MoveTo and then perform Harvest
    finish_moveto(&mut app, &mut q, e, 16);
    // Perform the Harvest action
    let last_harvest = {
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        let plan = job_plan_toils(job).unwrap();
        match plan.front() {
            Some(ToilKind::Harvest { fixture_id }) => *fixture_id,
            x => panic!("expected Harvest next, got {x:?}"),
        }
    };
    app.update();
    {
        let (_tile, pawn, job) = q.get(app.world(), e).unwrap();
        assert!(matches!(job.kind, JobKind::None));
        assert!(pawn.inventory.of_kind(ItemKind::Berry).next().is_some());
        let harvest: &Harvestable = app.world().comp(&last_harvest);
        assert_eq!(harvest.seq_num, 1);
        assert!(harvest.countdown > 90);
    }

    // 2) Next assignment becomes Plant (now feasible). Should choose (3,1)
    //    first
    app.update();
    let first_plant_target = {
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        match &job.kind {
            JobKind::Task(_, TaskSpecKind::Plant) => {}
            _ => panic!("expected Plant job"),
        };
        let plan = job_plan_toils(job).unwrap();
        match (plan.front(), plan.get(1)) {
            (
                Some(ToilKind::MoveTo { .. }),
                Some(ToilKind::Plant {
                    seed_id: _,
                    tile_id,
                }),
            ) => {
                assert_eq!(
                    *tile_id,
                    TileId::new(3, 1),
                    "expected nearer Plant at (3,1)"
                );
                *tile_id
            }
            other => panic!("expected MoveTo then Plant, got {other:?}"),
        }
    };
    finish_moveto(&mut app, &mut q, e, 16);
    app.update(); // perform Plant
    {
        let (_tile, pawn, job) = q.get(app.world(), e).unwrap();
        assert!(matches!(job.kind, JobKind::None));
        assert!(pawn.inventory.of_kind(ItemKind::Berry).next().is_none());
        assert!(has_fixture_at(&app, first_plant_target));
    }

    // 3) Next should be Harvest again (Plant infeasible). Prefer F3 next
    app.update();
    {
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        match &job.kind {
            JobKind::Task(_, TaskSpecKind::Harvest) => {}
            _ => panic!("expected Harvest after Plant"),
        };
        let plan = job_plan_toils(job).unwrap();
        match (plan.front(), plan.get(1)) {
            (
                Some(ToilKind::MoveTo { .. }),
                Some(ToilKind::Harvest { fixture_id }),
            ) => {
                assert_eq!(
                    *fixture_id,
                    FixtureId(3),
                    "expected F3 as next nearest Harvest"
                );
            }
            other => panic!("expected MoveTo then Harvest, got {other:?}"),
        }
    }
    finish_moveto(&mut app, &mut q, e, 16);
    let last_harvest = {
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        let plan = job_plan_toils(job).unwrap();
        match plan.front() {
            Some(ToilKind::Harvest { fixture_id }) => *fixture_id,
            _ => panic!("expected Harvest next"),
        }
    };
    app.update();
    let harvest: &Harvestable = app.world().comp(&last_harvest);
    assert_eq!(harvest.seq_num, 1);
    assert!(harvest.countdown > 90);

    // 4) Plant the second target at (0,1)
    app.update();
    let second_plant_target = {
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        match &job.kind {
            JobKind::Task(_, TaskSpecKind::Plant) => {}
            _ => panic!("expected Plant job"),
        };
        let plan = job_plan_toils(job).unwrap();
        match (plan.front(), plan.get(1)) {
            (
                Some(ToilKind::MoveTo { .. }),
                Some(ToilKind::Plant { tile_id, .. }),
            ) => {
                assert_eq!(*tile_id, TileId::new(0, 1));
                *tile_id
            }
            other => panic!("expected MoveTo then Plant, got {other:?}"),
        }
    };
    finish_moveto(&mut app, &mut q, e, 32);
    app.update(); // perform Plant
    assert!(has_fixture_at(&app, second_plant_target));

    // 5) Remaining ready Harvest should be F1; F3 must wait until ready
    app.update();
    {
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        match &job.kind {
            JobKind::Task(_, TaskSpecKind::Harvest) => {}
            _ => panic!("expected Harvest job"),
        };
        let plan = job_plan_toils(job).unwrap();
        match (plan.front(), plan.get(1)) {
            (
                Some(ToilKind::MoveTo { .. }),
                Some(ToilKind::Harvest { fixture_id }),
            ) => {
                assert_eq!(
                    *fixture_id,
                    FixtureId(4),
                    "expected F4 as remaining ready Harvest"
                );
            }
            other => panic!("expected MoveTo then Harvest, got {other:?}"),
        }
    }
    finish_moveto(&mut app, &mut q, e, 32);
    let last_harvest = {
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        let plan = job_plan_toils(job).unwrap();
        match plan.front() {
            Some(ToilKind::Harvest { fixture_id }) => *fixture_id,
            _ => panic!("expected Harvest next"),
        }
    };
    app.update();
    assert_eq!(fixture_cd(&app, last_harvest).map(|h| h.seq_num), Some(1));

    // 6) Eventually F1 should become ready and be harvested.
    // Note: Current behavior lacks countdown ticking; this will fail until
    // it's implemented.
    step_until(
        &mut app,
        |app| {
            fixture_cd(app, FixtureId(1))
                == Some(Harvestable {
                    countdown: 100,
                    seq_num: 1,
                })
        },
        33,
    );

    // End: pawn idle
    {
        let (_tile, pawn, job) = q.get(app.world(), e).unwrap();
        assert!(matches!(job.kind, JobKind::None));
        assert!(job_plan_is_empty(job));
        assert_eq!(pawn.inventory.of_kind(ItemKind::Berry).count(), 1);
        assert!(pawn.hunger > Q40p24::from(95), "hunger: {:?}", pawn.hunger);
        assert!(pawn.sleep > Q40p24::from(60), "sleep: {:?}", pawn.sleep);
    }

    let task_board = app.world().resource::<TaskBoard>();
    // info!("task_board: {:#?}", task_board);
    assert_eq!(task_board.tasks.len(), 6);
    assert_eq!(task_board.tasks_by_status(TaskStatus::Pending).count(), 0);
    assert_eq!(task_board.tasks_by_status(TaskStatus::Done).count(), 6);
    assert_eq!(
        task_board
            .tasks_by_status(TaskStatus::Assigned(PawnId(1)))
            .count(),
        0
    );
    assert_eq!(
        task_board.tasks.get(&task_ids[0]).unwrap().spec,
        TaskSpec::Harvest {
            to_harvest: FixtureId(1),
            target_seq_num: 1,
        }
    );
    assert_eq!(
        task_board.tasks.get(&task_ids[1]).unwrap().spec,
        TaskSpec::Harvest {
            to_harvest: FixtureId(2),
            target_seq_num: 1,
        }
    );
    assert_eq!(
        task_board.tasks.get(&task_ids[2]).unwrap().status,
        TaskStatus::Done
    );
    assert_eq!(
        *task_board.tasks.get(&task_ids[3]).unwrap(),
        Task {
            id: task_ids[3],
            spec: TaskSpec::Harvest {
                to_harvest: FixtureId(4),
                target_seq_num: 1,
            },
            status: TaskStatus::Done,
        }
    );
}

#[test_log::test]
fn test_two_pawns() {
    let def = ScenarioDef {
        map: MapDef {
            size: MapSize { x: 8, y: 8 },
            tiles: vec![],
            schematic: Some(
                "
                    .  P1 .  .  .  .  .  .
                    .  .  .  .  .  .  . .
                    .  .  .  .  .  .  .  .
                    .  .  .  .  F3  .  .  .
                    .  .  .  .  .  .  .  .
                    .  .  F2 .  .  .  F1  .
                    .  .  .  .  F4  .  .  .
                    .  .  .  .  .  .  P2 .
                "
                .to_owned(),
            ),
        },
        sim_seed: Some(1),
        pawns: vec![
            PawnDef {
                id: Some(1),
                name: Some("Veronica".into()),
                priorities: vec![TaskSpecKind::Plant, TaskSpecKind::Harvest],
                ..Default::default()
            },
            PawnDef {
                id: Some(2),
                name: Some("Sam".into()),
                priorities: vec![TaskSpecKind::Plant, TaskSpecKind::Harvest],
                ..Default::default()
            },
        ],
        fixtures: vec![
            FixtureDef {
                id: Some(1),
                kind: "BerryBush".into(),
                harvest_countdown: Some(0),
                ..Default::default()
            },
            FixtureDef {
                id: Some(2),
                kind: "BerryBush".into(),
                harvest_countdown: Some(0),
                ..Default::default()
            },
            FixtureDef {
                id: Some(3),
                kind: "BerryBush".into(),
                harvest_countdown: Some(2),
                ..Default::default()
            },
            FixtureDef {
                id: Some(4),
                kind: "BerryBush".into(),
                harvest_countdown: Some(0),
                ..Default::default()
            },
        ],
        tasks: vec![],
    };

    let mut app = app_with_scenario(def);
    let mut task_board = app.world_mut().resource_mut::<TaskBoard>();
    let task_ids = [
        task_board.add_task(TaskSpec::Harvest {
            to_harvest: FixtureId(1),
            target_seq_num: 1,
        }),
        task_board.add_task(TaskSpec::Harvest {
            to_harvest: FixtureId(2),
            target_seq_num: 1,
        }),
        task_board.add_task(TaskSpec::Harvest {
            to_harvest: FixtureId(3),
            target_seq_num: 1,
        }),
        task_board.add_task(TaskSpec::Harvest {
            to_harvest: FixtureId(4),
            target_seq_num: 1,
        }),
        task_board
            .add_task(TaskSpec::Plant(TileId::new(0, 1), ItemKind::Berry)),
        task_board
            .add_task(TaskSpec::Plant(TileId::new(3, 3), ItemKind::Berry)),
    ];
    for i in 0..20 {
        warn!("Update {i}");
        app.update();
    }

    let task_board = app.world().resource::<TaskBoard>();
    info!("task_board: {:#?}", task_board);
    assert_eq!(task_board.tasks.len(), 6);
    assert_eq!(task_board.tasks_by_status(TaskStatus::Pending).count(), 0);
    assert_eq!(task_board.tasks_by_status(TaskStatus::Done).count(), 6);
    assert_eq!(
        task_board
            .tasks_by_status(TaskStatus::Assigned(PawnId(2)))
            .count(),
        0
    );
    assert_eq!(
        task_board
            .tasks_by_status(TaskStatus::Assigned(PawnId(1)))
            .count(),
        0
    );
    assert_eq!(
        task_board.tasks.get(&task_ids[2]).unwrap().status,
        TaskStatus::Done
    );

    let pawn = app.world().get_simid(&PawnId(1)).0;
    assert_eq!(pawn.inventory.of_kind(ItemKind::Berry).count(), 0);
    let pawn = app.world().get_simid(&PawnId(2)).0;
    assert_eq!(pawn.inventory.of_kind(ItemKind::Berry).count(), 2);
}

#[test_log::test]
fn test_construction() {
    let def = ScenarioDef {
        map: MapDef {
            size: MapSize { x: 5, y: 5 },
            tiles: vec![],
            schematic: Some(
                "
                    .  P1 .  .  .  
                    .  .  .  .  .  
                    .  .  .  .  .  
                    .  .  .  .  .  
                    .  .  .  .  .  
                "
                .to_owned(),
            ),
        },
        sim_seed: Some(1),
        pawns: vec![PawnDef {
            id: Some(1),
            name: Some("Veronica".into()),
            inventory: vec![
                ItemDef {
                    id: Some(1),
                    kind: ItemKind::Wood,
                    qty: 1,
                    pos: Some(TileId::new(0, 0)),
                },
                ItemDef {
                    id: Some(2),
                    kind: ItemKind::Wood,
                    qty: 1,
                    pos: Some(TileId::new(0, 0)),
                },
                ItemDef {
                    id: Some(3),
                    kind: ItemKind::Wood,
                    qty: 1,
                    pos: Some(TileId::new(0, 0)),
                },
            ],
            ..Default::default()
        }],
        ..default()
    };

    let mut app = app_with_scenario(def);
    let mut task_board = app.world_mut().resource_mut::<TaskBoard>();
    let top_left = TileId::new(2, 2);
    let task_id = task_board.add_task(TaskSpec::Build(BuildingSpec {
        top_left,
        dim: UVec2::new(1, 1),
        fixture_kind: FixtureKind::Cabin,
        required_items: vec![(ItemKind::Wood, 3)],
        work_units: 3,
    }));
    for i in 0..20 {
        warn!("Update {i}");
        app.update();
    }

    let task_board = app.world().resource::<TaskBoard>();
    debug!("task_board: {:#?}", task_board);
    assert_eq!(task_board.tasks.len(), 1);
    assert_eq!(task_board.tasks_by_status(TaskStatus::Pending).count(), 0);
    assert_eq!(task_board.tasks_by_status(TaskStatus::Done).count(), 1);
    assert_eq!(
        task_board
            .tasks_by_status(TaskStatus::Assigned(PawnId(1)))
            .count(),
        0
    );
    assert_eq!(
        task_board.tasks.get(&task_id).unwrap().status,
        TaskStatus::Done
    );

    let fixture_tile_index = app.world().resource::<TileMapIndex<FixtureId>>();
    let fixture_id = fixture_tile_index.get(top_left).unwrap();
    let world = app.world();
    let (fixture, e) = world.get_simid(&fixture_id);
    assert_eq!(fixture.kind, FixtureKind::Cabin);
    assert_eq!(
        world.entity(e).get::<ConstructionSite>(),
        None,
        "ConstructionSite should be removed"
    );
}
