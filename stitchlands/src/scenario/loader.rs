use std::{fs, str::FromStr};

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use simkit_core::{
    grid::{index::TileMapIndex, GridConfig, TileId},
    ids::IdIndex,
};

use super::model::{MapSize, ScenarioDef};
use crate::{
    model::{components::*, ids::*},
    snapshot::load_world_snapshot,
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
    mut pawn_index: ResMut<IdIndex<PawnId>>,
    mut item_index: ResMut<IdIndex<ItemId>>,
    mut fixture_index: ResMut<IdIndex<FixtureId>>,
    mut task_index: ResMut<IdIndex<TaskId>>,
) {
    // Resources provided by plugin init

    // If snapshot path provided, load world from snapshot RON and return
    if let Some(snap_path) =
        cli.as_deref().and_then(|c| c.snapshot.as_ref()).cloned()
    {
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

        for pawn in snap.pawns.iter() {
            pawn_index.register(pawn.pawn.id);
        }

        for item in snap.items.iter() {
            item_index.register(item.item.id);
        }

        load_world_snapshot(
            &mut commands,
            &mut pawn_index,
            &mut item_index,
            &mut fixture_index,
            &mut task_index,
            &snap,
        );
        return;
    }

    let scenario_path = cli
        .as_deref()
        .and_then(|c| c.scenario.as_ref())
        .cloned()
        .expect("Either snapshot or scenario must be provided");

    // Parse editable ScenarioDef from RON
    let scenario_str =
        fs::read_to_string(&scenario_path).expect("read scenario");
    let scenario_def =
        ron::from_str(&scenario_str).expect("parse scenario RON");

    load_scenario_from_def(
        commands,
        rng,
        pawn_index,
        item_index,
        fixture_index,
        task_index,
        scenario_def,
        cli.as_deref().map(|c| c.seed).unwrap_or(1),
    );
}

#[allow(clippy::too_many_arguments)]
pub fn load_scenario_from_def(
    mut commands: Commands,
    mut rng: ResMut<RngResource>,
    mut pawn_index: ResMut<IdIndex<PawnId>>,
    mut item_index: ResMut<IdIndex<ItemId>>,
    mut fixture_index: ResMut<IdIndex<FixtureId>>,
    task_index: ResMut<IdIndex<TaskId>>,
    scenario_def: ScenarioDef,
    fallback_seed: u64,
) {
    // Seed RNG for sim runtime using scenario or CLI
    let sim_seed = scenario_def.sim_seed.unwrap_or(fallback_seed);
    rng.0 = SmallRng::seed_from_u64(sim_seed);
    commands.insert_resource(LoadedScenarioMeta {
        sim_seed: Some(sim_seed),
    });

    // Make a mutable copy so we can materialize schematic positions
    let mut scenario_def = scenario_def;
    let map_size = scenario_def.map.size;

    // Pass 1: If a schematic is present, parse and copy positions into
    // existing pawn and fixture defs (by matching ids). Validate conflicts.
    if let Some(s) = &scenario_def.map.schematic {
        let (schem_pawns, schem_fixtures) = parse_schematic(s, map_size);

        // Validate schematic ids exist
        let pawn_ids: std::collections::HashSet<u64> = scenario_def
            .pawns
            .iter()
            .filter_map(|p| p.id)
            .collect();
        for id in schem_pawns.keys() {
            assert!(
                pawn_ids.contains(id),
                "Schematic references Pawn id {} not present in [pawns] list",
                id
            );
        }
        let fixture_ids: std::collections::HashSet<u64> = scenario_def
            .fixtures
            .iter()
            .filter_map(|f| f.id)
            .collect();
        for id in schem_fixtures.keys() {
            assert!(
                fixture_ids.contains(id),
                "Schematic references Fixture id {} not present in [fixtures] list",
                id
            );
        }

        // Apply pawn positions
        for p in scenario_def.pawns.iter_mut() {
            if let Some(id) = p.id {
                if let Some(pos) = schem_pawns.get(&id).copied() {
                    if let Some(explicit) = p.pos {
                        assert_eq!(
                            explicit, pos,
                            "Schematic position for Pawn id={} conflicts with explicit pos: {:?} vs {:?}",
                            id, pos, explicit
                        );
                    }
                    p.pos = Some(pos);
                }
            }
        }
        // Apply fixture positions
        for f in scenario_def.fixtures.iter_mut() {
            if let Some(id) = f.id {
                if let Some(pos) = schem_fixtures.get(&id).copied() {
                    if let Some(explicit) = f.pos {
                        assert_eq!(
                            explicit, pos,
                            "Schematic position for Fixture id={} conflicts with explicit pos: {:?} vs {:?}",
                            id, pos, explicit
                        );
                    }
                    f.pos = Some(pos);
                }
            }
        }
    }

    // Build and insert world grid from map, and prepare tile indices
    let world_grid = WorldGrid::from_map(&scenario_def.map);
    let cfg = GridConfig {
        width: map_size.x,
        height: map_size.y,
    };
    let mut pawn_tile_index: TileMapIndex<PawnId> = TileMapIndex::new(cfg);
    let mut item_tile_index: TileMapIndex<ItemId> = TileMapIndex::new(cfg);
    let mut fixture_tile_index: TileMapIndex<FixtureId> =
        TileMapIndex::new(cfg);

    // Pawns
    spawn_pawns_from_def(
        &mut commands,
        &mut rng.0,
        map_size,
        &mut pawn_index,
        &scenario_def.pawns,
        &mut pawn_tile_index,
        &mut item_index,
        &mut item_tile_index,
    );

    // Fixtures
    spawn_fixtures_from_def(
        &mut commands,
        &mut rng.0,
        map_size,
        &mut fixture_index,
        &scenario_def.fixtures,
        &mut fixture_tile_index,
        &mut item_index,
        &mut item_tile_index,
    );

    // Items on Ground
    // Place tile-defined items at their tile positions.
    let map_items: Vec<super::model::ItemDef> = scenario_def
        .map
        .tiles
        .iter()
        .filter_map(|t| {
            t.item.as_ref().map(|it| {
                let mut it2 = it.clone();
                it2.pos = Some(t.pos);
                it2
            })
        })
        .collect();
    spawn_items_from_def(
        &mut commands,
        &mut rng.0,
        map_size,
        &mut item_index,
        &map_items,
        &mut item_tile_index,
    );

    // Designations
    // spawn_designations_from_def(&mut commands, &scenario_def.tasks);

    // Finally insert resources
    commands.insert_resource(world_grid);
    commands.insert_resource(pawn_tile_index);
    commands.insert_resource(item_tile_index);
    commands.insert_resource(fixture_tile_index);
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

fn norm_rect(
    a: TileId,
    b: TileId,
    size: super::model::MapSize,
) -> (TileId, TileId) {
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

fn parse_schematic(
    s: &str,
    size: MapSize,
) -> (HashMap<u64, TileId>, HashMap<u64, TileId>) {
    let mut pawn_pos: HashMap<u64, TileId> = HashMap::new();
    let mut fixture_pos: HashMap<u64, TileId> = HashMap::new();

    let lines: Vec<&str> = s.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines.len() as u32,
        size.y,
        "schematic row count {} differs from map.size.y {}",
        lines.len(),
        size.y
    );

    // Track duplicates within each category by tile as well
    let mut seen_pawn_tiles: HashSet<(i32, i32)> = HashSet::new();
    let mut seen_fix_tiles: HashSet<(i32, i32)> = HashSet::new();

    for (y, line) in lines.iter().enumerate() {
        let cols: Vec<&str> = line.trim().split_whitespace().collect();
        assert_eq!(
            cols.len() as u32,
            size.x,
            "schematic column count at row {} is {}, expected {}",
            y,
            cols.len(),
            size.x
        );
        for (x, tok) in cols.iter().enumerate() {
            if *tok == "." {
                continue;
            }
            let tile = TileId::new(x as i32, y as i32);
            if let Some(id) = tok.strip_prefix('P') {
                let id: u64 = id.parse().expect("Invalid pawn id in schematic");
                assert!(
                    !pawn_pos.contains_key(&id),
                    "Pawn id {} appears multiple times in schematic",
                    id
                );
                assert!(
                    seen_pawn_tiles.insert((tile.x, tile.y)),
                    "Multiple pawns placed on the same tile {:?}",
                    tile
                );
                pawn_pos.insert(id, tile);
            } else if let Some(id) = tok.strip_prefix('F') {
                let id: u64 =
                    id.parse().expect("Invalid fixture id in schematic");
                assert!(
                    !fixture_pos.contains_key(&id),
                    "Fixture id {} appears multiple times in schematic",
                    id
                );
                assert!(
                    seen_fix_tiles.insert((tile.x, tile.y)),
                    "Multiple fixtures placed on the same tile {:?}",
                    tile
                );
                fixture_pos.insert(id, tile);
            } else {
                panic!("Unknown token in schematic: '{}'", tok);
            }
        }
    }

    (pawn_pos, fixture_pos)
}

fn spawn_pawns_from_def(
    commands: &mut Commands,
    rng: &mut SmallRng,
    map_size: super::model::MapSize,
    index: &mut IdIndex<PawnId>,
    pawns: &[super::model::PawnDef],
    tile_index: &mut TileMapIndex<PawnId>,
    item_index: &mut IdIndex<ItemId>,
    item_tile_index: &mut TileMapIndex<ItemId>,
) {
    use std::collections::HashSet;
    let mut used_positions: HashSet<(i32, i32)> = HashSet::new();
    for (i, p) in pawns.iter().enumerate() {
        let typed = index.alloc(p.id.map(PawnId));
        let name = p.name.clone().unwrap_or_else(|| format!("Pawn{}", i + 1));

        let mut gen = || rand_pos(rng, map_size);
        let pos = match p.pos {
            Some(pos) => unique_pos(&mut used_positions, pos, &mut gen),
            None => unique_pos(&mut used_positions, gen(), &mut gen),
        };

        // Build pawn inventory: spawn items carried by this pawn
        let mut inventory = Inventory::default();
        for it in p.inventory.iter() {
            let typed_item = item_index.alloc(it.id.map(ItemId));
            let kind = ItemKind::from_str(&it.kind).unwrap();
            let entity = commands
                .spawn((
                    crate::WorldTag,
                    Name::new(format!("Item#{}", typed_item.0)),
                    Item {
                        id: typed_item,
                        kind,
                        qty: it.qty,
                    },
                    ItemRelation::CarriedBy(typed),
                ))
                .id();
            item_index.insert(typed_item, entity);
            inventory.add((typed_item, kind));
        }

        let entity = commands
            .spawn((
                crate::WorldTag,
                Name::new(name),
                Pawn {
                    id: typed,
                    inventory,
                    sleep: p.sleep.unwrap_or(100).into(),
                    hunger: p.hunger.unwrap_or(100).into(),
                },
                pos,
            ))
            .id();
        index.insert(typed, entity);

        // Tile index mark
        tile_index.move_id(None, pos, typed);
    }
}

fn spawn_items_from_def(
    commands: &mut Commands,
    rng: &mut SmallRng,
    map_size: super::model::MapSize,
    index: &mut IdIndex<ItemId>,
    items: &[super::model::ItemDef],
    tile_index: &mut TileMapIndex<ItemId>,
) -> Inventory {
    use std::collections::HashSet;
    let mut used_positions: HashSet<(i32, i32)> = HashSet::new();
    let mut gen = || rand_pos(rng, map_size);

    let mut inventory = Inventory::default();

    for it in items.iter() {
        let typed = index.alloc(it.id.map(ItemId));
        let kind = ItemKind::from_str(&it.kind).unwrap();
        let pos = match it.pos {
            Some(pos) => unique_pos(&mut used_positions, pos, &mut gen),
            None => unique_pos(&mut used_positions, gen(), &mut gen),
        };

        // Spawn item on ground (via ItemRelation)
        let entity = commands
            .spawn((
                crate::WorldTag,
                Name::new(format!("Item#{}", typed.0)),
                Item {
                    id: typed,
                    kind,
                    qty: it.qty,
                },
                ItemRelation::OnGround(pos),
            ))
            .id();
        index.insert(typed, entity);

        // Add to inventory
        inventory.add((typed, kind));
        // Tile index mark
        tile_index.move_id(None, pos, typed);
    }

    inventory
}

fn spawn_fixtures_from_def(
    commands: &mut Commands,
    rng: &mut SmallRng,
    map_size: super::model::MapSize,
    index: &mut IdIndex<FixtureId>,
    fixtures: &[super::model::FixtureDef],
    tile_index: &mut TileMapIndex<FixtureId>,
    item_index: &mut IdIndex<ItemId>,
    item_tile_index: &mut TileMapIndex<ItemId>,
) {
    use std::collections::HashSet;
    let mut used_positions: HashSet<(i32, i32)> = HashSet::new();
    for it in fixtures.iter() {
        let typed = index.alloc(it.id.map(FixtureId));
        let mut gen = || rand_pos(rng, map_size);
        let pos = match it.pos {
            Some(pos) => unique_pos(&mut used_positions, pos, &mut gen),
            None => unique_pos(&mut used_positions, gen(), &mut gen),
        };
        // Build fixture inventory by spawning items attached to the fixture,
        // not on the ground.
        let mut inventory = Inventory::default();
        for def in it.inventory.iter() {
            let typed_item = item_index.alloc(def.id.map(ItemId));
            let kind = ItemKind::from_str(&def.kind).unwrap();
            let entity = commands
                .spawn((
                    crate::WorldTag,
                    Name::new(format!("Item#{}", typed_item.0)),
                    Item {
                        id: typed_item,
                        kind,
                        qty: def.qty,
                    },
                    ItemRelation::InFixture(typed),
                ))
                .id();
            item_index.insert(typed_item, entity);
            inventory.add((typed_item, kind));
        }
        let kind = FixtureKind::from_str(&it.kind).unwrap();

        let entity = commands
            .spawn((
                crate::WorldTag,
                Name::new(format!("Fixture#{}", typed.0)),
                Fixture {
                    id: typed,
                    harvest_countdown: match it.harvest_countdown {
                        Some(countdown) => Some(countdown),
                        None => match kind {
                            FixtureKind::BerryBush => Some(100),
                            _ => None,
                        },
                    },
                    kind,
                    inventory,
                },
                pos,
            ))
            .id();
        index.insert(typed, entity);

        // Tile index mark
        tile_index.move_id(None, pos, typed);
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
        pawn_index: ResMut<IdIndex<PawnId>>,
        item_index: ResMut<IdIndex<ItemId>>,
        fixture_index: ResMut<IdIndex<FixtureId>>,
        task_index: ResMut<IdIndex<TaskId>>,
        scn: Res<TestScenario>,
    ) {
        load_scenario_from_def(
            commands,
            rng,
            pawn_index,
            item_index,
            fixture_index,
            task_index,
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
                schematic: None,
            },
            pawns: vec![
                model::PawnDef {
                    id: Some(10),
                    name: Some("Ada".into()),
                    pos: None,
                    sleep: Some(50),
                    hunger: Some(5),
                    inventory: vec![],
                    priorities: Default::default(),
                },
                model::PawnDef {
                    id: None,
                    name: None,
                    pos: None,
                    sleep: Some(50),
                    hunger: Some(50),
                    inventory: vec![],
                    priorities: Default::default(),
                },
            ],
            fixtures: vec![model::FixtureDef {
                id: None,
                kind: "Stockpile".into(),
                pos: None,
                inventory: vec![],
                harvest_countdown: None,
            }],
            tasks: vec![],
        };

        let mut app = App::new();
        app.init_resource::<RngResource>()
            .init_resource::<IdIndex<PawnId>>()
            .init_resource::<IdIndex<ItemId>>()
            .init_resource::<IdIndex<FixtureId>>()
            .init_resource::<IdIndex<TaskId>>()
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
        let names: Vec<String> =
            pawns.iter().map(|(_, _, n)| n.to_string()).collect();
        assert!(names.contains(&"Ada".to_string()));
        assert!(names.iter().any(|s| s == "Pawn2"));

        // Validate items: with this ScenarioDef (no items on map, none in
        // pawns/fixtures), there should be zero items spawned.
        let mut item_q = world.query::<(&Item, &TileId)>();
        let items: Vec<_> = item_q.iter(world).collect();
        assert_eq!(items.len(), 0);

        // Validate fixtures
        let mut fixture_q = world.query::<(&Fixture, &TileId)>();
        let fixtures: Vec<_> = fixture_q.iter(world).collect();
        assert_eq!(fixtures.len(), 1);
        assert!(fixtures[0].0.id.0 >= 1000);
        let fpos = fixtures[0].1;
        assert!(fpos.x >= 0 && fpos.x < 4 && fpos.y >= 0 && fpos.y < 4);
    }

    #[test]
    fn scenario_loading_spawns_map_items_and_indices_ok() {
        let def = ScenarioDef {
            sim_seed: Some(7),
            map: model::MapDef {
                size: model::MapSize { x: 5, y: 5 },
                tiles: vec![model::TileDef {
                    pos: TileId::new(2, 3),
                    walkable: true,
                    terrain: model::Terrain::Grass,
                    item: Some(model::ItemDef {
                        id: None,
                        kind: "Berry".into(),
                        qty: 1,
                        pos: None,
                    }),
                }],
                schematic: None,
            },
            pawns: vec![],
            fixtures: vec![],
            tasks: vec![],
        };

        let mut app = App::new();
        app.init_resource::<RngResource>()
            .init_resource::<IdIndex<PawnId>>()
            .init_resource::<IdIndex<ItemId>>()
            .init_resource::<IdIndex<FixtureId>>()
            .init_resource::<IdIndex<TaskId>>()
            .insert_resource(TestScenario(def))
            .add_systems(Startup, sys_load_from_def);
        app.update();

        let world = app.world_mut();
        let mut item_q = world.query::<(&Item, &ItemRelation)>();
        let items: Vec<_> = item_q.iter(world).collect();
        assert_eq!(items.len(), 1);
        let (it, rel) = items[0];
        assert!(it.id.0 >= 1000);
        // Item is spawned and placed somewhere on the map
        let pos = match rel {
            ItemRelation::OnGround(p) => *p,
            _ => panic!("expected OnGround item"),
        };
        assert!(pos.x >= 0 && pos.x < 5 && pos.y >= 0 && pos.y < 5);

        // Index reflects placement
        let idx = world.resource::<TileMapIndex<ItemId>>();
        assert_eq!(idx.get(pos), Some(it.id));
    }

    // This test encodes the expected behavior that a TileDef with `item`
    // should place that item at the tile's position. It currently FAILS
    // because loader ignores the TileDef.pos for items and randomizes the
    // item position.
    #[test]
    fn scenario_map_tile_item_respects_tile_position() {
        let def = ScenarioDef {
            sim_seed: Some(7),
            map: model::MapDef {
                size: model::MapSize { x: 5, y: 5 },
                tiles: vec![model::TileDef {
                    pos: TileId::new(2, 3),
                    walkable: true,
                    terrain: model::Terrain::Grass,
                    item: Some(model::ItemDef {
                        id: None,
                        kind: "Berry".into(),
                        qty: 1,
                        pos: None,
                    }),
                }],
                schematic: None,
            },
            pawns: vec![],
            fixtures: vec![],
            tasks: vec![],
        };

        let mut app = App::new();
        app.init_resource::<RngResource>()
            .init_resource::<IdIndex<PawnId>>()
            .init_resource::<IdIndex<ItemId>>()
            .init_resource::<IdIndex<FixtureId>>()
            .init_resource::<IdIndex<TaskId>>()
            .insert_resource(TestScenario(def))
            .add_systems(Startup, sys_load_from_def);
        app.update();

        let world = app.world_mut();
        let mut item_q = world.query::<(&Item, &ItemRelation)>();
        let items: Vec<_> = item_q.iter(world).collect();
        assert_eq!(items.len(), 1);
        let (_, rel) = items[0];
        let pos = match rel {
            ItemRelation::OnGround(p) => *p,
            _ => panic!("expected OnGround item"),
        };
        assert_eq!(pos, TileId::new(2, 3));
    }

    #[test]
    fn scenario_loading_assigns_pawn_names_positions_and_index() {
        let def = ScenarioDef {
            sim_seed: Some(99),
            map: model::MapDef {
                size: model::MapSize { x: 6, y: 4 },
                tiles: vec![],
                schematic: None,
            },
            pawns: vec![
                model::PawnDef {
                    id: Some(100),
                    name: Some("Eve".into()),
                    pos: Some(TileId::new(0, 0)),
                    ..Default::default()
                },
                model::PawnDef::default(),
            ],
            fixtures: vec![],
            tasks: vec![],
        };

        let mut app = App::new();
        app.init_resource::<RngResource>()
            .init_resource::<IdIndex<PawnId>>()
            .init_resource::<IdIndex<ItemId>>()
            .init_resource::<IdIndex<FixtureId>>()
            .init_resource::<IdIndex<TaskId>>()
            .insert_resource(TestScenario(def))
            .add_systems(Startup, sys_load_from_def);
        app.update();

        let world = app.world_mut();
        let mut pawn_q = world.query::<(&Pawn, &TileId, &Name)>();
        let pawns: Vec<_> = pawn_q.iter(world).collect();
        assert_eq!(pawns.len(), 2);

        // Eve with id 100 exists at a valid position
        let eve = pawns
            .iter()
            .find(|(p, _, n)| p.id.0 == 100 && n.as_str() == "Eve")
            .expect("Eve pawn present");
        assert!(eve.1.x >= 0 && eve.1.x < 6 && eve.1.y >= 0 && eve.1.y < 4);

        // Positions are within bounds and unique in the small map
        let mut seen = std::collections::HashSet::new();
        for (_, pos, _) in pawns.iter() {
            assert!(pos.x >= 0 && pos.x < 6 && pos.y >= 0 && pos.y < 4);
            assert!(
                seen.insert((pos.x, pos.y)),
                "pawn positions must be unique"
            );
        }

        // Index reflects placement
        let idx = world.resource::<TileMapIndex<PawnId>>();
        for (p, pos, _) in pawns.iter() {
            assert_eq!(idx.get(**pos), Some(p.id));
        }
    }

    #[test]
    fn scenario_pawn_inventory_items_are_carried_and_not_on_ground() {
        let def = ScenarioDef {
            sim_seed: Some(11),
            map: model::MapDef {
                size: model::MapSize { x: 5, y: 5 },
                tiles: vec![],
                schematic: None,
            },
            pawns: vec![model::PawnDef {
                id: Some(200),
                name: Some("Kai".into()),
                pos: Some(TileId::new(2, 2)),
                inventory: vec![model::ItemDef {
                    id: Some(3000),
                    kind: "Berry".into(),
                    qty: 1,
                    pos: None,
                }],
                ..Default::default()
            }],
            fixtures: vec![],
            tasks: vec![],
        };

        let mut app = App::new();
        app.init_resource::<RngResource>()
            .init_resource::<IdIndex<PawnId>>()
            .init_resource::<IdIndex<ItemId>>()
            .init_resource::<IdIndex<FixtureId>>()
            .init_resource::<IdIndex<TaskId>>()
            .insert_resource(TestScenario(def))
            .add_systems(Startup, sys_load_from_def);
        app.update();

        let world = app.world_mut();
        // Fetch pawn and its inventory contents, but copy out the values to
        // drop the borrow
        let (pawn_id, item_id) = {
            let mut pawn_q = world.query::<(&Pawn, &TileId)>();
            let pawns: Vec<_> = pawn_q.iter(world).collect();
            assert_eq!(pawns.len(), 1);
            let (pawn, _pos) = pawns[0];
            let (item_id, _) = pawn.inventory.0[0];
            (pawn.id, item_id)
        };

        // Item should be carried by this pawn and not on ground
        let mut item_q = world.query::<&ItemRelation>();
        let ent = world.resource::<IdIndex<ItemId>>().get(&item_id);
        let relation = item_q.get(world, ent).unwrap();
        assert_eq!(relation, &ItemRelation::CarriedBy(pawn_id));
    }

    #[test]
    fn scenario_ground_item_is_not_in_any_inventory() {
        let def = ScenarioDef {
            sim_seed: Some(13),
            map: model::MapDef {
                size: model::MapSize { x: 4, y: 4 },
                tiles: vec![model::TileDef {
                    pos: TileId::new(1, 1),
                    walkable: true,
                    terrain: model::Terrain::Grass,
                    item: Some(model::ItemDef {
                        id: None,
                        kind: "Berry".into(),
                        qty: 1,
                        pos: None,
                    }),
                }],
                schematic: None,
            },
            pawns: vec![model::PawnDef::default()],
            fixtures: vec![model::FixtureDef {
                id: None,
                kind: "Stockpile".into(),
                pos: None,
                inventory: vec![],
                harvest_countdown: None,
            }],
            tasks: vec![],
        };

        let mut app = App::new();
        app.init_resource::<RngResource>()
            .init_resource::<IdIndex<PawnId>>()
            .init_resource::<IdIndex<ItemId>>()
            .init_resource::<IdIndex<FixtureId>>()
            .init_resource::<IdIndex<TaskId>>()
            .insert_resource(TestScenario(def))
            .add_systems(Startup, sys_load_from_def);
        app.update();

        let world = app.world_mut();
        let ground_item_id = {
            let mut item_q = world.query::<(&Item, &ItemRelation)>();
            let items: Vec<_> = item_q.iter(world).collect();
            assert_eq!(items.len(), 1);
            let (it, rel) = items[0];
            assert!(matches!(rel, ItemRelation::OnGround(_)));
            it.id
        };

        // Ensure no pawn or fixture inventory contains this ground item
        let mut pawn_q = world.query::<&Pawn>();
        for p in pawn_q.iter(world) {
            assert!(!p.inventory.contains(&ground_item_id));
        }
        let mut fix_q = world.query::<&Fixture>();
        for f in fix_q.iter(world) {
            assert!(!f.inventory.contains(&ground_item_id));
        }
    }

    // This test captures a likely bug: items declared in a fixture's
    // inventory should be attached with InFixture and not be on the ground.
    // It currently FAILS with the existing loader logic.
    #[test]
    fn scenario_fixture_inventory_items_are_infixture_and_not_on_ground() {
        let def = ScenarioDef {
            sim_seed: Some(5),
            map: model::MapDef {
                size: model::MapSize { x: 4, y: 4 },
                tiles: vec![],
                schematic: None,
            },
            pawns: vec![],
            fixtures: vec![model::FixtureDef {
                id: Some(123),
                kind: "BerryBush".into(),
                pos: Some(TileId::new(1, 1)),
                inventory: vec![model::ItemDef {
                    id: None,
                    kind: "Berry".into(),
                    qty: 1,
                    pos: None,
                }],
                harvest_countdown: None,
            }],
            tasks: vec![],
        };

        let mut app = App::new();
        app.init_resource::<RngResource>()
            .init_resource::<IdIndex<PawnId>>()
            .init_resource::<IdIndex<ItemId>>()
            .init_resource::<IdIndex<FixtureId>>()
            .init_resource::<IdIndex<TaskId>>()
            .insert_resource(TestScenario(def))
            .add_systems(Startup, sys_load_from_def);
        app.update();

        let world = app.world_mut();
        // Scope fixture borrow so we can run other queries after.
        let (fixture_copy, fpos_copy, item_id) = {
            let mut f_q = world.query::<(&Fixture, &TileId)>();
            let fixtures: Vec<_> = f_q.iter(world).collect();
            assert_eq!(fixtures.len(), 1);
            let (fixture, fpos) = fixtures[0];
            assert_eq!(fixture.id.0, 123);
            assert_eq!(fixture.kind, FixtureKind::BerryBush);
            assert_eq!(fixture.harvest_countdown, Some(100));
            assert_eq!(*fpos, TileId::new(1, 1));

            let (item_id, _) = fixture
                .inventory
                .0
                .first()
                .copied()
                .expect("fixture inventory item present");
            (fixture.clone(), *fpos, item_id)
        };

        // EXPECTED: item should be attached to the fixture via InFixture
        // and should NOT have a TileId on the ground.
        // CURRENT BEHAVIOR: item is spawned on the ground with TileId and no
        // InFixture. This assertion will FAIL until loader attaches
        // items to fixtures.
        let mut q = world.query::<&ItemRelation>();
        let ent = world.resource::<IdIndex<ItemId>>().get(&item_id);
        let relation = q.get(world, ent).unwrap();
        assert_eq!(relation, &ItemRelation::InFixture(fixture_copy.id));
    }

    #[test]
    fn schematic_places_pawns_and_fixtures() {
        // 4x4 map with schematic that places Pawn#1 at (1,1), Pawn#2 at (0,3),
        // Fixture#5 at (2,2)
        let schematic = "\n. . . .\n. P1 . .\n. . F5 .\nP2 . . .\n".to_string();

        let def = ScenarioDef {
            sim_seed: Some(1),
            map: model::MapDef {
                size: model::MapSize { x: 4, y: 4 },
                tiles: vec![],
                schematic: Some(schematic),
            },
            pawns: vec![
                model::PawnDef {
                    id: Some(1),
                    name: Some("Sam".into()),
                    ..Default::default()
                },
                model::PawnDef {
                    id: Some(2),
                    name: Some("Bil".into()),
                    ..Default::default()
                },
            ],
            fixtures: vec![model::FixtureDef {
                id: Some(5),
                kind: "Stockpile".into(),
                pos: None,
                inventory: vec![],
                harvest_countdown: None,
            }],
            tasks: vec![],
        };

        let mut app = App::new();
        app.init_resource::<RngResource>()
            .init_resource::<IdIndex<PawnId>>()
            .init_resource::<IdIndex<ItemId>>()
            .init_resource::<IdIndex<FixtureId>>()
            .init_resource::<IdIndex<TaskId>>()
            .insert_resource(TestScenario(def))
            .add_systems(Startup, sys_load_from_def);
        app.update();

        let world = app.world_mut();
        // Assert pawn positions
        let mut pq = world.query::<(&Pawn, &TileId)>();
        let mut p1_ok = false;
        let mut p2_ok = false;
        for (p, pos) in pq.iter(world) {
            match p.id.0 {
                1 => {
                    assert_eq!(*pos, TileId::new(1, 1));
                    p1_ok = true;
                }
                2 => {
                    assert_eq!(*pos, TileId::new(0, 3));
                    p2_ok = true;
                }
                _ => {}
            }
        }
        assert!(p1_ok && p2_ok);

        // Assert fixture position
        let mut fq = world.query::<(&Fixture, &TileId)>();
        let mut f_ok = false;
        for (f, pos) in fq.iter(world) {
            if f.id.0 == 5 {
                assert_eq!(*pos, TileId::new(2, 2));
                f_ok = true;
            }
        }
        assert!(f_ok);
    }
}
