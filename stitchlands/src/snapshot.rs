use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::scenario::model::{Item, Pawn, Position, Zone};
use simkit_core::Playback;

pub fn stable_hash_json<T: Serialize>(value: &T) -> String {
    let json = serde_json::to_vec(value).expect("serialize snapshot");
    fnv1a64_hex(&json)
}

fn fnv1a64_hex(bytes: &[u8]) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001B3;
    let mut hash = FNV_OFFSET;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PawnEntry {
    pub id: u64,
    pub x: i32,
    pub y: i32,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemEntry {
    pub id: u64,
    pub kind: String,
    pub qty: u32,
    pub x: i32,
    pub y: i32,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZoneEntry {
    pub id: u64,
    pub kind: String,
    pub rect: ((i32, i32), (i32, i32)),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorldSnapshot {
    pub tick: i32,
    pub scenario_seed: Option<u64>,
    pub pawns: Vec<PawnEntry>,
    pub items: Vec<ItemEntry>,
    pub zones: Vec<ZoneEntry>,
}

pub fn build_world_snapshot(
    playback: &Playback,
    scenario_seed: Option<u64>,
    pawns: &[(Pawn, Position)],
    items: &[(Item, Position)],
    zones: &[Zone],
) -> WorldSnapshot {
    let mut pawn_entries: Vec<PawnEntry> = pawns
        .iter()
        .map(|(p, pos)| PawnEntry {
            id: p.0 .0,
            x: pos.0.x,
            y: pos.0.y,
        })
        .collect();
    pawn_entries.sort_by_key(|e| e.id);
    let mut item_entries: Vec<ItemEntry> = items
        .iter()
        .map(|(it, pos)| ItemEntry {
            id: it.id.0,
            kind: it.kind.clone(),
            qty: it.qty,
            x: pos.0.x,
            y: pos.0.y,
        })
        .collect();
    item_entries.sort_by_key(|e| e.id);
    let mut zone_entries: Vec<ZoneEntry> = zones
        .iter()
        .map(|z| ZoneEntry {
            id: z.id.0,
            kind: z.kind.clone(),
            rect: ((z.rect.0.x, z.rect.0.y), (z.rect.1.x, z.rect.1.y)),
        })
        .collect();
    zone_entries.sort_by_key(|e| e.id);

    WorldSnapshot {
        tick: playback.tick.0,
        scenario_seed,
        pawns: pawn_entries,
        items: item_entries,
        zones: zone_entries,
    }
}

// Load a snapshot back into the world using the same serializable definition
pub fn load_world_snapshot(
    commands: &mut Commands,
    pawn_index: &mut simkit_core::ids::IdIndex<simkit_core::ids::PawnId>,
    item_index: &mut simkit_core::ids::IdIndex<simkit_core::ids::ItemId>,
    zone_index: &mut simkit_core::ids::IdIndex<simkit_core::ids::ZoneId>,
    snapshot: &WorldSnapshot,
) {
    use simkit_core::ids::{ItemId, PawnId, ZoneId};
    // Spawn pawns
    for p in snapshot.pawns.iter() {
        let typed = PawnId(p.id);
        let entity = commands
            .spawn((
                crate::WorldTag,
                Name::new(format!("Pawn#{}", typed.0)),
                Pawn(typed),
                Position(crate::scenario::model::TilePos { x: p.x, y: p.y }),
            ))
            .id();
        pawn_index.insert(typed, entity);
    }
    // Spawn items
    for it in snapshot.items.iter() {
        let typed = ItemId(it.id);
        let entity = commands
            .spawn((
                crate::WorldTag,
                Name::new(format!("Item#{}", typed.0)),
                Item {
                    id: typed,
                    kind: it.kind.clone(),
                    qty: it.qty,
                },
                Position(crate::scenario::model::TilePos { x: it.x, y: it.y }),
            ))
            .id();
        item_index.insert(typed, entity);
    }
    // Spawn zones
    for z in snapshot.zones.iter() {
        let typed = ZoneId(z.id);
        let entity = commands
            .spawn((
                crate::WorldTag,
                Name::new(format!("Zone#{}", typed.0)),
                Zone {
                    id: typed,
                    kind: z.kind.clone(),
                    rect: (
                        crate::scenario::model::TilePos {
                            x: (z.rect).0 .0,
                            y: (z.rect).0 .1,
                        },
                        crate::scenario::model::TilePos {
                            x: (z.rect).1 .0,
                            y: (z.rect).1 .1,
                        },
                    ),
                    filters: vec![],
                },
            ))
            .id();
        zone_index.insert(typed, entity);
    }
}
