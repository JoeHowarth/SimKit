use std::str::FromStr;

use bevy::{
    ecs::{
        query::{QueryData, ROQueryItem},
        system::SystemParam,
    },
    log::tracing::span::Id,
    prelude::*,
};
use serde::{Deserialize, Serialize};
use simkit_core::{
    fixed_point::Q40p24,
    ids::{HasSimId, IdIndex, SimId},
    impl_hassimid,
};

use crate::{
    model::ids::{FixtureId, ItemId, PawnId},
    WorldTag,
};

/// Pawns
/// Required components: WorldTag, TileId
#[derive(Component, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[require(WorldTag)]
pub struct Pawn {
    pub id: PawnId,
    pub inventory: Vec<(ItemId, ItemKind)>,
    pub sleep: Q40p24,
    pub hunger: Q40p24,
    // health
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

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum ItemKind {
    Berry,
    Untyped(String),
}

impl FromStr for ItemKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (s, reset) = s.split_once(':').unwrap_or((s, ""));
        match s {
            "Berry" => Ok(Self::Berry),
            "Untyped" => Ok(Self::Untyped(reset.to_string())),
            _ => Err(format!("Invalid ItemKind: {s}")),
        }
    }
}

#[derive(Component, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CarriedBy(pub PawnId);

#[derive(Component, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct InFixture(pub FixtureId);

#[derive(SystemParam)]
pub struct IdQuery<
    'w,
    's,
    C: HasSimId,
    D: bevy::ecs::query::QueryData + 'static,
> {
    pub query: Query<'w, 's, (&'static C, D)>,
    index: Res<'w, IdIndex<<C as HasSimId>::Id>>,
}

impl<'w, 's: 'w, C, D> IdQuery<'w, 's, C, D>
where
    D: QueryData,
    C: HasSimId,
{
    pub fn get(
        &'w self,
        id: &<C as HasSimId>::Id,
    ) -> (&'w C, ROQueryItem<'w, D>) {
        let entity = self.index.get(id);
        self.query.get(entity).unwrap()
    }
}

pub type FixtureQuery<'w, 's, D> = IdQuery<'w, 's, Fixture, D>;
pub type ItemQuery<'w, 's, D> = IdQuery<'w, 's, Item, D>;
pub type PawnQuery<'w, 's, D> = IdQuery<'w, 's, Pawn, D>;

pub trait WorldExt {
    fn get_simid<Id: SimId>(&self, id: &Id) -> (&Id::Type, Entity);
    fn get_comp_simid<C: Component, Id: SimId>(
        &self,
        id: &Id,
    ) -> (&Id::Type, Entity, &C);
}

impl WorldExt for World {
    fn get_simid<Id: SimId>(&self, id: &Id) -> (&Id::Type, Entity) {
        let x = self.resource::<IdIndex<Id>>();
        let e = x.get(id);
        let entity = self.get::<Id::Type>(e);
        (entity.unwrap(), e)
    }

    fn get_comp_simid<C: Component, Id: SimId>(
        &self,
        id: &Id,
    ) -> (&Id::Type, Entity, &C) {
        let (entity, e) = self.get_simid(id);
        let component = self.get::<C>(e).unwrap();
        (entity, e, component)
    }
}

// #[derive(SystemParam)]
// pub struct FixtureQuery<'w, 's, D: bevy::ecs::query::QueryData + 'static> {
//     pub query: Query<'w, 's, (&'static Fixture, D)>,
//     index: Res<'w, IdIndex<FixtureId>>,
// }

// impl<'w, 's: 'w, D> FixtureQuery<'w, 's, D>
// where
//     D: QueryData,
// {
//     pub fn get(&'w self, id: &FixtureId) -> (&'w Fixture, ROQueryItem<'w, D>)
// {         let entity = self.index.get(id);
//         self.query.get(entity).unwrap()
//     }
// }

/// Fixtures
/// Required components: WorldTag, TileId
#[derive(Component, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[require(WorldTag)]
pub struct Fixture {
    pub id: FixtureId,
    pub kind: FixtureKind,
    pub inventory: Vec<(ItemId, ItemKind)>,
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
