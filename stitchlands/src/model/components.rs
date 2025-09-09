use std::str::FromStr;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use simkit_core::{fixed_point::Q40p24, grid::TileId, impl_hassimid};

use crate::{
    model::{
        ids::{FixtureId, ItemId, PawnId},
        FixtureQuery,
        PawnQuery,
    },
    tasks::Job,
    WorldTag,
};

/// Pawns
/// Required components: WorldTag, TileId
#[derive(Component, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[require(WorldTag, Job)]
pub struct Pawn {
    pub id: PawnId,
    pub inventory: Inventory,
    pub sleep: Q40p24,
    pub hunger: Q40p24,
    // health
}

#[derive(
    Component, Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default,
)]
pub struct Inventory(pub Vec<(ItemId, ItemKind)>);

impl Inventory {
    pub fn add(&mut self, item: (ItemId, ItemKind)) {
        self.0.push(item);
    }
    pub fn remove(&mut self, item_id: &ItemId) {
        self.0.retain(|(id, _)| *id != *item_id);
    }
    pub fn contains(&self, item_id: &ItemId) -> bool {
        self.0.iter().any(|(id, _)| *id == *item_id)
    }
    pub fn find(&self, item_kind: ItemKind) -> Option<ItemId> {
        self.of_kind(item_kind).next()
    }
    pub fn of_kind<'a>(
        &'a self,
        item_kind: ItemKind,
    ) -> impl Iterator<Item = ItemId> + 'a {
        self.0.iter().filter_map(move |(id, kind)| {
            if *kind == item_kind {
                Some(*id)
            } else {
                None
            }
        })
    }
}

/// Items
/// Required components: WorldTag
/// Item is either:
/// - On the ground: has TileId + reverse TileMapIndex lookup
/// - On a pawn: has CarriedBy(PawnId), invariant: pawn must include the item in
///   their inventory
/// - In a fixture: has InFixture(FixtureId), invariant: fixture must include
///   the item in its inventory
#[derive(Component, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[require(WorldTag)]
pub struct Item {
    pub id: ItemId,
    pub kind: ItemKind,
    pub qty: u32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum ItemKind {
    Berry,
    // Untyped(&'static str),
}

impl ItemKind {
    #[allow(dead_code)]
    fn has_nutrition(&self) -> Option<Q40p24> {
        match self {
            ItemKind::Berry => Some(Q40p24::ONE),
            // ItemKind::Untyped(_) => None,
        }
    }

    #[allow(dead_code)]
    fn plantable_fixture(&self) -> Option<FixtureKind> {
        match self {
            ItemKind::Berry => Some(FixtureKind::BerryBush),
            // ItemKind::Untyped(_) => None,
        }
    }
}

impl FromStr for ItemKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (s, _reset) = s.split_once(':').unwrap_or((s, ""));
        match s {
            "Berry" => Ok(Self::Berry),
            // "Untyped" => Ok(Self::Untyped(reset.to_string())),
            _ => Err(format!("Invalid ItemKind: {s}")),
        }
    }
}

#[derive(
    Component, Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize,
)]
pub enum ItemRelation {
    CarriedBy(PawnId),
    InFixture(FixtureId),
    OnGround(TileId),
}

impl Item {
    pub fn join_no_pawns(
        &self,
        relation: ItemRelation,
        fixture_pos: impl Fn(FixtureId) -> TileId,
    ) -> ItemJoined {
        let pos = match relation {
            ItemRelation::InFixture(fixture_id) => fixture_pos(fixture_id),
            ItemRelation::OnGround(tile_id) => tile_id,
            ItemRelation::CarriedBy(_) => panic!("Item is carried by a pawn"),
        };
        ItemJoined {
            relation,
            pos,
            item: self.clone(),
        }
    }

    pub fn join(
        &self,
        relation: ItemRelation,
        fixtures: &FixtureQuery<&TileId>,
        pawns: &PawnQuery<&TileId>,
    ) -> ItemJoined {
        let pos = match relation {
            ItemRelation::CarriedBy(pawn_id) => *pawns.get(&pawn_id).1,
            ItemRelation::InFixture(fixture_id) => *fixtures.get(&fixture_id).1,
            ItemRelation::OnGround(tile_id) => tile_id,
        };
        ItemJoined {
            relation,
            pos,
            item: self.clone(),
        }
    }
}

pub struct ItemJoined {
    pub relation: ItemRelation,
    pub pos: TileId,
    pub item: Item,
}

/// Fixtures
/// Required components: WorldTag, TileId
#[derive(Component, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[require(WorldTag)]
pub struct Fixture {
    pub id: FixtureId,
    pub kind: FixtureKind,
    pub inventory: Inventory,
    pub harvest_countdown: Option<u32>,
}

impl FromStr for FixtureKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (s, reset) = s.split_once(':').unwrap_or((s, ""));
        match s {
            "SleepingPad" => Ok(Self::SleepingPad),
            "Stockpile" => Ok(Self::Stockpile),
            "BerryBush" => Ok(Self::BerryBush),
            "Untyped" => Ok(Self::Untyped(reset.to_string())),
            _ => Err(format!("Invalid FixtureKind: {s}")),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum FixtureKind {
    SleepingPad,
    Stockpile,
    BerryBush,
    Untyped(String),
    // Field,
    // Tree,
    // House,
}

impl_hassimid!(Pawn, PawnId);
impl_hassimid!(Item, ItemId);
impl_hassimid!(Fixture, FixtureId);
