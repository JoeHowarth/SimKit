use bevy::{
    ecs::{
        query::{QueryData, QueryFilter, ROQueryItem},
        system::SystemParam,
    },
    prelude::*,
};
use simkit_core::ids::{HasSimId, IdIndex, SimId};

use super::*;

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
