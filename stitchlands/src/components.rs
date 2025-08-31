use crate::ids::{ItemId, PawnId, ZoneId};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use simkit_core::grid::TileId;
use simkit_core::ids::HasSimId;
use simkit_core::impl_hassimid;

#[derive(Component, Debug, Clone, Eq, PartialEq, Copy, Serialize, Deserialize)]
pub struct Pawn {
    pub id: PawnId,
}

// Minimal item/zones components for spawned entities
#[derive(Component, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub kind: String,
    pub qty: u32,
}

#[derive(Component, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    pub id: ZoneId,
    pub kind: String,
    pub rect: (TileId, TileId),
    pub filters: Vec<String>,
}

impl_hassimid!(Pawn, PawnId);
impl_hassimid!(Item, ItemId);
impl_hassimid!(Zone, ZoneId);
