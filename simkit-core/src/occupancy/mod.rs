use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::grid::{Grid2D, GridConfig, TileId};

bitflags::bitflags! {
    #[derive(Serialize, Deserialize, Reflect)]
    pub struct Occupancy: u8 {
        const PAWN = 0b0001;
        const ITEM = 0b0010;
        const BUILDING = 0b0100;
    }
}

impl Default for Occupancy {
    fn default() -> Self {
        Occupancy::empty()
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub struct EntityTileLink {
    pub tile: TileId,
}

/// Resource managing tile occupancy flags.
#[derive(Resource, Debug, Clone)]
pub struct OccupancyMap {
    grid: Grid2D<Occupancy>,
}

impl OccupancyMap {
    pub fn new(cfg: GridConfig) -> Self {
        Self {
            grid: Grid2D::new(cfg, Occupancy::empty()),
        }
    }

    #[inline]
    pub fn occupy(&mut self, tile: TileId, flags: Occupancy) {
        if let Some(cell) = self.grid.get_mut(tile) {
            *cell |= flags;
        }
    }

    #[inline]
    pub fn vacate(&mut self, tile: TileId, flags: Occupancy) {
        if let Some(cell) = self.grid.get_mut(tile) {
            *cell &= !flags;
        }
    }

    #[inline]
    pub fn get(&self, tile: TileId) -> Occupancy {
        self.grid.get(tile).copied().unwrap_or_else(Occupancy::empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn occupancy_invariants() {
        let cfg = GridConfig { width: 2, height: 2 };
        let mut occ = OccupancyMap::new(cfg);
        let t = TileId::new(0, 0);
        occ.occupy(t, Occupancy::PAWN);
        assert!(occ.get(t).contains(Occupancy::PAWN));
        // Idempotent occupy
        occ.occupy(t, Occupancy::PAWN);
        assert!(occ.get(t).contains(Occupancy::PAWN));
        // Vacate
        occ.vacate(t, Occupancy::PAWN);
        assert!(!occ.get(t).contains(Occupancy::PAWN));
        // Idempotent vacate
        occ.vacate(t, Occupancy::PAWN);
        assert!(!occ.get(t).contains(Occupancy::PAWN));
    }
}

