use std::sync::Arc;

use bevy::prelude::*;
use dashmap::DashSet;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReservationKey {
    Item(ItemId),
    Tile(TileId),
    Fixture(FixtureId),
}

#[derive(Debug)]
pub struct ReservationGuard {
    pub key: ReservationKey,
    pub reservations: Reservations,
}

impl PartialEq for ReservationGuard {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for ReservationGuard {}

impl Drop for ReservationGuard {
    fn drop(&mut self) {
        self.reservations.release(self.key);
    }
}

#[derive(Resource, Debug, Default, Clone)]
pub struct Reservations(Arc<DashSet<ReservationKey>>);

impl Reservations {
    pub fn is_reserved(&self, key: impl Into<ReservationKey>) -> bool {
        self.0.contains(&key.into())
    }

    pub fn reserve(&self, key: impl Into<ReservationKey>) -> ReservationGuard {
        let key = key.into();
        self.0.insert(key);
        ReservationGuard {
            key,
            reservations: self.clone(),
        }
    }

    pub fn release(&self, key: impl Into<ReservationKey>) {
        let key = key.into();
        self.0.remove(&key);
    }
}

impl From<ItemId> for ReservationKey {
    fn from(item_id: ItemId) -> Self {
        ReservationKey::Item(item_id)
    }
}

impl From<TileId> for ReservationKey {
    fn from(tile_id: TileId) -> Self {
        ReservationKey::Tile(tile_id)
    }
}

impl From<FixtureId> for ReservationKey {
    fn from(fixture_id: FixtureId) -> Self {
        ReservationKey::Fixture(fixture_id)
    }
}

#[derive(Debug)]
pub struct HeldReservations {
    held: HashMap<ReservationKey, ReservationGuard>,
    pub handle: Reservations,
}

impl PartialEq for HeldReservations {
    fn eq(&self, other: &Self) -> bool {
        self.held == other.held
    }
}

impl Eq for HeldReservations {}

impl HeldReservations {
    pub fn new(res: &Reservations) -> Self {
        Self {
            held: HashMap::new(),
            handle: res.clone(),
        }
    }

    pub fn try_ensure(
        &mut self,
        key: impl Into<ReservationKey>,
    ) -> Result<(), String> {
        let key = key.into();
        if self.held.contains_key(&key) {
            return Ok(());
        }
        self.try_reserve(key)
    }

    pub fn try_reserve(
        &mut self,
        key: impl Into<ReservationKey>,
    ) -> Result<(), String> {
        let key = key.into();
        if self.handle.is_reserved(key) {
            return Err(format!("Key {:?} is already reserved", key));
        }
        self.held.insert(key, self.handle.reserve(key));
        Ok(())
    }

    pub fn insert(&mut self, guard: ReservationGuard) {
        self.held.insert(guard.key, guard);
    }

    pub fn remove(&mut self, key: impl Into<ReservationKey>) {
        self.held.remove(&key.into());
    }

    pub fn is_reserved(&self, key: impl Into<ReservationKey>) -> bool {
        self.held.contains_key(&key.into())
    }
}
