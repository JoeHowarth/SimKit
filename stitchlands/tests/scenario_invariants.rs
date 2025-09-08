use std::path::PathBuf;

use bevy::prelude::*;
use simkit_core::{grid::TileId, ids::IdIndex};

use stitchlands::{
    invariants::validate_world,
    model::{
        components::{Fixture, Item, Pawn},
        ids::{FixtureId, ItemId, PawnId, TaskId},
    },
    scenario::{load_scenario_from_def, model::ScenarioDef},
    RngResource,
};

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
fn scenario_toml_loads_and_passes_invariants() {
    // Load TOML scenario file
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/small.toml");
    let s = std::fs::read_to_string(&path).expect("read scenario toml");
    let scenario: ScenarioDef = toml::from_str(&s).expect("parse scenario toml");

    // Build app, load scenario, then run invariants validation
    let mut app = App::new();
    app.init_resource::<RngResource>()
        .init_resource::<IdIndex<PawnId>>()
        .init_resource::<IdIndex<ItemId>>()
        .init_resource::<IdIndex<FixtureId>>()
        .init_resource::<IdIndex<TaskId>>()
        .insert_resource(TestScenario(scenario))
        .add_systems(Startup, sys_load_from_def);

    // Run Startup systems to load the scenario
    app.update();

    // Quick spot checks: ensure some entities exist
    {
        let world = app.world_mut();
        // Pawns present
        let mut qp = world.query::<(&Pawn, &TileId)>();
        let pawns: Vec<_> = qp.iter(world).collect();
        assert!(pawns.len() >= 2);
        // Items exist
        let mut qi = world.query::<&Item>();
        let items: Vec<_> = qi.iter(world).collect();
        assert!(!items.is_empty());
        // Fixtures exist
        let mut qf = world.query::<&Fixture>();
        let fixtures: Vec<_> = qf.iter(world).collect();
        assert!(!fixtures.is_empty());
    }

    // Validate invariants
    let errs = validate_world(app.world_mut());
    assert!(errs.is_empty(), "invariants failed: {:?}", errs);
}
