use bevy::prelude::*;
use serde::Serialize;

use crate::{EditBudget, RngResource};
use simkit_core::Playback;

#[derive(Debug, Serialize)]
pub struct SnapshotV0 {
    pub version: u32,
    pub tick: i32,
    pub seed: u64,
    pub edit_per_tick: u32,
}

pub fn extract_snapshot_v0(playback: &Playback, rng: &RngResource, budget: &EditBudget) -> SnapshotV0 {
    SnapshotV0 {
        version: 0,
        tick: playback.tick.0,
        seed: get_seed_from_rng(rng),
        edit_per_tick: budget.per_tick,
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

