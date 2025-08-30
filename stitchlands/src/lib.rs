use bevy::prelude::*;
use rand::{rngs::SmallRng, SeedableRng};

use simkit_core::{AppState, KitSystemSet, Playback};
use crate::snapshot::{extract_snapshot_v0, stable_hash_json};

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

#[derive(Component)]
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
            .add_systems(
                FixedUpdate,
                reset_edit_budget.in_set(StitchPreStepSet::ResetEditBudget),
            )
            .add_systems(
                FixedUpdate,
                needs_daemon_emit_stub.in_set(StitchPreStepSet::NeedsDaemonEmit),
            )
            .add_systems(
                FixedUpdate,
                designation_spawner_stub.in_set(StitchPreStepSet::DesignationSpawner),
            )
            .add_systems(
                FixedUpdate,
                task_prune_stub.in_set(StitchPreStepSet::TaskPrune),
            )
            .add_systems(
                FixedUpdate,
                path_cache_prepare_stub.in_set(StitchPreStepSet::PathCachePrepare),
            )
            // Systems (Step)
            .add_systems(
                FixedUpdate,
                hard_need_interrupts_stub.in_set(StitchStepSet::HardNeedInterrupts),
            )
            .add_systems(
                FixedUpdate,
                scheduler_assign_stub.in_set(StitchStepSet::SchedulerAssign),
            )
            .add_systems(
                FixedUpdate,
                job_tick_stub.in_set(StitchStepSet::JobTick),
            )
            // Systems (PostStep)
            .add_systems(
                FixedUpdate,
                release_stale_reservations_stub
                    .in_set(StitchPostStepSet::ReleaseStaleReservations),
            )
            .add_systems(
                FixedUpdate,
                telemetry_sample_stub.in_set(StitchPostStepSet::TelemetrySample),
            )
            // Seed RNG at game start from CLI seed
            .add_systems(
                OnEnter(AppState::InGame),
                seed_rng_from_cli,
            )
            // Auto-enter InGame when running headless
            .add_systems(OnEnter(simkit_core::AppState::Menu), auto_enter_ingame_if_headless)
            // Exit headless after N ticks exactly (post-step to complete the tick)
            .add_systems(
                FixedUpdate,
                headless_exit_after_ticks.in_set(KitSystemSet::PostStep),
            );
    }
}

// 0.a: PreStep system — reset budget
fn reset_edit_budget(mut budget: ResMut<EditBudget>) {
    budget.remaining = budget.per_tick;
}

// 0.a: Stubs for future phases
fn needs_daemon_emit_stub() {}
fn designation_spawner_stub() {}
fn task_prune_stub() {}
fn path_cache_prepare_stub() {}
fn hard_need_interrupts_stub() {}
fn scheduler_assign_stub() {}
fn job_tick_stub() {}
fn release_stale_reservations_stub() {}
fn telemetry_sample_stub() {}

fn seed_rng_from_cli(mut rng: ResMut<RngResource>, cli: Option<Res<CliOptions>>) {
    let seed = cli.as_deref().map(|c| c.seed).unwrap_or(1);
    rng.0 = SmallRng::seed_from_u64(seed);
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

fn headless_exit_after_ticks(
    cli: Option<Res<CliOptions>>,
    playback: Res<Playback>,
    budget: Res<EditBudget>,
    rng: Res<RngResource>,
    mut exit: EventWriter<AppExit>,
) {
    let Some(cli) = cli else { return };
    if cli.mode != RunMode::Headless {
        return;
    }
    if let Some(limit) = cli.ticks {
        if (playback.tick.0 as u64) >= limit {
            // Extract a minimal baseline snapshot and print a stable hash for determinism testing
            let snap = extract_snapshot_v0(&playback, &rng, &budget);
            let hash = stable_hash_json(&snap);
            println!("SNAP:{}", hash);
            exit.write(AppExit::Success);
        }
    }
}

pub mod snapshot;
