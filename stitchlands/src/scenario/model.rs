use bevy::prelude::*;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use simkit_core::ids::{ItemId, PawnId, ZoneId};
use std::collections::{HashMap, HashSet};

// Basic map/tiles; unused in 0.b beyond size
// Legacy struct removed; on-disk files are ScenarioDef (serde-renamed to Scenario)

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DefaultsDef {}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct MapDef {
    #[serde(default)]
    pub size: MapSize,
    #[serde(default)]
    pub tiles: Vec<TileDef>,
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

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct TileDef {
    pub pos: TilePos,
    #[serde(default)]
    pub walkable: bool,
    #[serde(default)]
    pub terrain: Terrain,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
pub enum Terrain {
    #[default]
    Grass,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PawnDef {
    pub id: Option<u64>,
    pub name: Option<String>,
    pub pos: Option<TilePos>,
    #[serde(default)]
    pub needs: NeedsDef,
    #[serde(default)]
    pub priorities: HashMap<String, i32>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
pub struct NeedsDef {
    pub hunger: f32,
    pub rest: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ItemDef {
    pub id: Option<u64>,
    pub kind: String,
    pub qty: u32,
    pub pos: Option<TilePos>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ZoneDef {
    pub id: Option<u64>,
    pub kind: String,
    pub rect: Option<(TilePos, TilePos)>,
    #[serde(default)]
    pub filters: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum DesignationDef {
    Harvest(TilePos),
}

// Runtime components for spawned entities (minimal)
#[derive(Component, Debug, Clone, Copy, Serialize)]
pub struct Pawn(pub PawnId);

#[derive(Component, Debug, Clone, Copy, Serialize)]
pub struct Position(pub TilePos);

// Minimal item/zones components for spawned entities
#[derive(Component, Debug, Clone, Serialize)]
pub struct Item {
    pub id: ItemId,
    pub kind: String,
    pub qty: u32,
}

#[derive(Component, Debug, Clone, Serialize)]
pub struct Zone {
    pub id: ZoneId,
    pub kind: String,
    pub rect: (TilePos, TilePos),
    pub filters: Vec<String>,
}

// Editable form used in RON files (allows many omissions)
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename = "Scenario")] // on-disk name is simply `Scenario(...)`
pub struct ScenarioDef {
    pub sim_seed: Option<u64>,
    #[serde(default)]
    pub map: MapDefDef,
    #[serde(default)]
    pub pawns: Vec<PawnDef>,
    #[serde(default)]
    pub items: Vec<ItemDef>,
    #[serde(default)]
    pub zones: Vec<ZoneDef>,
    #[serde(default)]
    pub designations: Vec<DesignationDef>,
    pub defaults: Option<DefaultsDef>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct MapDefDef {
    #[serde(default)]
    pub size: MapSize,
    #[serde(default)]
    pub tiles: Vec<TileDef>,
}

// Fully-populated scenario used by the loader after fill-in
#[derive(Debug, Clone, Serialize)]
pub struct Scenario {
    pub sim_seed: u64,
    pub map: MapDef,
    pub pawns: Vec<PawnComplete>,
    pub items: Vec<ItemComplete>,
    pub zones: Vec<ZoneComplete>,
    pub designations: Vec<DesignationDef>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PawnComplete {
    pub name: String,
    pub pawn: Pawn,
    pub position: Position,
    pub needs: NeedsDef,
    pub priorities: HashMap<String, i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemComplete {
    pub item: Item,
    pub position: Position,
}

#[derive(Debug, Clone, Serialize)]
pub struct ZoneComplete {
    pub zone: Zone,
}

pub fn complete_scenario(def: &ScenarioDef, fallback_seed: u64) -> Scenario {
    let sim_seed = def.sim_seed.unwrap_or(fallback_seed);
    let mut rng = SmallRng::seed_from_u64(sim_seed);

    // Map defaults
    let map = def.map.clone();
    let map_size = map.size;
    let map_complete = MapDef {
        size: map_size,
        tiles: map.tiles,
    };

    // Helper to generate a bounded random TilePos
    let mut next_rand_pos = || TilePos {
        x: rng.gen_range(0..map_size.x as i32),
        y: rng.gen_range(0..map_size.y as i32),
    };

    // Canonicalize and clamp a rect to map bounds
    let clamp = |mut p: TilePos| -> TilePos {
        if p.x < 0 {
            p.x = 0
        };
        if p.y < 0 {
            p.y = 0
        };
        if p.x >= map_size.x as i32 {
            p.x = map_size.x as i32 - 1
        };
        if p.y >= map_size.y as i32 {
            p.y = map_size.y as i32 - 1
        };
        p
    };
    let norm_rect = |a: TilePos, b: TilePos| -> (TilePos, TilePos) {
        let a = clamp(a);
        let b = clamp(b);
        let minx = a.x.min(b.x);
        let miny = a.y.min(b.y);
        let maxx = a.x.max(b.x);
        let maxy = a.y.max(b.y);
        (TilePos { x: minx, y: miny }, TilePos { x: maxx, y: maxy })
    };

    // Deterministic ID completion: fill missing IDs with the lowest positive integer not yet used.
    let complete_ids = |given: &mut Vec<Option<u64>>| {
        let mut used: HashSet<u64> = given.iter().filter_map(|x| *x).collect();
        for id in given.iter_mut() {
            if id.is_none() {
                let mut candidate = 1_000u64;
                while used.contains(&candidate) {
                    candidate += 1;
                }
                used.insert(candidate);
                *id = Some(candidate);
            }
        }
    };

    // Pawns
    let mut pawn_ids: Vec<Option<u64>> = def.pawns.iter().map(|p| p.id).collect();
    complete_ids(&mut pawn_ids);
    // Enforce uniqueness of generated pawn/item positions (best-effort with cap)
    let mut used_pawn_positions: HashSet<(i32, i32)> = HashSet::new();
    let mut used_item_positions: HashSet<(i32, i32)> = HashSet::new();
    let unique_pos =
        |used: &mut HashSet<(i32, i32)>, mut pos: TilePos, gen: &mut dyn FnMut() -> TilePos| {
            let mut tries = 0;
            let max_tries = 1000;
            while used.contains(&(pos.x, pos.y)) && tries < max_tries {
                pos = gen();
                tries += 1;
            }
            used.insert((pos.x, pos.y));
            pos
        };

    let pawns: Vec<PawnComplete> = def
        .pawns
        .iter()
        .enumerate()
        .map(|(i, p)| PawnComplete {
            name: p.name.clone().unwrap_or_else(|| format!("Pawn{}", i + 1)),
            pawn: Pawn(PawnId(pawn_ids[i].unwrap())),
            position: Position({
                let base = p.pos.unwrap_or_else(&mut next_rand_pos);
                unique_pos(&mut used_pawn_positions, base, &mut next_rand_pos)
            }),
            needs: p.needs,
            priorities: p.priorities.clone(),
        })
        .collect();

    // Items
    let mut item_ids: Vec<Option<u64>> = def.items.iter().map(|it| it.id).collect();
    complete_ids(&mut item_ids);
    let items: Vec<ItemComplete> = def
        .items
        .iter()
        .enumerate()
        .map(|(i, it)| ItemComplete {
            item: Item {
                id: ItemId(item_ids[i].unwrap()),
                kind: it.kind.clone(),
                qty: it.qty,
            },
            position: Position({
                let base = it.pos.unwrap_or_else(&mut next_rand_pos);
                unique_pos(&mut used_item_positions, base, &mut next_rand_pos)
            }),
        })
        .collect();

    // Zones
    let mut zone_ids: Vec<Option<u64>> = def.zones.iter().map(|z| z.id).collect();
    complete_ids(&mut zone_ids);
    let zones: Vec<ZoneComplete> = def
        .zones
        .iter()
        .enumerate()
        .map(|(i, z)| ZoneComplete {
            zone: Zone {
                id: ZoneId(zone_ids[i].unwrap()),
                kind: z.kind.clone(),
                rect: {
                    let (a, b) = z.rect.unwrap_or_else(|| {
                        let p = next_rand_pos();
                        (p, p)
                    });
                    let (lo, hi) = norm_rect(a, b);
                    (lo, hi)
                },
                filters: z.filters.clone(),
            },
        })
        .collect();

    Scenario {
        sim_seed,
        map: map_complete,
        pawns,
        items,
        zones,
        designations: def.designations.clone(),
    }
}

// no legacy converter retained

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_missing_pawn_pos_and_ids_deterministically() {
        let def = ScenarioDef {
            sim_seed: Some(123),
            map: MapDefDef {
                size: MapSize { x: 8, y: 8 },
                tiles: vec![],
            },
            pawns: vec![
                PawnDef {
                    id: Some(10),
                    name: None,
                    pos: None,
                    needs: NeedsDef {
                        hunger: 0.5,
                        rest: 0.9,
                    },
                    priorities: HashMap::new(),
                },
                PawnDef {
                    id: None,
                    name: None,
                    pos: None,
                    needs: NeedsDef {
                        hunger: 0.5,
                        rest: 0.9,
                    },
                    priorities: HashMap::new(),
                },
            ],
            items: vec![],
            zones: vec![],
            designations: vec![],
            defaults: None,
        };
        let comp1 = complete_scenario(&def, 1);
        let comp2 = complete_scenario(&def, 1);

        assert_eq!(comp1.sim_seed, 123);
        assert_eq!(comp2.sim_seed, 123);

        assert_eq!(comp1.pawns.len(), 2);
        assert_eq!(comp1.pawns[0].pawn.0 .0, 10);
        assert_eq!(comp1.pawns[1].pawn.0 .0, 1000); // next lowest unused given allocator base
                                                    // Deterministic positions with same seed
        assert_eq!(comp1.pawns[0].position.0.x, comp2.pawns[0].position.0.x);
        assert_eq!(comp1.pawns[0].position.0.y, comp2.pawns[0].position.0.y);
        assert!(comp1.pawns[0].position.0.x >= 0 && comp1.pawns[0].position.0.x < 8);
        assert!(comp1.pawns[0].position.0.y >= 0 && comp1.pawns[0].position.0.y < 8);

        // Changing seed changes positions when missing
        let comp3 = complete_scenario(
            &ScenarioDef {
                sim_seed: Some(124),
                ..def.clone()
            },
            1,
        );
        assert_ne!(
            (comp1.pawns[0].position.0.x, comp1.pawns[0].position.0.y),
            (comp3.pawns[0].position.0.x, comp3.pawns[0].position.0.y)
        );
    }

    #[test]
    fn fills_item_position_and_zone_rect_and_normalizes() {
        let def = ScenarioDef {
            sim_seed: Some(42),
            map: MapDefDef {
                size: MapSize { x: 4, y: 4 },
                tiles: vec![],
            },
            pawns: vec![],
            items: vec![ItemDef {
                id: None,
                kind: "Grain".into(),
                qty: 5,
                pos: None,
            }],
            zones: vec![
                ZoneDef {
                    id: None,
                    kind: "Stockpile".into(),
                    rect: Some((TilePos { x: 3, y: 3 }, TilePos { x: 1, y: 2 })),
                    filters: vec![],
                },
                ZoneDef {
                    id: None,
                    kind: "Dump".into(),
                    rect: None,
                    filters: vec![],
                },
            ],
            designations: vec![],
            defaults: None,
        };
        let comp = complete_scenario(&def, 1);
        assert_eq!(comp.items.len(), 1);
        let itpos = comp.items[0].position.0;
        assert!(itpos.x >= 0 && itpos.x < 4 && itpos.y >= 0 && itpos.y < 4);

        assert_eq!(comp.zones.len(), 2);
        let z0 = &comp.zones[0].zone.rect; // normalized
        assert!(z0.0.x <= z0.1.x && z0.0.y <= z0.1.y);
        // clamped (all within bounds)
        assert!(z0.0.x >= 0 && z0.1.x < 4 && z0.0.y >= 0 && z0.1.y < 4);

        let z1 = &comp.zones[1].zone.rect; // generated 1x1
        assert_eq!(z1.0.x, z1.1.x);
        assert_eq!(z1.0.y, z1.1.y);
    }
}
