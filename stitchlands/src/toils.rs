use std::collections::VecDeque;

use bevy::prelude::*;
use simkit_core::{
    fixed_point::Q40p24,
    grid::{index::TileMapIndex, TileId},
};

use crate::{
    model::*,
    tasks::{
        manhattan,
        nearest_fixture_with_item,
        nearest_item_on_ground,
        Job,
        JobKind,
        Task,
        TaskBoard,
        TaskSpec,
    },
};

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

pub enum ToilResult {
    Done,
    Failed(String),
    Running,
}

#[derive(Event)]
pub struct CompletedTask(pub TaskId);

pub fn step_jobs(
    mut commands: Commands,
    mut completed_tasks: EventWriter<CompletedTask>,
    mut pawns: PawnQueryMut<(&mut TileId, &mut Job)>,
    mut q: Query<&mut TileId>,
    mut items: ItemQueryMut<&mut ItemRelation>,
    mut fixtures: FixtureQueryMut<&TileId>,
    mut pawn_tile_map_index: ResMut<TileMapIndex<PawnId>>,
    mut item_tile_map_index: ResMut<TileMapIndex<ItemId>>,
    mut fixture_tile_index: ResMut<TileMapIndex<FixtureId>>,
) {
    for (mut pawn, (mut tile, mut job)) in pawns.query.iter_mut() {
        if job.current_toil.is_none() {
            let Some(toil) = job.plan.pop_front() else {
                // If the job is a task, complete it
                if let JobKind::Task(task_id) = job.kind {
                    completed_tasks.write(CompletedTask(task_id));
                }

                // Reset our job for scheduling the next job
                job.kind = JobKind::None;
                info!("No more toils to run for pawn {:?}", pawn.id);
                continue;
            };

            // Start the next toil
            job.current_toil = Some(toil);
        }

        let x = q.get_mut(fixtures.index.get(&FixtureId(0)));
        let y = items.get_mut(&ItemId(0));

        // Run the current toil
        let toil = job.current_toil.as_mut().unwrap();
        step_toil(
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
    }
}

pub fn step_toil<'a>(
    commands: &mut Commands,
    toil: &mut ToilKind,
    pawn: &mut Pawn,
    pawn_tile: &mut TileId,
    items: &mut ItemQueryMut<&mut ItemRelation>,
    fixtures: &mut FixtureQueryMut<&TileId>,
    pawn_tile_map_index: &mut ResMut<TileMapIndex<PawnId>>,
    item_tile_map_index: &mut ResMut<TileMapIndex<ItemId>>,
    fixture_tile_index: &mut ResMut<TileMapIndex<FixtureId>>,
) -> ToilResult {
    match toil {
        ToilKind::ReserveItem { item } => {
            // We will implement this later, noop for now
            ToilResult::Done
        }
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
        ToilKind::PickUp { item_id } => {
            let (item, mut item_relation) = items.get_mut(item_id);
            let item_joined = item.join_no_pawns(*item_relation, |id| {
                *fixtures.get(&id).1
            });

            if manhattan(*pawn_tile, item_joined.pos) > 1 {
                return ToilResult::Failed(format!(
                    "Invalid PickUp toil: item not adjacent pawn_pos: \
                     {pawn_tile:?} item_pos: {:?}",
                    item_joined.pos
                ));
            }

            match &item_joined.relation {
                ItemRelation::CarriedBy(pawn_id) => {
                    warn!(
                        "PickUp toil should not be planned if item already in \
                         inventory"
                    );
                    return ToilResult::Done;
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
        ToilKind::PutDown {
            item_id,
            target_tile,
        } => {
            assert!(pawn.inventory.contains(item_id), "Item not in inventory");
            let (item, item_relation) = items.get(item_id);

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
            let fixture_id = fixtures.index.alloc(None);
            let fixture_entity = commands
                .spawn((
                    Fixture {
                        id: fixture_id,
                        kind: FixtureKind::BerryBush,
                        inventory: Inventory::default(),
                        harvest_countdown: Some(100),
                    },
                    *target_tile_id,
                    Name::new(format!("BerryBush#{}", fixture_id.0)),
                ))
                .id();
            fixtures.index.insert(fixture_id, fixture_entity);
            fixture_tile_index.move_id(None, *target_tile_id, fixture_id);

            ToilResult::Done
        }
        ToilKind::Harvest { fixture_id } => {
            let (mut fixture, fixture_tile) = fixtures.get_mut(fixture_id);

            // Check preconditions
            assert!(
                manhattan(*pawn_tile, *fixture_tile) == 1,
                "Harvest toil must be adjacent to the fixture"
            );
            assert_eq!(fixture.kind, FixtureKind::BerryBush);
            assert_eq!(
                fixture.harvest_countdown,
                Some(0),
                "Fixture is not ready to harvest"
            );

            // Update fixture
            fixture.harvest_countdown = Some(100);

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

            ToilResult::Done
        }
        ToilKind::Sleep { fixture_id } => {
            let (fixture, fixture_tile) = fixtures.get(fixture_id);
            assert_eq!(fixture.kind, FixtureKind::SleepingPad);
            assert!(
                manhattan(*pawn_tile, *fixture_tile) <= 1,
                "Sleep toil must be adjacent to or on top of the sleeping \
                 fixture"
            );

            // Update pawn
            if pawn.sleep >= Q40p24::from(0.1) {
                pawn.sleep -= Q40p24::from(0.1);
                ToilResult::Running
            } else {
                pawn.sleep = Q40p24::ZERO;
                ToilResult::Done
            }
        }
    }
}

impl JobKind {
    pub fn build_plan(
        &self,
        pawn: &Pawn,
        pawn_tile: &TileId,
        tasks: &TaskBoard,
        items: &ItemQuery<&ItemRelation>,
        fixtures: &FixtureQuery<&TileId>,
        fixture_tile_index: &TileMapIndex<FixtureId>,
    ) -> Result<VecDeque<ToilKind>, String> {
        match self {
            JobKind::Sleep => build_sleep_plan(pawn_tile, fixtures),
            JobKind::Eat => build_eat_plan(pawn, pawn_tile, items, fixtures),
            JobKind::Task(task_id) => build_plan_for_task(
                tasks.tasks.get(task_id).unwrap(),
                pawn,
                pawn_tile,
                items,
                fixtures,
                fixture_tile_index,
            ),
            JobKind::None => Err("No plan for none job".to_string()),
        }
    }
}

pub fn build_plan_for_task(
    task: &Task,
    pawn: &Pawn,
    pawn_tile: &TileId,
    items: &ItemQuery<&ItemRelation>,
    fixtures: &FixtureQuery<&TileId>,
    fixture_tile_index: &TileMapIndex<FixtureId>,
) -> Result<VecDeque<ToilKind>, String> {
    match &task.spec {
        TaskSpec::Harvest(fixture_id) => {
            let (fixture, fixture_tile) = fixtures.get(fixture_id);

            // Check if fixture is ready to harvest
            if fixture.harvest_countdown.is_none()
                || fixture.harvest_countdown.unwrap() > 0
            {
                return Err(format!(
                    "Fixture {:?} is not ready to harvest",
                    fixture_id
                ));
            }

            Ok(VecDeque::from_iter([
                ToilKind::MoveTo {
                    target: *fixture_tile,
                    path: manhattan_path(*pawn_tile, *fixture_tile),
                },
                ToilKind::Harvest {
                    fixture_id: *fixture_id,
                },
            ]))
        }
        TaskSpec::Plant(target_tile_id, item_kind) => {
            // Check if tile is a plantable tile
            if let Some(fixture_id) = fixture_tile_index.get(*target_tile_id) {
                return Err(format!(
                    "Tile {:?} is not a plantable tile. Contains fixture {:?}",
                    target_tile_id, fixture_id
                ));
            }

            let (mut plan, seed) = build_acquire_item_plan(
                pawn, pawn_tile, item_kind, items, fixtures,
            )
            .ok_or_else(|| format!("Failed to acquire item {:?}", item_kind))?;

            let seed_pos = match items.get(&seed).1 {
                ItemRelation::CarriedBy(_) => *pawn_tile,
                ItemRelation::InFixture(fixture_id) => {
                    *fixtures.get(fixture_id).1
                }
                ItemRelation::OnGround(tile_id) => *tile_id,
            };

            plan.push_back(ToilKind::MoveTo {
                target: *target_tile_id,
                path: manhattan_path(seed_pos, *target_tile_id),
            });
            plan.push_back(ToilKind::Plant {
                seed_id: seed,
                tile_id: *target_tile_id,
            });
            Ok(plan)
        }
    }
}

pub fn build_sleep_plan(
    pawn_pos: &TileId,
    fixtures: &FixtureQuery<&TileId>,
) -> Result<VecDeque<ToilKind>, String> {
    let sleeping_pad = fixtures
        .query
        .iter()
        .filter(|(fixture, _)| fixture.kind == FixtureKind::SleepingPad)
        .min_by_key(|(_, pos)| manhattan(*pawn_pos, **pos));

    let Some((sleeping_pad, sleeping_pad_pos)) = sleeping_pad else {
        return Err(format!("No sleeping pad found for pawn {:?}", pawn_pos));
    };

    Ok(VecDeque::from_iter([
        ToilKind::MoveTo {
            target: *sleeping_pad_pos,
            path: manhattan_path(*pawn_pos, *sleeping_pad_pos),
        },
        ToilKind::Sleep {
            fixture_id: sleeping_pad.id,
        },
    ]))
}

pub fn build_eat_plan(
    pawn: &Pawn,
    pawn_tile: &TileId,
    items: &ItemQuery<&ItemRelation>,
    fixtures: &FixtureQuery<&TileId>,
) -> Result<VecDeque<ToilKind>, String> {
    let Some((mut plan, item_id)) = build_acquire_item_plan(
        pawn,
        pawn_tile,
        &ItemKind::Berry,
        items,
        fixtures,
    ) else {
        return Err(format!("Failed to acquire item {:?}", ItemKind::Berry));
    };
    plan.push_back(ToilKind::Consume { item_id });
    Ok(plan)
}

fn build_acquire_item_plan(
    pawn: &Pawn,
    pawn_pos: &TileId,
    item_kind: &ItemKind,
    items: &ItemQuery<&ItemRelation>,
    fixtures: &FixtureQuery<&TileId>,
) -> Option<(VecDeque<ToilKind>, ItemId)> {
    if let Some(item_id) = pawn.inventory.find(*item_kind) {
        return Some((VecDeque::new(), item_id));
    }

    let on_ground = nearest_item_on_ground(item_kind, pawn_pos, items);
    let fixture = nearest_fixture_with_item(item_kind, pawn_pos, fixtures);
    let (item_id, _dist) = closer_option_item_locator(on_ground, fixture)?;

    let item_pos = match items.get(&item_id).1 {
        ItemRelation::CarriedBy(pawn_id) => *pawn_pos,
        ItemRelation::InFixture(fixture_id) => *fixtures.get(fixture_id).1,
        ItemRelation::OnGround(tile_id) => *tile_id,
    };

    Some((
        VecDeque::from_iter([
            ToilKind::MoveTo {
                target: item_pos,
                path: manhattan_path(*pawn_pos, item_pos),
            },
            ToilKind::PickUp { item_id },
        ]),
        item_id,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemLocator {
    InInventory(ItemId, TileId),
    OnGround(ItemId, TileId, u32),
    InFixture(FixtureId, TileId, ItemId, u32),
}

impl ItemLocator {
    pub fn tile_id(&self) -> TileId {
        match self {
            ItemLocator::OnGround(_, tile_id, _) => *tile_id,
            ItemLocator::InFixture(_, tile_id, _, _) => *tile_id,
            ItemLocator::InInventory(_, tile_id) => *tile_id,
        }
    }

    pub fn item_id(&self) -> ItemId {
        match self {
            ItemLocator::OnGround(item_id, _, _) => *item_id,
            ItemLocator::InFixture(_, _, item_id, _) => *item_id,
            ItemLocator::InInventory(item_id, _) => *item_id,
        }
    }

    pub fn distance(&self) -> u32 {
        match self {
            ItemLocator::OnGround(_, _, d) => *d,
            ItemLocator::InFixture(_, _, _, d) => *d,
            ItemLocator::InInventory(_, _) => 0,
        }
    }

    pub fn closer(self, other: Self) -> Self {
        if self.distance() < other.distance() {
            self
        } else {
            other
        }
    }
}

pub fn closer_option_item_locator(
    a: Option<(ItemId, u32)>,
    b: Option<(ItemId, u32)>,
) -> Option<(ItemId, u32)> {
    match (a, b) {
        (Some(a), Some(b)) => {
            if a.1 < b.1 {
                Some(a)
            } else {
                Some(b)
            }
        }
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn manhattan_path(start: TileId, end: TileId) -> VecDeque<TileId> {
    let mut path = VecDeque::new();

    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let mut x = start.x;
    let mut y = start.y;
    while x != end.x {
        x += dx.signum();
        path.push_back(TileId::new(x, y))
    }
    while y != end.y {
        y += dy.signum();
        path.push_back(TileId::new(x, y));
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
