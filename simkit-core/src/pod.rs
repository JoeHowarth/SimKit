use std::{fmt::Debug, hash::Hash};

use bevy::reflect::{FromReflect, GetTypeRegistration, Reflect, Typed};
use serde::{de::DeserializeOwned, Serialize};

/// A trait for types that are POD (Plain Old Data)
pub trait POD:
    Reflect
    + FromReflect
    + Debug
    + Clone
    + PartialEq
    + Eq
    + Hash
    + Send
    + Sync
    + Typed
    + GetTypeRegistration
    + Serialize
    + DeserializeOwned
    + 'static
{
}
impl<
        T: Reflect
            + FromReflect
            + Debug
            + Clone
            + PartialEq
            + Eq
            + Hash
            + Send
            + Sync
            + Typed
            + GetTypeRegistration
            + Serialize
            + DeserializeOwned
            + 'static,
    > POD for T
{
}


// add derives to a struct or enum definnition
// used like:
// pod!{
// pub struct MyState(pub MyStateSubType);
// }
// this will add the following to the definnition:
// #[derive(Debug, Clone, Reflect, Default, Resource, Hash, PartialEq, Eq, Serialize, Deserialize)]
// pub struct MyState(pub MyStateSubType);
// pod! will take a block of definnitions and add the derives to each type definnition inside it
// }
#[macro_export]
macro_rules! pod {
    ($($item:item)* ) => {
        $(
            #[derive(Reflect, Debug, Clone, PartialEq, Eq, Hash,  serde::Serialize, serde::Deserialize)]
            $item
        )*
    }
}
