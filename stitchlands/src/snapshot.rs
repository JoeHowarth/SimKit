use bevy::prelude::*;
use serde::Serialize;

use crate::scenario::model::{Item, Pawn, Position, Zone};
use crate::scenario::LoadedScenarioMeta;
use crate::{EditBudget, RngResource, WorldTag};
use simkit_core::Playback;

#[derive(Debug, Serialize)]
pub struct SnapshotV0 {
    pub version: u32,
    pub tick: i32,
    pub seed: u64,
    pub edit_per_tick: u32,
    pub world_tag_count: u32,
    pub pawns_count: u32,
    pub scenario_seed: Option<u64>,
}

pub fn extract_snapshot_v0(
    playback: &Playback,
    rng: &RngResource,
    budget: &EditBudget,
) -> SnapshotV0 {
    // Note: called at end of tick in headless. We reconstruct counts via a temporary AppWorld if needed.
    // In practice, this function will be extended to extract from World; for 0.a/0.b keep it simple.
    SnapshotV0 {
        version: 0,
        tick: playback.tick.0,
        seed: get_seed_from_rng(rng),
        edit_per_tick: budget.per_tick,
        world_tag_count: 0,
        pawns_count: 0,
        scenario_seed: None,
    }
}

fn get_seed_from_rng(_rng: &RngResource) -> u64 {
    // In 0.a we just mirror CLI seed; SmallRng doesn't expose seed, so we rely on CLI seeding.
    // For determinism checks, we pass the CLI seed separately.
    // This function remains as a placeholder for later extraction logic.
    0
}

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

#[derive(Debug, Serialize)]
pub struct PawnEntry {
    pub id: u64,
    pub x: i32,
    pub y: i32,
}
#[derive(Debug, Serialize)]
pub struct ItemEntry {
    pub id: u64,
    pub kind: String,
    pub qty: u32,
    pub x: i32,
    pub y: i32,
}
#[derive(Debug, Serialize)]
pub struct ZoneEntry {
    pub id: u64,
    pub kind: String,
    pub rect: ((i32, i32), (i32, i32)),
}

#[derive(Debug, Serialize)]
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
    pawns: &Vec<(Pawn, Position)>,
    items: &Vec<(Item, Position)>,
    zones: &Vec<Zone>,
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
