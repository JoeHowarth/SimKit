use std::{fs, path::Path};

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use simkit_core::{grid::TileId, ids::IdIndex};

use super::{load_scenario_from_def, model::ScenarioDef};
use crate::{
    RngResource,
    StepSystemLabel,
    environment_step::EnvironmentStepPlugin,
    model::{
        Harvestable,
        components::{Fixture, Pawn},
        ids::{FixtureId, ItemId, PawnId, TaskId},
    },
    tasks::TaskPlugin,
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

/// Build an App, load the given ScenarioDef, run Startup, and return the App.
pub fn app_with_scenario(def: ScenarioDef) -> App {
    let mut app = App::new();
    simkit_core::configure_sets(&mut app, StepSystemLabel::default(), false);
    app.init_resource::<RngResource>()
        .init_resource::<IdIndex<PawnId>>()
        .init_resource::<IdIndex<ItemId>>()
        .init_resource::<IdIndex<FixtureId>>()
        .init_resource::<IdIndex<TaskId>>()
        .insert_resource(TestScenario(def))
        .add_plugins(TaskPlugin)
        .add_plugins(EnvironmentStepPlugin);

    {
        let world = app.world_mut();
        world.run_system_once(sys_load_from_def).unwrap();
        world.flush();
    }

    app
}

/// Assert a tile position is within [0, w) x [0, h).
pub fn assert_within_bounds(pos: TileId, size: (u32, u32)) {
    assert!(
        pos.x >= 0
            && pos.x < size.0 as i32
            && pos.y >= 0
            && pos.y < size.1 as i32,
        "pos {:?} not within bounds {:?}",
        pos,
        size
    );
}

/// Find a pawn by id and return (Entity, Pawn clone, TileId).
pub fn pawn_by_id(world: &mut World, id: u64) -> (Entity, Pawn, TileId) {
    let mut q = world.query::<(Entity, &Pawn, &TileId)>();
    for (e, p, pos) in q.iter(world) {
        if p.id.0 == id {
            return (e, p.clone(), *pos);
        }
    }
    panic!("Pawn with id={} not found", id);
}

/// Find a fixture by id and return (Entity, Fixture clone, TileId).
pub fn fixture_by_id(
    world: &mut World,
    id: u64,
) -> (Entity, Fixture, TileId, Option<Harvestable>) {
    let mut q =
        world.query::<(Entity, &Fixture, &TileId, Option<&Harvestable>)>();
    for (e, f, pos, harvest_countdown) in q.iter(world) {
        if f.id.0 == id {
            return (e, f.clone(), *pos, harvest_countdown.copied());
        }
    }
    panic!("Fixture with id={} not found", id);
}

/// Load a ScenarioDef from a TOML file path.
/// Note: this helper relies on the `toml` crate, which is available in
/// dev-dependencies for tests.
pub fn load_toml<P: AsRef<Path>>(p: P) -> ScenarioDef {
    let s = fs::read_to_string(p).expect("read scenario toml");
    toml::from_str(&s).expect("parse scenario toml")
}
