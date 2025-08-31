use std::fs;

use bevy::prelude::*;
use rand::{rngs::SmallRng, SeedableRng};

use crate::{CliOptions, RngResource};

use super::model::{
    complete_scenario, ItemComplete, PawnComplete, ScenarioDef,
    ZoneComplete,
};
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

    // Complete the scenario deterministically before spawning "real" components
    let fallback_seed = cli.as_deref().map(|c| c.seed).unwrap_or(1);
    let complete = complete_scenario(&scenario_def, fallback_seed);

    // Seed RNG for sim runtime
    rng.0 = SmallRng::seed_from_u64(complete.sim_seed);
    commands.insert_resource(LoadedScenarioMeta {
        sim_seed: Some(complete.sim_seed),
    });

    // Ensure allocators do not reuse provided IDs by bumping next past the max used
    if let Some(max) = complete.pawns.iter().map(|p| p.pawn.0 .0).max() {
        if pawn_alloc.next <= max {
            pawn_alloc.reset(max + 1);
        }
    }
    if let Some(max) = complete.items.iter().map(|i| i.item.id.0).max() {
        if item_alloc.next <= max {
            item_alloc.reset(max + 1);
        }
    }
    if let Some(max) = complete.zones.iter().map(|z| z.zone.id.0).max() {
        if zone_alloc.next <= max {
            zone_alloc.reset(max + 1);
        }
    }

    // Spawn pawns, items, zones with typed ID alloc/index using completed data
    for p in complete.pawns.iter() {
        spawn_pawn_completed(&mut commands, &mut pawn_index, p);
    }
    for it in complete.items.iter() {
        let _ = spawn_item_completed(&mut commands, &mut item_index, it);
    }
    for z in complete.zones.iter() {
        let _ = spawn_zone_completed(&mut commands, &mut zone_index, z);
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

fn spawn_pawn_completed(
    commands: &mut Commands,
    index: &mut IdIndex<PawnId>,
    p: &PawnComplete,
) -> Entity {
    let typed = p.pawn.0;
    let entity = commands
        .spawn((
            crate::WorldTag,
            Name::new(p.name.clone()),
            p.pawn,
            p.position,
        ))
        .id();
    index.insert(typed, entity);
    entity
}

fn spawn_item_completed(
    commands: &mut Commands,
    index: &mut IdIndex<ItemId>,
    def: &ItemComplete,
) -> Entity {
    let typed = def.item.id;
    let entity = commands
        .spawn((
            crate::WorldTag,
            Name::new(format!("Item#{}", typed.0)),
            def.item.clone(),
            def.position,
        ))
        .id();
    index.insert(typed, entity);
    entity
}

fn spawn_zone_completed(
    commands: &mut Commands,
    index: &mut IdIndex<ZoneId>,
    _def: &ZoneComplete,
) -> Entity {
    let typed = _def.zone.id;
    let entity = commands
        .spawn((
            crate::WorldTag,
            Name::new(format!("Zone#{}", typed.0)),
            _def.zone.clone(),
        ))
        .id();
    index.insert(typed, entity);
    entity
}
