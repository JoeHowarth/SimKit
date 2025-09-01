use bevy::prelude::*;

use crate::{
    grid::{Grid2D, GridConfig, TileId},
    ids::{HasSimId, SimId},
};

#[derive(Resource, Debug, Clone)]
pub struct TileMapIndex<T: SimId>(pub Grid2D<Option<T>>);

impl<T: SimId> TileMapIndex<T> {
    pub fn new(cfg: GridConfig) -> Self {
        Self(Grid2D::new(cfg, None))
    }

    #[inline]
    pub fn get(&self, tile: TileId) -> Option<T> {
        self.0.get(tile).and_then(|v| *v)
    }

    #[inline]
    fn set(&mut self, tile: TileId, id: T) {
        if let Some(cell) = self.0.get_mut(tile) {
            *cell = Some(id);
        }
    }

    #[inline]
    pub fn clear(&mut self, tile: TileId) {
        if let Some(cell) = self.0.get_mut(tile) {
            *cell = None;
        }
    }

    #[inline]
    /// Set an ID at a tile, clearing the old tile and updating the reference if
    /// provided.
    pub fn move_id(&mut self, from: Option<&mut TileId>, to: TileId, id: T) {
        if let Some(f) = from {
            self.clear(*f);
            *f = to;
        }
        self.set(to, id);
    }
}

pub fn sync_tile_index<T: HasSimId>(
    positions: Query<(&TileId, &T), Changed<TileId>>,
    mut idx: ResMut<TileMapIndex<T::Id>>,
) {
    for (pos, id) in positions.iter() {
        idx.set(*pos, id.id());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
    struct FooId(u64);
    impl SimId for FooId {
        fn from_u64(v: u64) -> Self {
            Self(v)
        }
        fn to_u64(self) -> u64 {
            self.0
        }
    }

    #[test]
    fn set_get_move_clear() {
        let cfg = GridConfig {
            width: 3,
            height: 2,
        };
        let mut idx = TileMapIndex::<FooId>::new(cfg);
        let a = TileId::new(0, 0);
        let b = TileId::new(1, 0);
        idx.set(a, FooId(1));
        assert_eq!(idx.get(a), Some(FooId(1)));
        let mut prev = a;
        idx.move_id(Some(&mut prev), b, FooId(1));
        assert_eq!(idx.get(a), None);
        assert_eq!(idx.get(b), Some(FooId(1)));
        idx.clear(b);
        assert_eq!(idx.get(b), None);
    }
}
