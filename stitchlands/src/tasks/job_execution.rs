use std::collections::VecDeque;

use bevy::prelude::*;
use simkit_core::{
    fixed_point::Q40p24,
    grid::{TileId, index::TileMapIndex},
    ids::SimId,
};

use crate::{
    model::*,
    tasks::{
        CompletedTask,
        Job,
        JobKind,
        TaskBoard,
        ToilEvent,
        ToilKind,
        ToilResult,
        manhattan,
    },
};

pub fn evaluate_job(
    mut params: ParamSet<(
        (
            &World,
            EventReader<ToilEvent>,
            ItemQuery<&ItemRelation>,
            FixtureQuery,
        ),
        (EventWriter<CompletedTask>, PawnQueryMut<&mut Job>),
    )>,
) {
    debug!("--- evaluate_job ---");
    let (
        world, //
        mut toil_events,
        items,
        fixtures,
    ) = params.p0();

    let mut job_mutations: Vec<(PawnId, Job)> = Vec::new();
    let task_board = world.resource::<TaskBoard>();

    for toil_event in toil_events.read() {
        debug!("Toil event: {:?}", toil_event);

        let job: &Job = world.comp(&toil_event.pawn_id);

        let mut replan = || {
            if let Ok(plan) = job.kind.build_plan_from_world(
                world,
                &toil_event.pawn_id,
                &items,
                &fixtures,
            ) {
                info!("Replanned job: {:?}", plan);
                job_mutations.push((
                    toil_event.pawn_id,
                    Job {
                        kind: job.kind,
                        plan: Some(plan),
                        retries: job.retries + 1,
                    },
                ));
                return;
            }
            job_mutations.push((toil_event.pawn_id, Job::default()));
            warn!("Failed to replan job: {:?}", job.kind);
        };

        // Falls through means task is done
        match &toil_event.failure_reason {
            None => {
                debug!("Toil done: {:?}", toil_event.toil);

                if !job.plan.as_ref().unwrap().toils.is_empty() {
                    continue;
                }

                // only replan if job is a non-complete task
                // else mark complete
                if let JobKind::Task(task_id, _) = job.kind {
                    let task_spec =
                        &task_board.tasks.get(&task_id).unwrap().spec;
                    if !task_spec.completed(world) {
                        replan();
                        continue;
                    }
                }
            }
            Some(failure_reason) => {
                debug!(
                    "Toil failed: {:?} reason: {failure_reason}",
                    toil_event.toil
                );
                if job.retries < 2 {
                    replan();
                    continue;
                }
            }
        }

        job_mutations.push((toil_event.pawn_id, Job::default()));
    }

    let (mut completed_tasks, mut pawns) = params.p1();

    for (pawn_id, new_job) in job_mutations {
        let (_, mut job) = pawns.get_mut(&pawn_id);

        if new_job.kind == JobKind::None
            && let JobKind::Task(task_id, _) = job.kind
        {
            completed_tasks.write(CompletedTask(task_id));
        }

        *job = new_job;
    }
}

pub fn step_jobs(
    mut commands: Commands,
    mut toil_events: EventWriter<ToilEvent>,
    mut pawns: PawnQueryMut<(&mut TileId, &mut Job)>,
    mut items: ItemQueryMut<&mut ItemRelation>,
    mut fixtures: FixtureQueryMut<(
        &TileId,
        Option<&mut Harvestable>,
        Option<&mut ConstructionSite>,
    )>,
    mut pawn_tile_map_index: ResMut<TileMapIndex<PawnId>>,
    mut item_tile_map_index: ResMut<TileMapIndex<ItemId>>,
    mut fixture_tile_index: ResMut<TileMapIndex<FixtureId>>,
) {
    debug!("--- step_jobs ---");
    for (mut pawn, (mut tile, mut job)) in pawns.query.iter_mut() {
        debug!("Job: {:?}, Pawn: {:?}, Tile: {:?}", job.kind, pawn.id, tile);

        if job.kind == JobKind::None {
            continue;
        }

        // Run the current toil
        let plan = job.plan.as_mut().unwrap();
        let toil = plan.toils.front_mut().expect("Job has no toil");
        let toil_result = step_toil(
            &mut commands,
            toil,
            &mut pawn,
            &mut tile,
            &mut items,
            &mut fixtures,
            &mut pawn_tile_map_index,
            &mut item_tile_map_index,
            &mut fixture_tile_index,
        );

        debug!("Toil result: {:?}", toil_result);
        if toil_result != ToilResult::Running {
            debug!("Toil done");
            let toil = plan.toils.pop_front().expect("Job has no toil");
            toil_events.write(ToilEvent {
                pawn_id: pawn.id,
                toil,
                failure_reason: toil_result.failure_reason(),
            });
        }
    }
}

pub fn step_toil(
    commands: &mut Commands,
    toil: &mut ToilKind,
    pawn: &mut Pawn,
    pawn_tile: &mut TileId,
    items: &mut ItemQueryMut<&mut ItemRelation>,
    fixtures: &mut FixtureQueryMut<(
        &TileId,
        Option<&mut Harvestable>,
        Option<&mut ConstructionSite>,
    )>,
    pawn_tile_map_index: &mut ResMut<TileMapIndex<PawnId>>,
    item_tile_map_index: &mut ResMut<TileMapIndex<ItemId>>,
    fixture_tile_index: &mut ResMut<TileMapIndex<FixtureId>>,
) -> ToilResult {
    debug!(?toil, "step_toil");
    match toil {
        ToilKind::MoveTo { target, path } => {
            if let Some(next_tile) = path.pop_front() {
                assert!(
                    manhattan(*pawn_tile, next_tile) == 1,
                    "MoveTo toil must move to a neighboring tile"
                );

                // TODO: check that the tile can be entered

                // update both the pawn's tile and the pawn's tile map
                // index
                pawn_tile_map_index.move_id(
                    Some(&mut *pawn_tile),
                    next_tile,
                    pawn.id,
                );
            }
            if !path.is_empty() {
                return ToilResult::Running;
            }

            // We've reached the target tile
            assert_eq!(
                *pawn_tile, *target,
                "MoveTo toil must end at the target tile"
            );
            info!(
                "Pawn {:?} has reached the target tile {:?}",
                pawn.id, target
            );
            ToilResult::Done
        }
        ToilKind::PickUpItem { item_id } => {
            let (item, mut item_relation) = items.get_mut(item_id);
            let item_pos = match item_relation.as_ref() {
                ItemRelation::CarriedBy(pawn_id) => {
                    if *pawn_id == pawn.id {
                        return ToilResult::Done;
                    }
                    return ToilResult::Failed(format!(
                        "Invalid PickUp toil: item already carried by pawn \
                         {:?}",
                        pawn_id
                    ));
                }
                ItemRelation::InFixture(fixture_id) => {
                    *fixtures.get(fixture_id).1.0
                }
                ItemRelation::OnGround(tile_id) => *tile_id,
            };

            if manhattan(*pawn_tile, item_pos) > 1 {
                return ToilResult::Failed(format!(
                    "Invalid PickUp toil: item not adjacent pawn_pos: \
                     {pawn_tile:?} item_pos: {:?}",
                    item_pos
                ));
            }

            match item_relation.as_ref() {
                ItemRelation::CarriedBy(_) => {
                    unreachable!();
                }
                ItemRelation::OnGround(item_tile) => {
                    item_tile_map_index.remove(*item_tile, *item_id);
                }
                ItemRelation::InFixture(fixture_id) => {
                    let (mut fixture, _) = fixtures.get_mut(fixture_id);
                    fixture.inventory.remove(item_id);
                }
            }

            // Update item relation
            *item_relation = ItemRelation::CarriedBy(pawn.id);
            pawn.inventory.add((*item_id, item.kind));
            ToilResult::Done
        }
        ToilKind::PutDownItem {
            item_id,
            target_tile,
        } => {
            assert!(pawn.inventory.contains(item_id), "Item not in inventory");

            pawn.inventory.remove(item_id);
            item_tile_map_index.move_id(None, *target_tile, *item_id);
            *items.get_mut(item_id).1 = ItemRelation::OnGround(*target_tile);

            ToilResult::Done
        }
        ToilKind::Plant {
            seed_id,
            tile_id: target_tile_id,
        } => {
            let (item, item_relation) = items.get(seed_id);
            // Check preconditions
            assert_eq!(item.kind, ItemKind::Berry);
            assert_eq!(*item_relation, ItemRelation::CarriedBy(pawn.id));
            assert!(pawn.inventory.contains(seed_id));
            if manhattan(*pawn_tile, *target_tile_id) > 1 {
                return ToilResult::Failed(format!(
                    "Invalid Plant toil: target not adjacent pawn_pos: \
                     {pawn_tile:?} target_pos: {:?}",
                    target_tile_id
                ));
            }

            // Update item components
            commands.entity(items.entity(seed_id)).despawn();
            items.index.remove(*seed_id);
            pawn.inventory.remove(seed_id);

            // Create new fixture
            Fixture::spawn(
                commands,
                &mut fixtures.index,
                &mut *fixture_tile_index,
                Fixture {
                    id: FixtureId::dummy(),
                    kind: FixtureKind::BerryBush,
                    inventory: Inventory::default(),
                },
                *target_tile_id,
                Harvestable {
                    countdown: 100,
                    seq_num: 0,
                },
            );

            ToilResult::Done
        }
        ToilKind::Harvest { fixture_id } => {
            let (mut fixture, (fixture_tile, harvest_countdown, _)) =
                fixtures.get_mut(fixture_id);

            // Check preconditions
            assert!(
                manhattan(*pawn_tile, *fixture_tile) == 1,
                "Harvest toil must be adjacent to the fixture. Pawn Pos: \
                 {:?}, Fixture Pos: {:?}",
                pawn_tile,
                fixture_tile
            );
            assert!(harvest_countdown.is_some());
            let mut harvest_countdown = harvest_countdown.unwrap();
            assert_eq!(fixture.kind, FixtureKind::BerryBush);
            assert_eq!(
                harvest_countdown.as_ref().countdown,
                0,
                "Fixture is not ready to harvest"
            );

            // Update fixture
            *harvest_countdown = Harvestable {
                countdown: 100,
                seq_num: harvest_countdown.seq_num + 1,
            };

            // Create new item
            let item_id = items.index.alloc(None);
            let item_entity = commands
                .spawn((
                    Item {
                        id: item_id,
                        kind: ItemKind::Berry,
                        qty: 1,
                    },
                    Name::new(format!("Berry#{}", item_id.0)),
                    ItemRelation::CarriedBy(pawn.id),
                ))
                .id();
            items.index.insert(item_id, item_entity);

            // Update inventory
            pawn.inventory.add((item_id, ItemKind::Berry));

            ToilResult::Done
        }
        ToilKind::Consume { item_id } => {
            assert!(pawn.inventory.contains(item_id), "Item not in inventory");
            let (item, item_relation) = items.get(item_id);
            assert_eq!(*item_relation, ItemRelation::CarriedBy(pawn.id));

            assert_eq!(item.kind, ItemKind::Berry);
            assert!(pawn.inventory.contains(item_id));
            pawn.inventory.remove(item_id);

            commands.entity(items.entity(item_id)).despawn();

            // Reduce hunger (increase hunger stat toward full) with a fixed
            // nutrition value
            let nutrition = Q40p24::from(60);
            pawn.hunger = if pawn.hunger + nutrition > Q40p24::from(100) {
                Q40p24::from(100)
            } else {
                pawn.hunger + nutrition
            };

            ToilResult::Done
        }
        ToilKind::Sleep { fixture_id } => {
            let (fixture, (fixture_tile, _, _)) = fixtures.get(fixture_id);
            assert_eq!(fixture.kind, FixtureKind::SleepingPad);
            assert!(
                manhattan(*pawn_tile, *fixture_tile) <= 1,
                "Sleep toil must be adjacent to or on top of the sleeping \
                 fixture"
            );

            // Update pawn
            if pawn.sleep < Q40p24::from(100) {
                pawn.sleep += Q40p24::from(10);
                ToilResult::Running
            } else {
                pawn.sleep = Q40p24::from(100);
                ToilResult::Done
            }
        }
        ToilKind::PlaceConstructionSite { building_spec } => {
            // Create new fixture
            Fixture::spawn(
                commands,
                &mut fixtures.index,
                &mut *fixture_tile_index,
                Fixture {
                    id: FixtureId::dummy(),
                    kind: FixtureKind::ConstructionSite,
                    inventory: Inventory::default(),
                },
                building_spec.top_left,
                ConstructionSite {
                    building_spec: building_spec.clone(),
                    work_left: building_spec.work_units,
                },
            );

            ToilResult::Done
        }
        ToilKind::Build { fixture_id } => {
            let (mut fixture, (_, _, construction_site)) =
                fixtures.get_mut(fixture_id);
            assert_eq!(fixture.kind, FixtureKind::ConstructionSite);
            assert_eq!(construction_site.is_some(), true);
            let mut construction_site = construction_site.unwrap();

            construction_site.work_left =
                construction_site.work_left.saturating_sub(1);
            if construction_site.work_left == 0 {
                // The fixture becomes the intended kind once built
                fixture.kind =
                    construction_site.building_spec.fixture_kind.clone();

                // Remove consumed construction materials from inventory
                for (item_kind, qty) in
                    &construction_site.building_spec.required_items
                {
                    for _ in 0..*qty {
                        let item_id =
                            fixture.inventory.find(*item_kind).unwrap();
                        fixture.inventory.remove(&item_id);
                    }
                }

                // TODO: spawn any other components that the built fixture
                // should have
                commands
                    .entity(fixtures.entity(fixture_id))
                    .remove::<ConstructionSite>();

                return ToilResult::Done;
            }

            ToilResult::Running
        }
        ToilKind::StoreItem {
            item_id,
            target_fixture_id,
        } => {
            assert!(pawn.inventory.contains(item_id), "Item not in inventory");
            let (item, mut item_relation) = items.get_mut(item_id);
            assert_eq!(*item_relation, ItemRelation::CarriedBy(pawn.id));
            pawn.inventory.remove(item_id);

            let (mut fixture, _) = fixtures.get_mut(target_fixture_id);
            fixture.inventory.add((*item_id, item.kind));
            *item_relation = ItemRelation::InFixture(*target_fixture_id);
            ToilResult::Done
        }
    }
}
