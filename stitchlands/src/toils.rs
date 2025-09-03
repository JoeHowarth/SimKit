use std::collections::VecDeque;

use bevy::prelude::*;
use simkit_core::grid::{index::TileMapIndex, TileId};

use crate::{
    model::{
        components::{
            Fixture,
            FixtureQuery,
            ItemKind,
            ItemQuery,
            Pawn,
            WorldExt,
        },
        ids::*,
    },
    tasks::{
        item_in_inventory,
        nearest_fixture_with_item,
        nearest_item_on_ground,
        Task,
        TaskSpec,
    },
};

pub enum ToilKind {
    ReserveItem {
        item: ItemId,
    },
    MoveTo {
        target: TileId,
        path: Option<VecDeque<TileId>>,
    },
    PickUp {
        item: ItemId,
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

fn build_plan_for_task(
    task: &Task,
    pawn: &Pawn,
    world: &World,
) -> Result<VecDeque<ToilKind>, String> {
    let (pawn, pawn_entity, pawn_tile) =
        world.get_comp_simid::<TileId, _>(&pawn.id);

    match task.spec {
        TaskSpec::Harvest(fixture_id) => {
            let (fixture, fixture_entity, fixture_tile) =
                world.get_comp_simid::<TileId, _>(&fixture_id);

            // check if fixture is ready to harvest
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
                    path: Some(manhattan_path(*pawn_tile, *fixture_tile)),
                },
                ToilKind::Harvest { fixture_id },
            ]))
        }
        TaskSpec::Plant(tile_id, item_kind) => {
            // Plant(TileId, ItemKind)
            // •	If item already in pawn inventory:
            // •	Plan: [MoveTo{tile}, PutDown{item, tile}].
            // •	Else choose source using your neartest_item_position:
            // •	If ground source: [MoveTo{item_pos}, PickUp{item},
            // MoveTo{tile}, PutDown{item, tile}].
            // •	If fixture
            // source: [MoveTo{fixture_pos}, PickUp{item}, MoveTo{tile},
            // PutDown{item, tile}].
            // •	If none: return empty to
            // force requeue.

            // Check if tile is a plantable tile
            let fixture_tile_index =
                world.resource::<TileMapIndex<FixtureId>>();

            if let Some(fixture_id) = fixture_tile_index.get(tile_id) {
                return Err(format!(
                    "Tile {:?} is not a plantable tile. Contains fixture {:?}",
                    tile_id, fixture_id
                ));
            }

            // Check if item is in pawn inventory
            if let Some(seed_id) =
                item_in_inventory(&item_kind, &pawn.inventory)
            {
                return Ok(VecDeque::from_iter([
                    ToilKind::MoveTo {
                        target: tile_id,
                        path: Some(manhattan_path(*pawn_tile, tile_id)),
                    },
                    ToilKind::Plant { seed_id, tile_id },
                ]));
            }

            vec![ToilKind::Plant {
                fixture_id,
                item_kind,
            }]
        }
    }
}

fn build_acquire_item_plan(
    pawn: &Pawn,
    pawn_pos: &TileId,
    item_kind: &ItemKind,
    items: &ItemQuery<&TileId>,
    fixtures: &FixtureQuery<&TileId>,
) -> Option<(VecDeque<ToilKind>, ItemLocator)> {
    if let Some(item_id) = item_in_inventory(item_kind, &pawn.inventory) {
        // We already have the item!
        return Some((
            VecDeque::new(),
            ItemLocator::InInventory(item_id, *pawn_pos),
        ));
    }

    let on_ground = nearest_item_on_ground(item_kind, pawn_pos, items);
    let fixture = nearest_fixture_with_item(item_kind, pawn_pos, fixtures);
    let closer = closer_option_item_locator(on_ground, fixture)?;
    Some(VecDeque::from_iter([
        ToilKind::MoveTo {
            target: closer.tile_id(),
            path: Some(manhattan_path(*pawn_pos, closer.tile_id())),
        },
        ToilKind::PickUp {
            item: closer.item_id(),
        },
    ]))
}

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
