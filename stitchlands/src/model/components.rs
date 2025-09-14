use std::str::FromStr;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use simkit_core::{
    fixed_point::Q40p24,
    grid::{TileId, index::TileMapIndex},
    ids::IdIndex,
    impl_hassimid,
};

use crate::{
    WorldTag,
    model::{
        FixtureQuery,
        PawnQuery,
        ids::{FixtureId, ItemId, PawnId},
    },
    tasks::{BuildingSpec, Job},
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
            if *kind == item_kind { Some(*id) } else { None }
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
    Wood,
    Nails,
}

impl ItemKind {
    #[allow(dead_code)]
    fn has_nutrition(&self) -> Option<Q40p24> {
        match self {
            ItemKind::Berry => Some(Q40p24::ONE),
            ItemKind::Wood | ItemKind::Nails => None,
        }
    }

    #[allow(dead_code)]
    fn plantable_fixture(&self) -> Option<FixtureKind> {
        match self {
            ItemKind::Berry => Some(FixtureKind::BerryBush),
            ItemKind::Wood | ItemKind::Nails => None,
        }
    }
}

impl FromStr for ItemKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (s, _reset) = s.split_once(':').unwrap_or((s, ""));
        match s {
            "Berry" => Ok(Self::Berry),
            "Wood" => Ok(Self::Wood),
            "Nails" => Ok(Self::Nails),
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
}

impl Fixture {
    pub fn spawn(
        commands: &mut Commands,
        index: &mut IdIndex<FixtureId>,
        tile_index: &mut TileMapIndex<FixtureId>,
        mut fixture: Fixture,
        pos: TileId,
        bundle: impl Bundle,
    ) -> FixtureId {
        let fixture_id = index.alloc(None);
        fixture.id = fixture_id;
        let name = Name::new(format!("{}#{}", fixture.kind, fixture_id.0));
        let fixture_entity = commands.spawn((fixture, pos, bundle, name)).id();
        index.insert(fixture_id, fixture_entity);
        tile_index.move_id(None, pos, fixture_id);
        fixture_id
    }
}

/// How many work units are left to complete the build
#[derive(Component, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstructionSite {
    pub building_spec: BuildingSpec,
    pub work_left: u32,
}

/// Ticks until harvest is ready
#[derive(
    Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub struct Harvestable {
    pub countdown: u32,
    pub seq_num: u32,
}

impl FromStr for FixtureKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (s, reset) = s.split_once(':').unwrap_or((s, ""));
        match s {
            "SleepingPad" => Ok(Self::SleepingPad),
            "Stockpile" => Ok(Self::Stockpile),
            "BerryBush" => Ok(Self::BerryBush),
            _ => Err(format!("Invalid FixtureKind: {s}")),
        }
    }
}

impl std::fmt::Display for FixtureKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum FixtureKind {
    ConstructionSite,
    SleepingPad,
    Stockpile,
    BerryBush,
    Cabin,
    // Field,
    // Tree,
}

impl_hassimid!(Pawn, PawnId);
impl_hassimid!(Item, ItemId);
impl_hassimid!(Fixture, FixtureId);
