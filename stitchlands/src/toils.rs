use std::collections::VecDeque;

use simkit_core::grid::TileId;

use crate::model::ids::*;

pub enum ToilKind {
    ReserveItem {
        item: ItemId,
    },
    MoveTo {
        target: TileId,
        path: Option<VecDeque<TileId>>,
    },
    PickUp {
        item: ItemId,
    },
    PutDown {
        // consider allowing ItemId or ItemKind here
        item: ItemId,
        target: TileId,
    },
    Consume {
        item: ItemId,
    },
    Sleep {
        fixture: FixtureId,
    },
    Harvest {
        fixture: FixtureId,
    },
}
