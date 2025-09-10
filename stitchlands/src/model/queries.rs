use bevy::{
    ecs::{
        query::{QueryData, QueryFilter, ROQueryItem},
        system::{QueryLens, SystemParam},
    },
    prelude::*,
};
use simkit_core::{
    grid::TileId,
    ids::{HasSimId, IdIndex, SimId},
};

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

pub type FixtureQuery<
    'w,
    's,
    D = (
        &'static TileId,
        Option<&'static HarvestCountdown>,
        Option<&'static BuildWorkLeft>,
    ),
    F = (),
> = IdQuery<'w, 's, Fixture, D, (F, OnlyFixture)>;
pub type ItemQuery<'w, 's, D, F = ()> = IdQuery<'w, 's, Item, D, (F, OnlyItem)>;
pub type PawnQuery<'w, 's, D, F = ()> = IdQuery<'w, 's, Pawn, D, (F, OnlyPawn)>;

pub type FixtureQueryMut<'w, 's, D, F = ()> =
    IdQueryMut<'w, 's, Fixture, D, (F, OnlyFixture)>;
pub type ItemQueryMut<'w, 's, D, F = ()> =
    IdQueryMut<'w, 's, Item, D, (F, OnlyItem)>;
pub type PawnQueryMut<'w, 's, D, F = ()> =
    IdQueryMut<'w, 's, Pawn, D, (F, OnlyPawn)>;

impl FixtureQuery<'_, '_> {
    pub fn tile_id(&self, id: &FixtureId) -> &TileId {
        self.get(id).1.0
    }

    pub fn harvest_countdown(
        &self,
        id: &FixtureId,
    ) -> Option<&HarvestCountdown> {
        self.get(id).1.1
    }

    pub fn build_work_left(&self, id: &FixtureId) -> Option<&BuildWorkLeft> {
        self.get(id).1.2
    }
}

pub trait WorldExt {
    fn get_simid<Id: SimId>(&self, id: &Id) -> (&Id::Type, Entity);
    fn get_simid_mut<Id: SimId>(
        &mut self,
        id: &Id,
    ) -> (Mut<'_, Id::Type>, Entity);
    fn get_comp_simid<C: Component, Id: SimId>(
        &self,
        id: &Id,
    ) -> (&Id::Type, Entity, &C);
}

impl WorldExt for World {
    fn get_simid_mut<Id: SimId>(
        &mut self,
        id: &Id,
    ) -> (Mut<'_, Id::Type>, Entity) {
        let e = self.resource::<IdIndex<Id>>().get(id);
        let component = self.get_mut::<Id::Type>(e).unwrap();
        (component, e)
    }

    fn get_simid<Id: SimId>(&self, id: &Id) -> (&Id::Type, Entity) {
        let e = self.resource::<IdIndex<Id>>().get(id);
        let component = self.get::<Id::Type>(e).unwrap();
        (component, e)
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

pub fn neartest_item_pos(
    pawn: &Pawn,
    pawn_pos: &TileId,
    item_kind: &ItemKind,
    items: &ItemQuery<&ItemRelation>,
    fixtures: &FixtureQuery,
) -> Option<TileId> {
    if pawn.inventory.find(*item_kind).is_some() {
        return Some(*pawn_pos);
    }

    let on_ground = nearest_item_on_ground(item_kind, pawn_pos, items);
    let fixture = nearest_fixture_with_item(item_kind, pawn_pos, fixtures);

    let (item_id, _dist) = closer_option_item_locator(on_ground, fixture)?;
    Some(match items.get(&item_id).1 {
        ItemRelation::CarriedBy(_) => *pawn_pos,
        ItemRelation::InFixture(fixture_id) => *fixtures.tile_id(fixture_id),
        ItemRelation::OnGround(tile_id) => *tile_id,
    })
}

pub fn manhattan(a: TileId, b: TileId) -> u32 {
    ((a.x - b.x).abs() + (a.y - b.y).abs()) as u32
}

pub fn nearest_item_on_ground(
    target_kind: &ItemKind,
    current_pos: &TileId,
    items: &ItemQuery<&ItemRelation>,
) -> Option<(ItemId, u32)> {
    // find nearest item on ground that matches item
    let mut nearest = None;
    for (item, item_rel) in items.query.iter() {
        let ItemRelation::OnGround(item_pos) = item_rel else {
            continue;
        };

        if item.kind == *target_kind {
            let distance = manhattan(*current_pos, *item_pos);
            if distance
                > nearest
                    .as_ref()
                    .map(|(_, distance)| *distance)
                    .unwrap_or(u32::MAX)
            {
                continue;
            }
            nearest = Some((item.id, distance));
        }
    }
    nearest
}

pub fn nearest_fixture_with_item(
    target_kind: &ItemKind,
    current_pos: &TileId,
    fixtures: &FixtureQuery,
) -> Option<(ItemId, u32)> {
    // find nearest fixture that contains item
    let mut nearest = None;
    for (fixture, (fixture_pos, _, _)) in fixtures.query.iter() {
        let Some(loc) = fixture
            .inventory
            .find(*target_kind)
            .map(|id| (id, *current_pos))
        else {
            continue;
        };

        let distance = manhattan(*current_pos, *fixture_pos);
        if distance
            > nearest
                .as_ref()
                .map(|(_, distance)| *distance)
                .unwrap_or(u32::MAX)
        {
            continue;
        }

        nearest = Some((loc.0, distance));
    }
    nearest
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
