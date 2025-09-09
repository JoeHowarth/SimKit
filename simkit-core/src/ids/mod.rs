use std::{collections::BTreeMap, fmt::Debug, hash::Hash};

use bevy::prelude::*;

// Typed ID newtypes used across crates. Keep minimal now; extend later.

pub trait SimId:
    Debug + Copy + Eq + Ord + PartialOrd + Hash + Send + Sync + 'static
{
    type Type: HasSimId;
    fn from_u64(v: u64) -> Self;
    fn to_u64(self) -> u64;
}

pub trait HasSimId: Component {
    type Id: SimId;
    fn id(&self) -> Self::Id;
}

#[macro_export]
macro_rules! impl_simid {
    ($t:ident, $ty:ty) => {
        #[derive(
            Debug,
            Copy,
            Clone,
            Eq,
            PartialEq,
            PartialOrd,
            Ord,
            Hash,
            Reflect,
            Serialize,
            Deserialize,
        )]
        pub struct $t(pub u64);

        impl SimId for $t {
            type Type = $ty;

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
        impl simkit_core::ids::HasSimId for $t {
            type Id = $id;

            #[inline]
            fn id(&self) -> Self::Id {
                self.id
            }
        }
    };
}

// #[derive(Resource, Debug, Clone)]
// pub struct IdAllocator<T: SimId> {
//     pub next: u64,
//     _m: PhantomData<T>,
// }

// impl<T: SimId> Default for IdAllocator<T> {
//     fn default() -> Self {
//         Self {
//             next: 1_000,
//             _m: PhantomData,
//         }
//     }
// }

// impl<T: SimId> IdAllocator<T> {
//     #[inline]
//     pub fn assign(&mut self, provided: Option<T>) -> T {
//         if let Some(id) = provided {
//             return id;
//         }
//         let id = if self.next == 0 { 1_000 } else { self.next };
//         self.next = id + 1;
//         T::from_u64(id)
//     }

//     #[inline]
//     pub fn peek_next(&self) -> u64 {
//         if self.next == 0 {
//             1_000
//         } else {
//             self.next
//         }
//     }

//     #[inline]
//     pub fn reset(&mut self, next: u64) {
//         self.next = next;
//     }
// }

#[derive(Resource, Debug)]
pub struct IdIndex<T: SimId>(pub BTreeMap<T, Option<Entity>>);

impl<T: SimId> Default for IdIndex<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T: SimId> IdIndex<T> {
    #[inline]
    pub fn register(&mut self, provided: T) {
        if self.0.insert(provided, None).is_some() {
            panic!("IdIndex already contains id: {provided:?}");
        }
    }

    #[inline]
    pub fn alloc(&mut self, provided: Option<T>) -> T {
        if let Some(id) = provided {
            self.register(id);
            return id;
        }
        let id = self
            .0
            .last_key_value()
            .map(|(k, _)| k.to_u64())
            .unwrap_or(1_000);
        let id = (id + 1).max(1_000);
        let id = T::from_u64(id);
        self.0.insert(id, None);
        id
    }

    pub fn remove(&mut self, id: T) {
        match self.0.entry(id).and_modify(|e| *e = None) {
            std::collections::btree_map::Entry::Vacant(_) => {
                panic!("IdIndex does not contain id: {id:?}")
            }
            std::collections::btree_map::Entry::Occupied(_) => {}
        }
    }

    #[inline]
    pub fn insert(&mut self, id: T, e: Entity) {
        if let Some(Some(existing)) = self.0.insert(id, Some(e)) {
            if existing != e {
                panic!(
                    "IdIndex already contains id: {id:?} entity: {existing:?}"
                );
            }
        }
    }

    #[inline]
    pub fn get(&self, id: &T) -> Entity {
        self.0.get(id).and_then(|e| *e).unwrap()
    }
    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

// FromWorld is auto-implemented by Bevy for types that implement Default.
