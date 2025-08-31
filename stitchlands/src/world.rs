use bevy::prelude::*;
use simkit_core::grid::{Grid2D, GridConfig};

use crate::scenario::model::{MapDef, TileDef};

#[derive(Resource, Debug, Clone)]
pub struct WorldGrid {
    pub cfg: GridConfig,
    pub walkable: Grid2D<bool>,
}

impl WorldGrid {
    pub fn from_map(map: &MapDef) -> Self {
        let cfg = GridConfig {
            width: map.size.x,
            height: map.size.y,
        };
        let mut walkable = Grid2D::new(cfg, true);
        for TileDef {
            pos, walkable: w, ..
        } in map.tiles.iter().copied()
        {
            if let Some(cell) = walkable.get_mut(simkit_core::grid::TileId::new(pos.x, pos.y)) {
                *cell = w;
            }
        }
        Self { cfg, walkable }
    }
}
