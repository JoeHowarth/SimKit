use std::fs;

use bevy::prelude::*;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use simkit_core::{
    grid::{index::TileMapIndex, GridConfig, TileId},
    ids::{IdAllocator, IdIndex},
};

use super::model::ScenarioDef;
use crate::{
    model::{
        components::{Item, Pawn, Zone},
        ids::{ItemId, PawnId, ZoneId},
    },
    snapshot::load_world_snapshot,
    tasks::{Designation, Needs, TaskRef},
    world::WorldGrid,
    CliOptions,
    RngResource,
};

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
    let scenario_opt = cli.as_deref().and_then(|c| c.scenario.as_ref()).cloned();

    // If snapshot path provided, load world from snapshot RON and return
    if let Some(snap_path) = cli.as_deref().and_then(|c| c.snapshot.as_ref()).cloned() {
        let s = fs::read_to_string(&snap_path).expect("read snapshot");
        let snap: crate::snapshot::WorldSnapshot =
            ron::de::from_str(&s).expect("parse snapshot RON");
        // Seed RNG based on snapshot scenario_seed or CLI fallback
        let fallback_seed = cli.as_deref().map(|c| c.seed).unwrap_or(1);
        let sim_seed = snap.scenario_seed.unwrap_or(fallback_seed);
        rng.0 = SmallRng::seed_from_u64(sim_seed);
        commands.insert_resource(LoadedScenarioMeta {
            sim_seed: Some(sim_seed),
        });
        // Bump allocators past max ids present
        if let Some(max) = snap.pawns.iter().map(|p| p.pawn.id.0).max() {
            if pawn_alloc.next <= max {
                pawn_alloc.reset(max + 1);
            }
        }
        if let Some(max) = snap.items.iter().map(|i| i.item.id.0).max() {
            if item_alloc.next <= max {
                item_alloc.reset(max + 1);
            }
        }
        if let Some(max) = snap.zones.iter().map(|z| z.zone.id.0).max() {
            if zone_alloc.next <= max {
                zone_alloc.reset(max + 1);
            }
        }
        load_world_snapshot(
            &mut commands,
            &mut pawn_index,
            &mut item_index,
            &mut zone_index,
            &snap,
        );
        return;
    }

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

    load_scenario_from_def(
        commands,
        rng,
        pawn_alloc,
        pawn_index,
        item_alloc,
        item_index,
        zone_alloc,
        zone_index,
        scenario_def,
        cli.as_deref().map(|c| c.seed).unwrap_or(1),
    );
}

#[allow(clippy::too_many_arguments)]
pub fn load_scenario_from_def(
    mut commands: Commands,
    mut rng: ResMut<RngResource>,
    mut pawn_alloc: ResMut<IdAllocator<PawnId>>,
    mut pawn_index: ResMut<IdIndex<PawnId>>,
    mut item_alloc: ResMut<IdAllocator<ItemId>>,
    mut item_index: ResMut<IdIndex<ItemId>>,
    mut zone_alloc: ResMut<IdAllocator<ZoneId>>,
    mut zone_index: ResMut<IdIndex<ZoneId>>,
    scenario_def: ScenarioDef,
    fallback_seed: u64,
) {
    // Seed RNG for sim runtime using scenario or CLI
    let sim_seed = scenario_def.sim_seed.unwrap_or(fallback_seed);
    rng.0 = SmallRng::seed_from_u64(sim_seed);
    commands.insert_resource(LoadedScenarioMeta {
        sim_seed: Some(sim_seed),
    });

    let map_size = scenario_def.map.size;

    // Build and insert world grid from map, and prepare tile indices
    let world_grid = WorldGrid::from_map(&scenario_def.map);
    let cfg = GridConfig {
        width: map_size.x,
        height: map_size.y,
    };
    let mut pawn_tile_index: TileMapIndex<PawnId> = TileMapIndex::new(cfg);
    let mut item_tile_index: TileMapIndex<ItemId> = TileMapIndex::new(cfg);

    // Pawns
    spawn_pawns_from_def(
        &mut commands,
        &mut rng.0,
        map_size,
        &mut pawn_alloc,
        &mut pawn_index,
        &scenario_def.pawns,
        &mut pawn_tile_index,
    );
    // Items
    spawn_items_from_def(
        &mut commands,
        &mut rng.0,
        map_size,
        &mut item_alloc,
        &mut item_index,
        &scenario_def.items,
        &mut item_tile_index,
    );
    // Zones
    spawn_zones_from_def(
        &mut commands,
        &mut rng.0,
        map_size,
        &mut zone_alloc,
        &mut zone_index,
        &scenario_def.zones,
    );

    // Designations
    spawn_designations_from_def(&mut commands, &scenario_def.designations);

    // Finally insert resources
    commands.insert_resource(world_grid);
    commands.insert_resource(pawn_tile_index);
    commands.insert_resource(item_tile_index);
}

fn rand_pos(rng: &mut SmallRng, size: super::model::MapSize) -> TileId {
    TileId {
        x: rng.gen_range(0..size.x as i32),
        y: rng.gen_range(0..size.y as i32),
    }
}

fn clamp_pos(mut p: TileId, size: super::model::MapSize) -> TileId {
    if p.x < 0 {
        p.x = 0
    };
    if p.y < 0 {
        p.y = 0
    };
    if p.x >= size.x as i32 {
        p.x = size.x as i32 - 1
    };
    if p.y >= size.y as i32 {
        p.y = size.y as i32 - 1
    };
    p
}

fn norm_rect(a: TileId, b: TileId, size: super::model::MapSize) -> (TileId, TileId) {
    let a = clamp_pos(a, size);
    let b = clamp_pos(b, size);
    let minx = a.x.min(b.x);
    let miny = a.y.min(b.y);
    let maxx = a.x.max(b.x);
    let maxy = a.y.max(b.y);
    (TileId { x: minx, y: miny }, TileId { x: maxx, y: maxy })
}

fn unique_pos(
    used: &mut std::collections::HashSet<(i32, i32)>,
    mut pos: TileId,
    gen: &mut dyn FnMut() -> TileId,
) -> TileId {
    let mut tries = 0;
    let max_tries = 1000;
    while used.contains(&(pos.x, pos.y)) && tries < max_tries {
        pos = gen();
        tries += 1;
    }
    used.insert((pos.x, pos.y));
    pos
}

fn bump_alloc_after_provided<T: simkit_core::ids::SimId>(
    alloc: &mut IdAllocator<T>,
    max_provided: Option<u64>,
) {
    if let Some(max) = max_provided {
        if alloc.next <= max {
            alloc.reset(max + 1);
        }
    }
}

fn spawn_pawns_from_def(
    commands: &mut Commands,
    rng: &mut SmallRng,
    map_size: super::model::MapSize,
    alloc: &mut IdAllocator<PawnId>,
    index: &mut IdIndex<PawnId>,
    pawns: &[super::model::PawnDef],
    tile_index: &mut TileMapIndex<PawnId>,
) {
    use std::collections::HashSet;
    let mut used_positions: HashSet<(i32, i32)> = HashSet::new();
    let max_provided = pawns.iter().filter_map(|p| p.id).max();
    let mut gen = || rand_pos(rng, map_size);
    for (i, p) in pawns.iter().enumerate() {
        let typed = alloc.assign(p.id.map(PawnId));
        let name = p.name.clone().unwrap_or_else(|| format!("Pawn{}", i + 1));
        let pos = match p.pos {
            Some(pos) => unique_pos(&mut used_positions, pos, &mut gen),
            None => unique_pos(&mut used_positions, gen(), &mut gen),
        };
        let needs = Needs {
            hunger: p.needs.hunger,
            rest: p.needs.rest,
        };
        let entity = commands
            .spawn((
                crate::WorldTag,
                Name::new(name),
                Pawn { id: typed },
                needs,
                pos,
            ))
            .id();
        index.insert(typed, entity);

        // Tile index mark
        tile_index.move_id(None, pos, typed);
    }
    bump_alloc_after_provided(alloc, max_provided);
}

fn spawn_items_from_def(
    commands: &mut Commands,
    rng: &mut SmallRng,
    map_size: super::model::MapSize,
    alloc: &mut IdAllocator<ItemId>,
    index: &mut IdIndex<ItemId>,
    items: &[super::model::ItemDef],
    tile_index: &mut TileMapIndex<ItemId>,
) {
    use std::collections::HashSet;
    let mut used_positions: HashSet<(i32, i32)> = HashSet::new();
    let max_provided = items.iter().filter_map(|i| i.id).max();
    let mut gen = || rand_pos(rng, map_size);
    for it in items.iter() {
        let typed = alloc.assign(it.id.map(ItemId));
        let pos = match it.pos {
            Some(pos) => unique_pos(&mut used_positions, pos, &mut gen),
            None => unique_pos(&mut used_positions, gen(), &mut gen),
        };
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
        index.insert(typed, entity);

        // Tile index mark
        tile_index.move_id(None, pos, typed);
    }
    bump_alloc_after_provided(alloc, max_provided);
}

fn spawn_zones_from_def(
    commands: &mut Commands,
    rng: &mut SmallRng,
    map_size: super::model::MapSize,
    alloc: &mut IdAllocator<ZoneId>,
    index: &mut IdIndex<ZoneId>,
    zones: &[super::model::ZoneDef],
) {
    let max_provided = zones.iter().filter_map(|z| z.id).max();
    for z in zones.iter() {
        let typed = alloc.assign(z.id.map(ZoneId));
        let rect = match z.rect {
            Some((a, b)) => norm_rect(a, b, map_size),
            None => {
                let p = rand_pos(rng, map_size);
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
        index.insert(typed, entity);
    }
    bump_alloc_after_provided(alloc, max_provided);
}

fn spawn_designations_from_def(
    commands: &mut Commands,
    designations: &[super::model::DesignationDef],
) {
    for d in designations.iter() {
        match d {
            super::model::DesignationDef::Harvest(tile) => {
                let name = format!("Designation(Harvest @{}, {})", tile.x, tile.y);
                commands.spawn((
                    crate::WorldTag,
                    Name::new(name),
                    Designation::Harvest(*tile),
                    TaskRef(None),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::model;

    #[derive(Resource)]
    struct TestScenario(pub ScenarioDef);

    fn sys_load_from_def(
        commands: Commands,
        rng: ResMut<RngResource>,
        pawn_alloc: ResMut<IdAllocator<PawnId>>,
        pawn_index: ResMut<IdIndex<PawnId>>,
        item_alloc: ResMut<IdAllocator<ItemId>>,
        item_index: ResMut<IdIndex<ItemId>>,
        zone_alloc: ResMut<IdAllocator<ZoneId>>,
        zone_index: ResMut<IdIndex<ZoneId>>,
        scn: Res<TestScenario>,
    ) {
        load_scenario_from_def(
            commands,
            rng,
            pawn_alloc,
            pawn_index,
            item_alloc,
            item_index,
            zone_alloc,
            zone_index,
            scn.0.clone(),
            1,
        );
    }

    #[test]
    fn scenario_loading_completes_optional_fields() {
        let def = ScenarioDef {
            sim_seed: Some(123),
            map: model::MapDef {
                size: model::MapSize { x: 4, y: 4 },
                tiles: vec![],
            },
            pawns: vec![
                model::PawnDef {
                    id: Some(10),
                    name: Some("Ada".into()),
                    pos: None,
                    needs: model::NeedsDef {
                        hunger: 0.5,
                        rest: 0.8,
                    },
                    priorities: Default::default(),
                },
                model::PawnDef {
                    id: None,
                    name: None,
                    pos: None,
                    needs: model::NeedsDef {
                        hunger: 0.6,
                        rest: 0.7,
                    },
                    priorities: Default::default(),
                },
            ],
            items: vec![model::ItemDef {
                id: None,
                kind: "Grain".into(),
                qty: 5,
                pos: None,
            }],
            zones: vec![
                model::ZoneDef {
                    id: None,
                    kind: "Stockpile".into(),
                    rect: Some((TileId { x: 3, y: 3 }, TileId { x: 1, y: 2 })),
                    filters: vec![],
                },
                model::ZoneDef {
                    id: None,
                    kind: "Dump".into(),
                    rect: None,
                    filters: vec![],
                },
            ],
            designations: vec![],
            defaults: None,
        };

        let mut app = App::new();
        app.init_resource::<RngResource>()
            .init_resource::<IdAllocator<PawnId>>()
            .init_resource::<IdIndex<PawnId>>()
            .init_resource::<IdAllocator<ItemId>>()
            .init_resource::<IdIndex<ItemId>>()
            .init_resource::<IdAllocator<ZoneId>>()
            .init_resource::<IdIndex<ZoneId>>()
            .insert_resource(TestScenario(def))
            .add_systems(Startup, sys_load_from_def);

        app.update();

        // Validate pawns
        let world = app.world_mut();
        let mut pawn_q = world.query::<(&Pawn, &TileId, &Name)>();
        let pawns: Vec<_> = pawn_q.iter(world).collect();
        assert_eq!(pawns.len(), 2);
        let mut ids: Vec<_> = pawns.iter().map(|(p, _, _)| p.id.0).collect();
        ids.sort_unstable();
        assert_eq!(ids[0], 10);
        assert_eq!(ids[1], 1000);
        for (_, pos, _) in pawns.iter() {
            assert!(pos.x >= 0 && pos.x < 4 && pos.y >= 0 && pos.y < 4);
        }
        // Names contain provided and fallback
        let names: Vec<String> = pawns.iter().map(|(_, _, n)| n.to_string()).collect();
        assert!(names.contains(&"Ada".to_string()));
        assert!(names.iter().any(|s| s == "Pawn2"));

        // Validate items
        let mut item_q = world.query::<(&Item, &TileId)>();
        let items: Vec<_> = item_q.iter(world).collect();
        assert_eq!(items.len(), 1);
        assert!(items[0].0.id.0 >= 1000);
        let ipos = items[0].1;
        assert!(ipos.x >= 0 && ipos.x < 4 && ipos.y >= 0 && ipos.y < 4);

        // Validate zones normalization / default
        let mut zone_q = world.query::<&Zone>();
        let zones: Vec<_> = zone_q.iter(world).collect();
        assert_eq!(zones.len(), 2);
        assert!(zones
            .iter()
            .any(|z| z.rect.0.x <= z.rect.1.x && z.rect.0.y <= z.rect.1.y));
        assert!(zones
            .iter()
            .any(|z| z.rect.0.x == z.rect.1.x && z.rect.0.y == z.rect.1.y));

        // Allocators bumped
        let pawn_alloc = world.resource::<IdAllocator<PawnId>>();
        let item_alloc = world.resource::<IdAllocator<ItemId>>();
        let zone_alloc = world.resource::<IdAllocator<ZoneId>>();
        assert!(pawn_alloc.peek_next() >= 1001);
        assert!(item_alloc.peek_next() >= 1001);
        assert!(zone_alloc.peek_next() >= 1001);
    }
}
