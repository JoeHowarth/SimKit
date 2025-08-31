use bevy::prelude::*;
use std::{collections::HashMap, fmt::Debug, hash::Hash, marker::PhantomData};

// Typed ID newtypes used across crates. Keep minimal now; extend later.

pub trait SimId: Copy + Eq + Hash + Send + Sync + 'static {
    fn from_u64(v: u64) -> Self;
    fn to_u64(self) -> u64;
}

pub trait HasSimId: Component {
    type Id: SimId;
    fn id(&self) -> Self::Id;
}

#[macro_export]
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

#[macro_export]
macro_rules! impl_hassimid {
    ($t:ty, $id:ty) => {
        impl HasSimId for $t {
            type Id = $id;

            #[inline]
            fn id(&self) -> Self::Id {
                self.id
            }
        }
    };
}

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
