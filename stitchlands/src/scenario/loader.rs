use std::fs;

use bevy::prelude::*;
use rand::{rngs::SmallRng, Rng, SeedableRng};

use crate::{CliOptions, RngResource};

use super::model::{Item, Pawn, Position, ScenarioDef, TilePos, Zone};
use simkit_core::ids::{IdAllocator, IdIndex, ItemId, PawnId, ZoneId};

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct LoadedScenarioMeta {
    pub sim_seed: Option<u64>,
}

pub fn load_scenario(
    mut commands: Commands,
    cli: Option<Res<CliOptions>>,
    mut rng: ResMut<RngResource>,
    mut pawn_alloc: ResMut<IdAllocator<PawnId>>,
    mut pawn_index: ResMut<IdIndex<PawnId>>,
    mut item_alloc: ResMut<IdAllocator<ItemId>>,
    mut item_index: ResMut<IdIndex<ItemId>>,
    mut zone_alloc: ResMut<IdAllocator<ZoneId>>,
    mut zone_index: ResMut<IdIndex<ZoneId>>,
) {
    // Resources provided by plugin init
    let scenario_opt = cli
        .as_deref()
        .and_then(|c| c.scenario.as_ref()).cloned();

    // Parse editable ScenarioDef from RON
    let scenario_def: ScenarioDef = if let Some(path) = scenario_opt {
        let s = fs::read_to_string(&path).expect("read scenario");
        ron::de::from_str::<ScenarioDef>(&s).expect("parse scenario RON")
    } else {
        // Minimal default scenario for dev
        ScenarioDef {
            sim_seed: None,
            map: Default::default(),
            pawns: Vec::new(),
            items: Vec::new(),
            zones: Vec::new(),
            designations: Vec::new(),
            defaults: None,
        }
    };

    // Seed RNG for sim runtime using scenario or CLI
    let fallback_seed = cli.as_deref().map(|c| c.seed).unwrap_or(1);
    let sim_seed = scenario_def.sim_seed.unwrap_or(fallback_seed);
    rng.0 = SmallRng::seed_from_u64(sim_seed);
    commands.insert_resource(LoadedScenarioMeta {
        sim_seed: Some(sim_seed),
    });

    // Helpers
    let map_size = scenario_def.map.size;
    let mut next_rand_pos = || TilePos {
        x: rng.0.gen_range(0..map_size.x as i32),
        y: rng.0.gen_range(0..map_size.y as i32),
    };
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

    use std::collections::HashSet;
    let mut used_pawn_positions: HashSet<(i32, i32)> = HashSet::new();
    let mut used_item_positions: HashSet<(i32, i32)> = HashSet::new();
    let mut unique_pos = |
        used: &mut HashSet<(i32, i32)>,
        mut pos: TilePos,
        gen: &mut dyn FnMut() -> TilePos,
    | {
        let mut tries = 0;
        let max_tries = 1000;
        while used.contains(&(pos.x, pos.y)) && tries < max_tries {
            pos = gen();
            tries += 1;
        }
        used.insert((pos.x, pos.y));
        pos
    };

    // Track maxima of provided IDs to bump allocators after spawning provided ones
    let max_pawn_provided = scenario_def
        .pawns
        .iter()
        .filter_map(|p| p.id)
        .max();
    let max_item_provided = scenario_def.items.iter().filter_map(|i| i.id).max();
    let max_zone_provided = scenario_def.zones.iter().filter_map(|z| z.id).max();

    // Spawn pawns
    for (i, p) in scenario_def.pawns.iter().enumerate() {
        let typed = pawn_alloc.assign(p.id.map(PawnId));
        let name = p
            .name
            .clone()
            .unwrap_or_else(|| format!("Pawn{}", i + 1));
        let pos = Position(match p.pos {
            Some(pos) => unique_pos(&mut used_pawn_positions, pos, &mut next_rand_pos),
            None => unique_pos(&mut used_pawn_positions, next_rand_pos(), &mut next_rand_pos),
        });
        let entity = commands
            .spawn((crate::WorldTag, Name::new(name), Pawn(typed), pos))
            .id();
        pawn_index.insert(typed, entity);
        // p.needs and p.priorities retained for future use
    }

    // Ensure pawn allocator does not reuse provided IDs
    if let Some(max) = max_pawn_provided {
        if pawn_alloc.next <= max {
            pawn_alloc.reset(max + 1);
        }
    }

    // Spawn items
    for it in scenario_def.items.iter() {
        let typed = item_alloc.assign(it.id.map(ItemId));
        let pos = Position(match it.pos {
            Some(pos) => unique_pos(&mut used_item_positions, pos, &mut next_rand_pos),
            None => unique_pos(&mut used_item_positions, next_rand_pos(), &mut next_rand_pos),
        });
        let entity = commands
            .spawn((
                crate::WorldTag,
                Name::new(format!("Item#{}", typed.0)),
                Item {
                    id: typed,
                    kind: it.kind.clone(),
                    qty: it.qty,
                },
                pos,
            ))
            .id();
        item_index.insert(typed, entity);
    }

    if let Some(max) = max_item_provided {
        if item_alloc.next <= max {
            item_alloc.reset(max + 1);
        }
    }

    // Spawn zones
    for z in scenario_def.zones.iter() {
        let typed = zone_alloc.assign(z.id.map(ZoneId));
        let rect = match z.rect {
            Some((a, b)) => norm_rect(a, b),
            None => {
                let p = next_rand_pos();
                (p, p)
            }
        };
        let entity = commands
            .spawn((
                crate::WorldTag,
                Name::new(format!("Zone#{}", typed.0)),
                Zone {
                    id: typed,
                    kind: z.kind.clone(),
                    rect,
                    filters: z.filters.clone(),
                },
            ))
            .id();
        zone_index.insert(typed, entity);
    }

    if let Some(max) = max_zone_provided {
        if zone_alloc.next <= max {
            zone_alloc.reset(max + 1);
        }
    }
}

pub fn load_scenario_if_headless(
    cli: Option<Res<CliOptions>>,
    commands: Commands,
    rng: ResMut<RngResource>,
    pawn_alloc: ResMut<IdAllocator<PawnId>>,
    pawn_index: ResMut<IdIndex<PawnId>>,
    item_alloc: ResMut<IdAllocator<ItemId>>,
    item_index: ResMut<IdIndex<ItemId>>,
    zone_alloc: ResMut<IdAllocator<ZoneId>>,
    zone_index: ResMut<IdIndex<ZoneId>>,
) {
    let Some(cli) = cli else { return };
    if cli.mode != crate::RunMode::Headless {
        return;
    }
    load_scenario(
        commands,
        Some(cli),
        rng,
        pawn_alloc,
        pawn_index,
        item_alloc,
        item_index,
        zone_alloc,
        zone_index,
    );
}

// spawn helpers removed; completion occurs inline during load
