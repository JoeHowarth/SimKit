use std::{collections::VecDeque, fmt::format};

use bevy::prelude::*;
use simkit_core::grid::{index::TileMapIndex, TileId};

use crate::{
    model::{
        components::{
            CarriedBy, Fixture, FixtureKind, FixtureQuery, InFixture, ItemKind, ItemQuery, Pawn, PawnQuery, WorldExt
        },
        ids::*,
    },
    tasks::{
        item_in_inventory,
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
        item_loc: ItemLocator,
    },
    PutDown {
        // consider allowing ItemId or ItemKind here
        item: ItemId,
        target: TileId,
    },
    Plant {
        seed_id: ItemId,
        tile_id: TileId,
    },
    Consume {
        item: ItemId,
    },
    Sleep {
        fixture: FixtureId,
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
struct CompletedTask(TaskId);

pub fn step_jobs(
    mut pawns: Query<(&mut Pawn, &mut TileId, &mut Job)>,
    mut pawn_tile_map_index: ResMut<TileMapIndex<PawnId>>,
    mut items: ItemQuery<&mut TileId>,
    mut item_tile_map_index: ResMut<TileMapIndex<ItemId>>,
    mut completed_tasks: EventWriter<CompletedTask>,
) {
    for (pawn, mut tile, mut job) in pawns.iter_mut() {
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

        // Run the current toil
        let toil = job.current_toil.as_mut().unwrap();
        match toil {
            ToilKind::ReserveItem { item } => todo!(),
            ToilKind::MoveTo { target, path } => {
                if let Some(next_tile) = path.pop_front() {
                    assert!(
                        manhattan(*tile, next_tile) == 1,
                        "MoveTo toil must move to a neighboring tile"
                    );
                    // update both the pawn's tile and the pawn's tile map
                    // index
                    pawn_tile_map_index.move_id(
                        Some(&mut tile),
                        next_tile,
                        pawn.id,
                    );
                }
                if path.is_empty() {
                    // We've reached the target tile
                    assert_eq!(
                        *tile, *target,
                        "MoveTo toil must end at the target tile"
                    );
                    info!(
                        "Pawn {:?} has reached the target tile {:?}",
                        pawn.id, target
                    );
                    job.current_toil = None;
                }
            }
            ToilKind::PickUp { item: item_id } => {
                //
                let Some(item) = items.get(item_id) else {
                    return ToilResult::Failed(format!(
                        "Item {:?} not found",
                        item_id
                    ));
                };
            }
            ToilKind::PutDown { item, target } => todo!(),
            ToilKind::Plant { seed_id, tile_id } => todo!(),
            ToilKind::Consume { item } => todo!(),
            ToilKind::Sleep { fixture } => todo!(),
            ToilKind::Harvest { fixture_id } => todo!(),
        }
    }
}

pub fn step_toil(
    mut commands: Commands,
    pawn: &mut Pawn,
    tile: &mut TileId,
    toil: &mut ToilKind,
    mut pawn_tile_map_index: ResMut<TileMapIndex<PawnId>>,
    mut items: ItemQuery<(&mut TileId, Option<&mut CarriedBy>, Option<&mut InFixture>)>,
    mut item_tile_map_index: ResMut<TileMapIndex<ItemId>>,
    mut completed_tasks: EventWriter<CompletedTask>,
) -> ToilResult {
    match toil {
        ToilKind::ReserveItem { item } => {
            // We will implement this later, noop for now
            ToilResult::Done
        }
        ToilKind::MoveTo { target, path } => {
            if let Some(next_tile) = path.pop_front() {
                assert!(
                    manhattan(*tile, next_tile) == 1,
                    "MoveTo toil must move to a neighboring tile"
                );

                // TODO: check that the tile can be entered

                // update both the pawn's tile and the pawn's tile map
                // index
                pawn_tile_map_index.move_id(
                    Some(&mut tile),
                    next_tile,
                    pawn.id,
                );
            }
            if !path.is_empty() {
                return ToilResult::Running;
            }

            // We've reached the target tile
            assert_eq!(
                *tile, *target,
                "MoveTo toil must end at the target tile"
            );
            info!(
                "Pawn {:?} has reached the target tile {:?}",
                pawn.id, target
            );
            return ToilResult::Done;
        }
        ToilKind::PickUp { item_loc } => {
            //
            let item_id = item_loc.item_id();
            let (item, (_, carried_by, in_fixture)) = items.get(&item_id);

            match item_loc {
                ItemLocator::InInventory(item_id, tile_id) => {
                    warn!(
                        "PickUp toil should not be planned if item already in \
                         inventory"
                    );
                    assert_eq!(carried_by, Some(&CarriedBy(pawn.id)));
                    assert_eq!(pawn.inventory.)
                    return ToilResult::Done;
                }
                ItemLocator::OnGround(item_id, item_tile, _) => {
                    if manhattan(*tile, *item_tile) > 1 {
                        return ToilResult::Failed(format!(
                            "Invalid PickUp toil: item not adjacent pawn_pos: \
                             {tile:?} item_pos: {item_tile:?}",
                        ));
                    }
                    item_tile_map_index.
                }
                ItemLocator::InFixture(fixture_id, tile_id, item_id, _) => {
                    todo!()
                }
            }

            // check if item is on ground

            // check if item is in a fixture
            todo!()
        }
        ToilKind::PutDown { item, target } => todo!(),
        ToilKind::Plant { seed_id, tile_id } => todo!(),
        ToilKind::Consume { item } => todo!(),
        ToilKind::Sleep { fixture } => todo!(),
        ToilKind::Harvest { fixture_id } => todo!(),
    }
}

impl JobKind {
    pub fn build_plan(
        &self,
        pawn: &Pawn,
        pawn_tile: &TileId,
        tasks: &TaskBoard,
        items: &ItemQuery<&TileId>,
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
    items: &ItemQuery<&TileId>,
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
                pawn, pawn_tile, &item_kind, items, fixtures,
            )
            .ok_or_else(|| format!("Failed to acquire item {:?}", item_kind))?;

            plan.push_back(ToilKind::MoveTo {
                target: *target_tile_id,
                path: manhattan_path(seed.tile_id(), *target_tile_id),
            });
            plan.push_back(ToilKind::Plant {
                seed_id: seed.item_id(),
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
            fixture: sleeping_pad.id,
        },
    ]))
}

pub fn build_eat_plan(
    pawn: &Pawn,
    pawn_tile: &TileId,
    items: &ItemQuery<&TileId>,
    fixtures: &FixtureQuery<&TileId>,
) -> Result<VecDeque<ToilKind>, String> {
    let Some((mut plan, item_locator)) = build_acquire_item_plan(
        pawn,
        pawn_tile,
        &ItemKind::Berry,
        items,
        fixtures,
    ) else {
        return Err(format!("Failed to acquire item {:?}", ItemKind::Berry));
    };
    plan.push_back(ToilKind::Consume {
        item: item_locator.item_id(),
    });
    Ok(plan)
}

fn build_acquire_item_plan(
    pawn: &Pawn,
    pawn_pos: &TileId,
    item_kind: &ItemKind,
    items: &ItemQuery<&TileId>,
    fixtures: &FixtureQuery<&TileId>,
) -> Option<(VecDeque<ToilKind>, ItemLocator)> {
    if let Some(loc) = item_in_inventory(pawn_pos, item_kind, &pawn.inventory) {
        // We already have the item!
        return Some((VecDeque::new(), loc));
    }

    let on_ground = nearest_item_on_ground(item_kind, pawn_pos, items);
    let fixture = nearest_fixture_with_item(item_kind, pawn_pos, fixtures);
    let closer = closer_option_item_locator(on_ground, fixture)?;
    Some((
        VecDeque::from_iter([
            ToilKind::MoveTo {
                target: closer.tile_id(),
                path: manhattan_path(*pawn_pos, closer.tile_id()),
            },
            ToilKind::PickUp { item_loc: closer },
        ]),
        closer,
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
    a: Option<ItemLocator>,
    b: Option<ItemLocator>,
) -> Option<ItemLocator> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.closer(b)),
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
    path.push_back(end);
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
