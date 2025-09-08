#![allow(clippy::type_complexity, clippy::too_many_arguments)]
#![feature(let_chains)]

use bevy::prelude::*;
use rand::{rngs::SmallRng, SeedableRng};
use simkit_core::{
    grid::{index::sync_tile_index, TileId},
    AppState,
    KitSystemSet,
    Playback,
};

use crate::{
    model::{
        components::{Fixture, Item, Pawn},
        ids::TaskId,
    },
    scenario::LoadedScenarioMeta,
    snapshot::{build_world_snapshot, stable_hash_json},
};

pub mod model;
pub mod scenario;
pub mod snapshot;
pub mod tasks;
pub mod toils;
pub mod world;
pub mod invariants;

// Resources and markers
#[derive(Resource, Debug, Clone, Copy)]
pub struct EditBudget {
    pub per_tick: u32,
    pub remaining: u32,
}

impl Default for EditBudget {
    fn default() -> Self {
        Self {
            per_tick: 16,
            remaining: 16,
        }
    }
}

#[derive(Resource)]
pub struct RngResource(pub SmallRng);

impl Default for RngResource {
    fn default() -> Self {
        // Deterministic default seed unless CLI overrides on enter
        Self(SmallRng::seed_from_u64(1))
    }
}

#[derive(Component, Default)]
pub struct WorldTag;

// Sub-labels within PreStep/Step/PostStep to stabilize ordering
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum StitchPreStepSet {
    ResetEditBudget,
    NeedsDaemonEmit,
    DesignationSpawner,
    TaskPrune,
    PathCachePrepare,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum StitchStepSet {
    HardNeedInterrupts,
    SchedulerAssign,
    JobTick,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum StitchPostStepSet {
    ReleaseStaleReservations,
    TelemetrySample,
}

// CLI options resource (populated by main)
#[derive(Resource, Debug, Clone)]
pub struct CliOptions {
    pub mode: RunMode,
    pub scenario: Option<std::path::PathBuf>,
    pub snapshot: Option<std::path::PathBuf>,
    pub ticks: Option<u64>,
    pub seed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Live,
    Headless,
}

pub struct StitchlandsCorePlugin;

impl Plugin for StitchlandsCorePlugin {
    fn build(&self, app: &mut App) {
        // Resources
        app.init_resource::<EditBudget>()
            .init_resource::<RngResource>()
            // Task system core resources
            .init_resource::<tasks::TaskBoard>()
            .add_plugins(crate::scenario::ScenarioPlugin)
            .add_event::<SnapshotSaveEvent>()
            // Chain our sub-sets inside KitSystemSet phases
            .configure_sets(
                FixedUpdate,
                (
                    (
                        StitchPreStepSet::ResetEditBudget,
                        StitchPreStepSet::NeedsDaemonEmit,
                        StitchPreStepSet::DesignationSpawner,
                        StitchPreStepSet::TaskPrune,
                        StitchPreStepSet::PathCachePrepare,
                    )
                        .chain()
                        .in_set(KitSystemSet::PreStep),
                    (
                        StitchStepSet::HardNeedInterrupts,
                        StitchStepSet::SchedulerAssign,
                        StitchStepSet::JobTick,
                    )
                        .chain()
                        .in_set(KitSystemSet::Step),
                    (
                        StitchPostStepSet::ReleaseStaleReservations,
                        StitchPostStepSet::TelemetrySample,
                    )
                        .chain()
                        .in_set(KitSystemSet::PostStep),
                ),
            )
            // Systems (PreStep)
            // consollidate into a single add_systems call
            .add_systems(
                FixedUpdate,
                (
                    reset_edit_budget.in_set(StitchPreStepSet::ResetEditBudget),
                    sync_tile_index::<Pawn>.in_set(KitSystemSet::PostStep),
                    sync_tile_index::<Item>.in_set(KitSystemSet::PostStep),
                ),
            )
            // Snapshot save handler (runs in Update for responsiveness)
            .add_systems(Update, handle_snapshot_save_events)
            // Auto-enter InGame when running headless
            .add_systems(
                OnEnter(simkit_core::AppState::Menu),
                auto_enter_ingame_if_headless,
            );
    }
}

// 0.a: PreStep system — reset budget
fn reset_edit_budget(mut budget: ResMut<EditBudget>) {
    budget.remaining = budget.per_tick;
}


fn auto_enter_ingame_if_headless(
    cli: Option<Res<CliOptions>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let Some(cli) = cli else { return };
    if cli.mode == RunMode::Headless {
        next_state.set(AppState::InGame);
    }
}

// fn headless_exit_after_ticks(
//     cli: Option<Res<CliOptions>>,
//     playback: Res<Playback>,
//     _budget: Res<EditBudget>,
//     _rng: Res<RngResource>,
//     _world_tag_q: Query<Entity, With<WorldTag>>,
//     pawn_q: Query<(&Pawn, &TileId)>,
//     item_q: Query<(&Item, &TileId)>,
//     fixture_q: Query<(&Fixture, &TileId)>,
//     // todo
//     // task_q: Query<&TaskId>,
//     scenario_meta: Option<Res<LoadedScenarioMeta>>,
//     mut exit: EventWriter<AppExit>,
// ) {
//     let Some(cli) = cli else { return };
//     if cli.mode != RunMode::Headless {
//         return;
//     }
//     if let Some(limit) = cli.ticks {
//         if (playback.tick.0 as u64) >= limit {
//             // Extract a baseline snapshot and print a stable hash for
//             // determinism testing
//             // let scenario_seed = scenario_meta.as_ref().and_then(|m|
//             // m.sim_seed); let pawns_vec: Vec<_> =
//             // pawn_q.iter().map(|(p, pos)| (*p, *pos)).collect();
//             // let items_vec: Vec<_> = item_q.iter().map(|(it, pos)|
//             // (it.clone(), *pos)).collect(); let fixtures_vec:
//             // Vec<_> = fixture_q.iter().map(|(f, pos)| (f.clone(),
//             // *pos)).collect(); let tasks_vec: Vec<_> =
//             // task_q.iter().map(|(t, _)| (t.clone(), *pos)).collect();
//             // let world_snap =
//             //     build_world_snapshot(&playback, scenario_seed, &pawns_vec,
//             // &items_vec, &zones_vec); let hash =
//             // stable_hash_json(&world_snap); println!("SNAP:{}",
//             // hash); exit.write(AppExit::Success);
//         }
//         todo!()
//     }
// }

// Event to trigger saving a snapshot to disk (RON format)
#[derive(Event)]
pub struct SnapshotSaveEvent {
    pub path: std::path::PathBuf,
}

fn handle_snapshot_save_events(
    mut evr: EventReader<SnapshotSaveEvent>,
    playback: Res<Playback>,
    scenario_meta: Option<Res<LoadedScenarioMeta>>,
    pawns_q: Query<(&Pawn, &TileId)>,
    items_q: Query<(&Item, &TileId)>,
    fixtures_q: Query<(&Fixture, &TileId)>,
    // tasks_q: Query<&TaskId>,
) {
    use std::fs;
    let scenario_seed = scenario_meta.as_ref().and_then(|m| m.sim_seed);
    for ev in evr.read() {
        let pawns_vec: Vec<_> =
            pawns_q.iter().map(|(p, pos)| (p.clone(), *pos)).collect();
        let items_vec: Vec<_> =
            items_q.iter().map(|(it, pos)| (it.clone(), *pos)).collect();
        let fixtures_vec: Vec<_> = fixtures_q
            .iter()
            .map(|(f, pos)| (f.clone(), *pos))
            .collect();
        let tasks_vec = vec![];
        let snap = build_world_snapshot(
            &playback,
            scenario_seed,
            &pawns_vec,
            &items_vec,
            &fixtures_vec,
            &tasks_vec,
        );
        let pretty = ron::ser::to_string_pretty(
            &snap,
            ron::ser::PrettyConfig::default(),
        )
        .expect("serialize snapshot to RON");
        if let Err(e) = fs::write(&ev.path, pretty) {
            eprintln!("Failed to write snapshot to {:?}: {}", ev.path, e);
        }
    }
}
