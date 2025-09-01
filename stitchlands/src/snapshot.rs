use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use simkit_core::{grid::TileId, ids::IdIndex, Playback};

use crate::model::{
    components::{Item, Pawn, Zone},
    ids::{ItemId, PawnId, ZoneId},
};

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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PawnEntry {
    pub pawn: Pawn,
    pub pos: TileId,
}
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ItemEntry {
    pub item: Item,
    pub pos: TileId,
}
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ZoneEntry {
    pub zone: Zone,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
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
    pawns: &[(Pawn, TileId)],
    items: &[(Item, TileId)],
    zones: &[Zone],
) -> WorldSnapshot {
    let mut pawn_entries: Vec<PawnEntry> = pawns
        .iter()
        .map(|(p, pos)| PawnEntry {
            pawn: *p,
            pos: *pos,
        })
        .collect();
    pawn_entries.sort_by_key(|e| e.pawn.id);
    let mut item_entries: Vec<ItemEntry> = items
        .iter()
        .map(|(it, pos)| ItemEntry {
            item: it.clone(),
            pos: *pos,
        })
        .collect();
    item_entries.sort_by_key(|e| e.item.id);
    let mut zone_entries: Vec<ZoneEntry> = zones
        .iter()
        .map(|z| ZoneEntry { zone: z.clone() })
        .collect();
    zone_entries.sort_by_key(|e| e.zone.id);

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
    pawn_index: &mut IdIndex<PawnId>,
    item_index: &mut IdIndex<ItemId>,
    zone_index: &mut IdIndex<ZoneId>,
    snapshot: &WorldSnapshot,
) {
    // Spawn pawns
    for p in snapshot.pawns.iter() {
        let entity = commands
            .spawn((
                crate::WorldTag,
                Name::new(format!("Pawn#{}", p.pawn.id.0)),
                p.pawn,
                p.pos,
            ))
            .id();
        pawn_index.insert(p.pawn.id, entity);
    }
    // Spawn items
    for it in snapshot.items.iter() {
        let entity = commands
            .spawn((
                crate::WorldTag,
                Name::new(format!("Item#{}", it.item.id.0)),
                it.item.clone(),
                it.pos,
            ))
            .id();
        item_index.insert(it.item.id, entity);
    }
    // Spawn zones
    for z in snapshot.zones.iter() {
        let entity = commands
            .spawn((
                crate::WorldTag,
                Name::new(format!("Zone#{}", z.zone.id.0)),
                z.zone.clone(),
            ))
            .id();
        zone_index.insert(z.zone.id, entity);
    }
}

#[cfg(test)]
mod tests {
    use simkit_core::{grid::TileId, ids::IdIndex, Playback};

    use super::*;
    use crate::model::components::{Item, Pawn};

    #[derive(Resource)]
    struct TestSnap(WorldSnapshot);

    fn sys_load_from_snap(
        mut commands: Commands,
        mut pawn_idx: ResMut<IdIndex<PawnId>>,
        mut item_idx: ResMut<IdIndex<ItemId>>,
        mut zone_idx: ResMut<IdIndex<ZoneId>>,
        snap: Res<TestSnap>,
    ) {
        load_world_snapshot(
            &mut commands,
            &mut pawn_idx,
            &mut item_idx,
            &mut zone_idx,
            &snap.0,
        );
    }

    #[test]
    fn world_snapshot_round_trip() {
        let snap = WorldSnapshot {
            tick: 7,
            scenario_seed: Some(42),
            pawns: vec![
                PawnEntry {
                    pawn: Pawn { id: PawnId(1) },
                    pos: TileId { x: 2, y: 3 },
                },
                PawnEntry {
                    pawn: Pawn { id: PawnId(1000) },
                    pos: TileId { x: 4, y: 5 },
                },
            ],
            items: vec![ItemEntry {
                item: Item {
                    id: ItemId(2000),
                    kind: "Grain".into(),
                    qty: 5,
                },
                pos: TileId { x: 1, y: 1 },
            }],
            zones: vec![ZoneEntry {
                zone: Zone {
                    id: ZoneId(3000),
                    kind: "Stockpile".into(),
                    rect: ((TileId { x: 0, y: 0 }), (TileId { x: 2, y: 2 })),
                    filters: vec![],
                },
            }],
        };

        let mut app = App::new();
        app.init_resource::<IdIndex<PawnId>>()
            .init_resource::<IdIndex<ItemId>>()
            .init_resource::<IdIndex<ZoneId>>()
            .insert_resource(TestSnap(snap.clone()))
            .insert_resource(Playback {
                tick: simkit_core::Tick(snap.tick),
                ..Default::default()
            })
            .add_systems(Startup, sys_load_from_snap);

        app.update();

        // Re-extract
        let world = app.world_mut();
        let mut pawn_q = world.query::<(&Pawn, &TileId)>();
        let mut item_q = world.query::<(&Item, &TileId)>();
        let mut zone_q = world.query::<&Zone>();

        let pawns_vec: Vec<_> = pawn_q.iter(world).map(|(p, pos)| (*p, *pos)).collect();
        let items_vec: Vec<_> = item_q
            .iter(world)
            .map(|(it, pos)| (it.clone(), *pos))
            .collect();
        let zones_vec: Vec<_> = zone_q.iter(world).cloned().collect();

        let playback = *world.resource::<Playback>();
        let new_snap = build_world_snapshot(
            &playback,
            snap.scenario_seed,
            &pawns_vec,
            &items_vec,
            &zones_vec,
        );

        assert_eq!(snap.tick, new_snap.tick);
        assert_eq!(snap.scenario_seed, new_snap.scenario_seed);
        assert_eq!(snap.pawns, new_snap.pawns);
        assert_eq!(snap.items, new_snap.items);
        assert_eq!(snap.zones, new_snap.zones);
    }
}
