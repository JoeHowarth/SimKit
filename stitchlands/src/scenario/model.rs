use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use simkit_core::grid::TileId;
use std::collections::HashMap;

// Basic map/tiles; unused in 0.b beyond size
// On-disk files are ScenarioDef (serde-renamed to Scenario)

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DefaultsDef {}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct MapDef {
    #[serde(default)]
    pub size: MapSize,
    #[serde(default)]
    pub tiles: Vec<TileDef>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct MapSize {
    pub x: u32,
    pub y: u32,
}

impl Default for MapSize {
    fn default() -> Self {
        Self { x: 64, y: 64 }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct TileDef {
    pub pos: TileId,
    #[serde(default)]
    pub walkable: bool,
    #[serde(default)]
    pub terrain: Terrain,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
pub enum Terrain {
    #[default]
    Grass,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PawnDef {
    pub id: Option<u64>,
    pub name: Option<String>,
    pub pos: Option<TileId>,
    #[serde(default)]
    pub needs: NeedsDef,
    #[serde(default)]
    pub priorities: HashMap<String, i32>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
pub struct NeedsDef {
    pub hunger: f32,
    pub rest: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ItemDef {
    pub id: Option<u64>,
    pub kind: String,
    pub qty: u32,
    pub pos: Option<TileId>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ZoneDef {
    pub id: Option<u64>,
    pub kind: String,
    pub rect: Option<(TileId, TileId)>,
    #[serde(default)]
    pub filters: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum DesignationDef {
    Harvest(TileId),
}

// Editable form used in RON files (allows many omissions)
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename = "Scenario")] // on-disk name is simply `Scenario(...)`
pub struct ScenarioDef {
    pub sim_seed: Option<u64>,
    #[serde(default)]
    pub map: MapDef,
    #[serde(default)]
    pub pawns: Vec<PawnDef>,
    #[serde(default)]
    pub items: Vec<ItemDef>,
    #[serde(default)]
    pub zones: Vec<ZoneDef>,
    #[serde(default)]
    pub designations: Vec<DesignationDef>,
    pub defaults: Option<DefaultsDef>,
}

// no legacy converter retained; completion occurs at load time
