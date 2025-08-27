use std::{collections::VecDeque, fmt::Debug, hash::Hash, path::PathBuf, sync::Arc, time::Instant};

use bevy::prelude::*;
use bevy_asset_loader::loading_state::{LoadingState, LoadingStateAppExt};

pub mod playback;
pub use playback::{PlayBackCommand, Playback};

pub mod menu;
pub use menu::MenuPlugin;

use crate::playback::SimStepTimer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct Tick(pub i32);

#[derive(Debug, Clone, Reflect)]
pub struct GameId(pub String);

#[derive(Debug, Clone, Reflect)]
pub enum CommandType {
    PlayBack(PlayBackCommand),
    LaunchGame(GameId),
    LoadSave(GameId),
    Save(GameId),
    ExitToMenu,
}

#[derive(Debug, Clone, Reflect, Event)]
pub struct SimCommand {
    pub command_type: CommandType,
    pub tick_sent: Tick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Resource, Default, States)]
pub enum AppState {
    InGame,
    Menu,
    #[default]
    AssetLoading,
}

// System sets to run before and after the sim step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, SystemSet)]
pub enum SimSystemSet {
    Tick,
    PreStep,
    Step,
    PostStep,
    HandleCommands,
    PerFrame,
}

/// A trait for types that are POD (Plain Old Data)
pub trait POD: Reflect + Debug + Clone + PartialEq + Eq + Hash + Send + Sync + 'static {}
impl<T: Reflect + Debug + Clone + PartialEq + Eq + Hash + Send + Sync + 'static> POD for T {}

pub trait Simulation: Sync + Send + 'static {
    type State: POD + Resource + Default;
    type Actions: POD + Event;
    type Event: POD + Event;

    fn step(
        &mut self,
        state: Self::State,
        actions: VecDeque<&Self::Actions>,
    ) -> (Self::State, VecDeque<Self::Event>);
}

pub struct CorePlugin<S: Simulation> {
    pub simulation: Arc<std::sync::Mutex<Option<S>>>,
}

impl<S: Simulation> CorePlugin<S> {
    pub fn new(simulation: S) -> Self {
        Self {
            simulation: Arc::new(std::sync::Mutex::new(Some(simulation))),
        }
    }
}

#[derive(Resource)]
pub struct SimulationResource<S: Simulation> {
    pub simulation: S,
}

impl<S: Simulation> Plugin for CorePlugin<S> {
    fn build(&self, app: &mut App) {
        let mut simulation = self.simulation.lock().unwrap();
        let simulation = std::mem::take(&mut *simulation).unwrap();

        app.register_type::<Tick>()
            .register_type::<GameId>()
            .register_type::<CommandType>()
            .register_type::<SimCommand>()
            .register_type::<Playback>()
            .add_event::<SimCommand>()
            .init_state::<AppState>()
            .add_loading_state(
                LoadingState::new(AppState::AssetLoading).continue_to_state(AppState::Menu),
            )
            .add_plugins(MenuPlugin)
            .init_resource::<Playback>()
            .insert_resource(SimStepTimer(Instant::now()))
            .init_resource::<S::State>()
            .insert_resource(SimulationResource { simulation })
            .add_systems(OnEnter(AppState::InGame), playback::setup_playback_resource)
            .add_systems(
                Update,
                playback::ensure_playback_resource.run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                (
                    Playback::inc_tick.in_set(SimSystemSet::Tick),
                    run_sim_step::<S>.in_set(SimSystemSet::Step),
                ),
            )
            .configure_sets(
                Update,
                (
                    // Simulation systems only run on tick
                    (
                        SimSystemSet::Tick,
                        SimSystemSet::PreStep,
                        SimSystemSet::Step,
                        SimSystemSet::PostStep,
                    )
                        .chain()
                        .run_if(Playback::should_step),
                    // Handle commands and per frame systems run every frame
                    SimSystemSet::HandleCommands,
                    SimSystemSet::PerFrame,
                )
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            );
    }
}



fn run_sim_step<S: Simulation>(
    mut state: ResMut<S::State>,
    mut actions: EventReader<S::Actions>,
    mut events: EventWriter<S::Event>,
    mut simulation: ResMut<SimulationResource<S>>,
) {
    let actions = actions.read().collect::<VecDeque<_>>();
    let (new_state, new_events) = simulation.simulation.step(state.clone(), actions);
    *state = new_state;
    for event in new_events {
        events.write(event);
    }
}

#[derive(Resource)]
struct JournalConfig {
    pub path: PathBuf,
}

impl Default for JournalConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("journal.json"),
        }
    }
}
