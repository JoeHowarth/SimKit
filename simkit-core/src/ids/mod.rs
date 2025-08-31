use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug, hash::Hash, marker::PhantomData};

// Typed ID newtypes used across crates. Keep minimal now; extend later.

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Reflect, Serialize, Deserialize)]
pub struct PawnId(pub u64);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Reflect, Serialize, Deserialize)]
pub struct ItemId(pub u64);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Reflect, Serialize, Deserialize)]
pub struct ZoneId(pub u64);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Reflect, Serialize, Deserialize)]
pub struct BlueprintId(pub u64);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Reflect, Serialize, Deserialize)]
pub struct BedId(pub u64);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Reflect, Serialize, Deserialize)]
pub struct TaskId(pub u64);

pub trait SimId: Copy + Eq + Hash + Send + Sync + 'static {
    fn from_u64(v: u64) -> Self;
    fn to_u64(self) -> u64;
}

macro_rules! impl_simid {
    ($t:ty) => {
        impl SimId for $t {
            #[inline]
            fn from_u64(v: u64) -> Self {
                Self(v)
            }
            #[inline]
            fn to_u64(self) -> u64 {
                self.0
            }
        }
    };
}

impl_simid!(PawnId);
impl_simid!(ItemId);
impl_simid!(ZoneId);
impl_simid!(BlueprintId);
impl_simid!(BedId);
impl_simid!(TaskId);

#[derive(Resource, Debug, Clone)]
pub struct IdAllocator<T: SimId> {
    pub next: u64,
    _m: PhantomData<T>,
}

impl<T: SimId> Default for IdAllocator<T> {
    fn default() -> Self {
        Self {
            next: 1_000,
            _m: PhantomData,
        }
    }
}

impl<T: SimId> IdAllocator<T> {
    #[inline]
    pub fn assign(&mut self, provided: Option<T>) -> T {
        if let Some(id) = provided {
            return id;
        }
        let id = if self.next == 0 { 1_000 } else { self.next };
        self.next = id + 1;
        T::from_u64(id)
    }

    #[inline]
    pub fn peek_next(&self) -> u64 {
        if self.next == 0 {
            1_000
        } else {
            self.next
        }
    }

    #[inline]
    pub fn reset(&mut self, next: u64) {
        self.next = next;
    }
}

#[derive(Resource, Debug)]
pub struct IdIndex<T: SimId>(pub HashMap<T, Entity>);

impl<T: SimId> Default for IdIndex<T> {
    fn default() -> Self {
        Self(HashMap::default())
    }
}

impl<T: SimId> IdIndex<T> {
    #[inline]
    pub fn insert(&mut self, id: T, e: Entity) {
        self.0.insert(id, e);
    }
    #[inline]
    pub fn get(&self, id: &T) -> Option<Entity> {
        self.0.get(id).copied()
    }
    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

// FromWorld is auto-implemented by Bevy for types that implement Default.
