use std::str::FromStr;

use bevy::{
    ecs::{
        query::{QueryData, QueryFilter, QueryItem, ROQueryItem},
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
    fn has_nutrition(&self) -> Option<Q40p24> {
        match self {
            ItemKind::Berry => Some(Q40p24::ONE),
            // ItemKind::Untyped(_) => None,
        }
    }

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
        let (s, reset) = s.split_once(':').unwrap_or((s, ""));
        match s {
            "Berry" => Ok(Self::Berry),
            // "Untyped" => Ok(Self::Untyped(reset.to_string())),
            _ => Err(format!("Invalid ItemKind: {s}")),
        }
    }
}

#[derive(Component, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CarriedBy(pub PawnId);

#[derive(Component, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct InFixture(pub FixtureId);

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

#[derive(SystemParam)]
pub struct IdQuery<
    'w,
    's,
    C: HasSimId,
    D: bevy::ecs::query::QueryData + 'static,
    F: QueryFilter + 'static = (),
> {
    pub query: Query<'w, 's, (&'static C, D), F>,
    pub index: Res<'w, IdIndex<<C as HasSimId>::Id>>,
}

#[derive(SystemParam)]
pub struct IdQueryMut<
    'w,
    's,
    C: HasSimId,
    D: bevy::ecs::query::QueryData + 'static,
    F: QueryFilter + 'static = (),
> where
    C: Component<Mutability = bevy::ecs::component::Mutable>,
{
    pub query: Query<'w, 's, (&'static mut C, D), F>,
    pub index: ResMut<'w, IdIndex<<C as HasSimId>::Id>>,
}

impl<'w, 's, C, D, F> IdQuery<'w, 's, C, D, F>
where
    D: QueryData,
    F: QueryFilter,
    C: HasSimId,
{
    pub fn get(&self, id: &<C as HasSimId>::Id) -> (&'_ C, ROQueryItem<'_, D>) {
        let entity = self.index.get(id);
        self.query.get(entity).unwrap()
    }

    pub fn entity(&'w self, id: &<C as HasSimId>::Id) -> Entity {
        self.index.get(id)
    }
}

impl<'w, 's, C, D, F> IdQueryMut<'w, 's, C, D, F>
where
    D: QueryData + 'static,
    F: QueryFilter + 'static,
    C: HasSimId
        + Component<Mutability = bevy::ecs::component::Mutable>
        + 'static,
{
    pub fn get(
        &'w self,
        id: &<C as HasSimId>::Id,
    ) -> (&'w C, ROQueryItem<'w, D>) {
        let entity = self.index.get(id);
        self.query.get(entity).unwrap()
    }

    pub fn get_mut(
        &mut self,
        id: &<C as HasSimId>::Id,
    ) -> (Mut<'_, C>, D::Item<'_>) {
        let entity = self.index.get(id);
        self.query.get_mut(entity).unwrap()
    }

    pub fn entity(&'w self, id: &<C as HasSimId>::Id) -> Entity {
        self.index.get(id)
    }
}

type OnlyItem = (Without<Pawn>, Without<Fixture>);
type OnlyPawn = (Without<Item>, Without<Fixture>);
type OnlyFixture = (Without<Pawn>, Without<Item>);

pub type FixtureQuery<'w, 's, D, F = ()> =
    IdQuery<'w, 's, Fixture, D, (F, OnlyFixture)>;
pub type ItemQuery<'w, 's, D, F = ()> = IdQuery<'w, 's, Item, D, (F, OnlyItem)>;
pub type PawnQuery<'w, 's, D, F = ()> = IdQuery<'w, 's, Pawn, D, (F, OnlyPawn)>;

pub type FixtureQueryMut<'w, 's, D, F = ()> =
    IdQueryMut<'w, 's, Fixture, D, (F, OnlyFixture)>;
pub type ItemQueryMut<'w, 's, D, F = ()> =
    IdQueryMut<'w, 's, Item, D, (F, OnlyItem)>;
pub type PawnQueryMut<'w, 's, D, F = ()> =
    IdQueryMut<'w, 's, Pawn, D, (F, OnlyPawn)>;

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
