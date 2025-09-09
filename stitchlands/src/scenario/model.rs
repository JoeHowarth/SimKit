use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use simkit_core::grid::TileId;

use crate::tasks::TaskSpecKind;

// Basic map/tiles; unused in 0.b beyond size
// On-disk files are ScenarioDef (serde-renamed to Scenario)

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct MapDef {
    pub size: MapSize,
    pub tiles: Vec<TileDef>,
    #[serde(default)]
    pub schematic: Option<String>,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TileDef {
    pub pos: TileId,
    #[serde(default)]
    pub walkable: bool,
    #[serde(default)]
    pub terrain: Terrain,
    #[serde(default)]
    pub item: Option<ItemDef>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
pub enum Terrain {
    #[default]
    Grass,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct PawnDef {
    pub id: Option<u64>,
    pub name: Option<String>,
    pub pos: Option<TileId>,
    pub sleep: Option<u32>,
    pub hunger: Option<u32>,
    pub priorities: Vec<TaskSpecKind>,
    pub inventory: Vec<ItemDef>,
}

impl Default for PawnDef {
    fn default() -> Self {
        Self {
            id: None,
            name: None,
            pos: None,
            sleep: Some(100),
            hunger: Some(100),
            priorities: vec![TaskSpecKind::Harvest, TaskSpecKind::Plant],
            inventory: vec![],
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ItemDef {
    pub id: Option<u64>,
    pub kind: String,
    pub qty: u32,
    pub pos: Option<TileId>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FixtureDef {
    pub id: Option<u64>,
    pub kind: String,
    pub pos: Option<TileId>,
    pub inventory: Vec<ItemDef>,
    pub harvest_countdown: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum TaskDef {
    Harvest(TileId),
}

// Editable form used in RON files (allows many omissions)
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename = "Scenario")]
#[serde(default)]
pub struct ScenarioDef {
    pub sim_seed: Option<u64>,
    pub map: MapDef,
    pub pawns: Vec<PawnDef>,
    pub fixtures: Vec<FixtureDef>,
    pub tasks: Vec<TaskDef>,
}
