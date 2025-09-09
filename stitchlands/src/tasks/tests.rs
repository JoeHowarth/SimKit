
use super::*;
use crate::{
    model::queries::WorldExt,
    scenario::{
        model::{FixtureDef, MapDef, MapSize, PawnDef, ScenarioDef},
        testutil::app_with_scenario,
    },
    tasks::{job_planning::manhattan_path, TaskPlugin, TaskSpecKind, TaskStatus},
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

#[test]
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
            sleep: Some(1),
            hunger: Some(1),
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
    app.add_plugins(TaskPlugin { schedule: Update });
    let task_id = app
        .world_mut()
        .resource_mut::<TaskBoard>()
        .add_task(TaskSpec::Harvest(FixtureId(2)));

    // Initial state before any updates
    let pawn_id = PawnId(1);
    let (_, e) = app.world().get_simid(&pawn_id);
    let mut q = app.world_mut().query::<(&TileId, &Pawn, &Job)>();
    {
        let (tile, pawn, job) = q.get(app.world(), e).unwrap();

        assert_eq!(job.kind, JobKind::None);
        assert!(job.current_toil.is_none());
        assert!(job.plan.is_empty());

        // Pawn starts at (1, 0) from the schematic
        assert_eq!(*tile, TileId::new(1, 0));
        assert!(pawn.inventory.of_kind(ItemKind::Berry).next().is_none());
    }

    // First update: scheduler assigns Harvest task and starts MoveTo
    app.update();
    {
        let (tile, _, job) = q.get(app.world(), e).unwrap();

        // Job assigned and plan created
        assert_eq!(job.kind, JobKind::Task(task_id, TaskSpecKind::Harvest));

        // Current toil should be MoveTo towards tile (2,2)
        match &job.current_toil {
            Some(ToilKind::MoveTo { target, path }) => {
                assert_eq!(*target, TileId::new(2, 2));
                // After one step, two steps remain: (2,1), (2,2)
                assert_eq!(path.len(), 2);
            }
            other => panic!("Expected MoveTo toil, got: {:?}", other),
        }

        // Only Harvest remains in the plan
        assert_eq!(job.plan.len(), 1);
        match job.plan.front().unwrap() {
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

        match &job.current_toil {
            Some(ToilKind::MoveTo { target, path }) => {
                assert_eq!(*target, TileId::new(2, 2));
                assert_eq!(path.len(), 1);
            }
            other => panic!("Expected MoveTo toil, got: {:?}", other),
        }
        // Plan still has Harvest
        assert_eq!(job.plan.len(), 1);
        assert_eq!(*tile, TileId::new(2, 1));
    }

    // Third update: finish MoveTo, next toil pending (Harvest at F2)
    app.update();
    {
        let (tile, _, job) = q.get(app.world(), e).unwrap();

        assert!(job.current_toil.is_none());
        assert_eq!(*tile, TileId::new(2, 2));
        // Harvest is still queued
        assert_eq!(job.plan.len(), 1);
        matches!(job.plan.front().unwrap(), ToilKind::Harvest { .. });
    }

    // Fourth update: perform Harvest; inventory gains a Berry; plan clears
    app.update();
    {
        let world = app.world();
        let (_, pawn, job) = q.get(world, e).unwrap();

        // Harvest finished this tick; job is cleared in the same tick
        assert!(matches!(job.kind, JobKind::None));
        assert!(job.current_toil.is_none());
        assert!(job.plan.is_empty());

        // Pawn now has a Berry in inventory
        assert!(pawn.inventory.of_kind(ItemKind::Berry).next().is_some());

        // Fixture harvest cooldown reset
        let (fixture, _e) = world.get_simid(&FixtureId(2));
        assert_eq!(fixture.harvest_countdown, Some(100));
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
            sleep: Some(1),
            hunger: Some(1),
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
    app.add_plugins(TaskPlugin { schedule: Update });
    let harvest_task_id = app
        .world_mut()
        .resource_mut::<TaskBoard>()
        .add_task(TaskSpec::Harvest(FixtureId(2)));
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
        assert!(job.current_toil.is_none());
        assert!(job.plan.is_empty());
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
        match &job.current_toil {
            Some(ToilKind::MoveTo { .. }) => {}
            other => {
                panic!("expected MoveTo as first toil, got: {:?}", other)
            }
        }
        match job.plan.front() {
            Some(ToilKind::Harvest { .. }) => {}
            other => {
                panic!("expected Harvest action queued, got: {:?}", other)
            }
        }
    }

    let mut reached_harvest_target = false;
    for _ in 0..3 {
        app.update();
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        if job.current_toil.is_none() {
            assert!(matches!(job.plan.front(), Some(ToilKind::Harvest { .. })));
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
        assert!(job.plan.is_empty());
        assert!(matches!(job.kind, JobKind::None));
        assert!(pawn.inventory.of_kind(ItemKind::Berry).next().is_some());

        let (fixture, _fe) = app.world().get_simid(&FixtureId(2));
        assert_eq!(fixture.harvest_countdown, Some(100));
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
        match &job.current_toil {
            Some(ToilKind::MoveTo { .. }) => {}
            other => {
                panic!("expected MoveTo before Plant, got: {:?}", other)
            }
        }
        match job.plan.front() {
            Some(ToilKind::Plant { .. }) => {}
            other => {
                panic!("expected Plant action queued, got: {:?}", other)
            }
        }
    }

    let mut reached_plant_target = false;
    for _ in 0..3 {
        app.update();
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        if job.current_toil.is_none() {
            assert!(matches!(job.plan.front(), Some(ToilKind::Plant { .. })));
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
        assert!(job.plan.is_empty());
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
        assert!(job.current_toil.is_none());
        assert!(job.plan.is_empty());
    }
}

pub fn fixture_cd(app: &App, fid: FixtureId) -> Option<u32> {
    app.world().get_simid(&fid).0.harvest_countdown
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
            j.current_toil.is_none()
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
                    .  .  .  .  F3  .  .  .
                    .  .  .  .  .  .  .  .
                    .  .  F2 .  .  .  F1  .
                    .  .  .  .  F4  .  .  .
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
                sleep: Some(1),
                hunger: Some(1),
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
    app.add_plugins(TaskPlugin { schedule: Update });
    let mut task_board = app.world_mut().resource_mut::<TaskBoard>();
    let task_ids = [
        task_board.add_task(TaskSpec::Harvest(FixtureId(1))),
        task_board.add_task(TaskSpec::Harvest(FixtureId(2))),
        task_board.add_task(TaskSpec::Harvest(FixtureId(3))),
        task_board.add_task(TaskSpec::Harvest(FixtureId(4))),
        task_board
            .add_task(TaskSpec::Plant(TileId::new(0, 1), ItemKind::Berry)),
        task_board
            .add_task(TaskSpec::Plant(TileId::new(3, 3), ItemKind::Berry)),
    ];

    // Helper functions (high-level, reduce boilerplate)
    use bevy::prelude::QueryState;
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
        match (&job.current_toil, job.plan.front()) {
            (
                Some(ToilKind::MoveTo { .. }),
                Some(ToilKind::Harvest { fixture_id }),
            ) => {
                // Should pick a ready bush and not F3 (countdown=2)
                assert!(
                    [FixtureId(1), FixtureId(2), FixtureId(4)]
                        .contains(fixture_id),
                    "first Harvest should be one of F1/F2/F4, got {:?}",
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
        match job.plan.front() {
            Some(ToilKind::Harvest { fixture_id }) => *fixture_id,
            _ => panic!("expected Harvest next"),
        }
    };
    app.update();
    {
        let (_tile, pawn, job) = q.get(app.world(), e).unwrap();
        assert!(matches!(job.kind, JobKind::None));
        assert!(pawn.inventory.of_kind(ItemKind::Berry).next().is_some());
        assert_eq!(fixture_cd(&app, last_harvest), Some(100));
    }

    // 2) Next assignment becomes Plant (now feasible). Should choose (3,3)
    //    first
    app.update();
    let first_plant_target = {
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        match &job.kind {
            JobKind::Task(_, TaskSpecKind::Plant) => {}
            _ => panic!("expected Plant job"),
        };
        match (&job.current_toil, job.plan.front()) {
            (
                Some(ToilKind::MoveTo { .. }),
                Some(ToilKind::Plant {
                    seed_id: _,
                    tile_id,
                }),
            ) => {
                assert_eq!(
                    *tile_id,
                    TileId::new(3, 3),
                    "expected nearer Plant at (3,3)"
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

    // 3) Next should be Harvest again (Plant infeasible). Prefer F4 next
    app.update();
    {
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        match &job.kind {
            JobKind::Task(_, TaskSpecKind::Harvest) => {}
            _ => panic!("expected Harvest after Plant"),
        };
        match (&job.current_toil, job.plan.front()) {
            (
                Some(ToilKind::MoveTo { .. }),
                Some(ToilKind::Harvest { fixture_id }),
            ) => {
                assert_eq!(
                    *fixture_id,
                    FixtureId(4),
                    "expected F4 as next nearest Harvest"
                );
            }
            other => panic!("expected MoveTo then Harvest, got {other:?}"),
        }
    }
    finish_moveto(&mut app, &mut q, e, 16);
    let last_harvest = {
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        match job.plan.front() {
            Some(ToilKind::Harvest { fixture_id }) => *fixture_id,
            _ => panic!("expected Harvest next"),
        }
    };
    app.update();
    assert_eq!(fixture_cd(&app, last_harvest), Some(100));

    // 4) Plant the second target at (0,1)
    app.update();
    let second_plant_target = {
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        match &job.kind {
            JobKind::Task(_, TaskSpecKind::Plant) => {}
            _ => panic!("expected Plant job"),
        };
        match (&job.current_toil, job.plan.front()) {
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
        match (&job.current_toil, job.plan.front()) {
            (
                Some(ToilKind::MoveTo { .. }),
                Some(ToilKind::Harvest { fixture_id }),
            ) => {
                assert_eq!(
                    *fixture_id,
                    FixtureId(1),
                    "expected F1 as remaining ready Harvest"
                );
            }
            other => panic!("expected MoveTo then Harvest, got {other:?}"),
        }
    }
    finish_moveto(&mut app, &mut q, e, 32);
    let last_harvest = {
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        match job.plan.front() {
            Some(ToilKind::Harvest { fixture_id }) => *fixture_id,
            _ => panic!("expected Harvest next"),
        }
    };
    app.update();
    assert_eq!(fixture_cd(&app, last_harvest), Some(100));

    // 6) Eventually F3 should become ready and be harvested.
    // Note: Current behavior lacks countdown ticking; this will fail until
    // it's implemented. step_until(&mut app, |app| fixture_cd(app,
    // FixtureId(3)) == Some(100), 33);

    // End: pawn idle
    {
        let (_tile, _pawn, job) = q.get(app.world(), e).unwrap();
        assert!(matches!(job.kind, JobKind::None));
        assert!(job.current_toil.is_none());
        assert!(job.plan.is_empty());
    }

    let task_board = app.world().resource::<TaskBoard>();
    // info!("task_board: {:#?}", task_board);
    assert_eq!(task_board.tasks.len(), 6);
    assert_eq!(task_board.tasks_by_status(TaskStatus::Pending).count(), 1);
    assert_eq!(task_board.tasks_by_status(TaskStatus::Done).count(), 5);
    assert_eq!(
        task_board
            .tasks_by_status(TaskStatus::Assigned(PawnId(1)))
            .count(),
        0
    );
    assert_eq!(
        task_board.tasks.get(&task_ids[0]).unwrap().spec,
        TaskSpec::Harvest(FixtureId(1))
    );
    assert_eq!(
        task_board.tasks.get(&task_ids[1]).unwrap().spec,
        TaskSpec::Harvest(FixtureId(2))
    );
    assert_eq!(
        task_board.tasks.get(&task_ids[2]).unwrap().status,
        TaskStatus::Pending
    );
    assert_eq!(
        *task_board.tasks.get(&task_ids[3]).unwrap(),
        Task {
            id: task_ids[3],
            spec: TaskSpec::Harvest(FixtureId(4)),
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
                sleep: Some(1),
                hunger: Some(1),
                ..Default::default()
            },
            PawnDef {
                id: Some(2),
                name: Some("Sam".into()),
                priorities: vec![TaskSpecKind::Plant, TaskSpecKind::Harvest],
                sleep: Some(1),
                hunger: Some(1),
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
    app.add_plugins(TaskPlugin { schedule: Update });
    let mut task_board = app.world_mut().resource_mut::<TaskBoard>();
    let task_ids = [
        task_board.add_task(TaskSpec::Harvest(FixtureId(1))),
        task_board.add_task(TaskSpec::Harvest(FixtureId(2))),
        task_board.add_task(TaskSpec::Harvest(FixtureId(3))),
        task_board.add_task(TaskSpec::Harvest(FixtureId(4))),
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
    // info!("task_board: {:#?}", task_board);
    assert_eq!(task_board.tasks.len(), 6);
    assert_eq!(task_board.tasks_by_status(TaskStatus::Pending).count(), 1);
    assert_eq!(task_board.tasks_by_status(TaskStatus::Done).count(), 5);
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
        TaskStatus::Pending
    );

    let pawn = app.world().get_simid(&PawnId(1)).0;
    assert_eq!(pawn.inventory.of_kind(ItemKind::Berry).count(), 0);
    let pawn = app.world().get_simid(&PawnId(2)).0;
    assert_eq!(pawn.inventory.of_kind(ItemKind::Berry).count(), 1);
}
