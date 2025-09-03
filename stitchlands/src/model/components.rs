use std::str::FromStr;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use simkit_core::{fixed_point::Q40p24, ids::HasSimId, impl_hassimid};

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
