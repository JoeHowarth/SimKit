pub mod index;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Configuration for a 2D grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct GridConfig {
    pub width: u32,
    pub height: u32,
}

impl GridConfig {
    #[inline]
    pub fn in_bounds(&self, tile: TileId) -> bool {
        tile.x >= 0 && tile.y >= 0 && (tile.x as u32) < self.width && (tile.y as u32) < self.height
    }

    #[inline]
    pub fn index(&self, tile: TileId) -> Option<usize> {
        if self.in_bounds(tile) {
            Some(tile.y as usize * self.width as usize + tile.x as usize)
        } else {
            None
        }
    }
}

/// Tile identifier using Bevy-style coordinates (x to the right, y up).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, Component, Default,
)]
pub struct TileId {
    pub x: i32,
    pub y: i32,
}

impl TileId {
    #[inline]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Simple 2D grid backed by a flat vector.
#[derive(Clone, Debug)]
pub struct Grid2D<T> {
    pub cfg: GridConfig,
    pub data: Vec<T>,
}

impl<T: Clone> Grid2D<T> {
    pub fn new(cfg: GridConfig, default: T) -> Self {
        let len = (cfg.width * cfg.height) as usize;
        Self {
            cfg,
            data: vec![default; len],
        }
    }

    #[inline]
    pub fn get(&self, tile: TileId) -> Option<&T> {
        self.cfg.index(tile).map(|i| &self.data[i])
    }

    #[inline]
    pub fn get_mut(&mut self, tile: TileId) -> Option<&mut T> {
        self.cfg.index(tile).map(move |i| &mut self.data[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_indexing() {
        let cfg = GridConfig {
            width: 4,
            height: 2,
        };
        let g: Grid2D<i32> = Grid2D::new(cfg, 0);
        assert_eq!(cfg.index(TileId::new(0, 0)), Some(0));
        assert_eq!(cfg.index(TileId::new(3, 0)), Some(3));
        assert_eq!(cfg.index(TileId::new(0, 1)), Some(4));
        assert!(cfg.index(TileId::new(4, 0)).is_none());
        assert_eq!(g.get(TileId::new(2, 1)), Some(&0));
    }
}
