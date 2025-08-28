use std::{
    collections::VecDeque,
    fmt::Debug,
    hash::Hash,
    io::{BufRead, Write},
    path::PathBuf,
    sync::Arc,
};

use bevy::{
    prelude::*,
    reflect::{GetTypeRegistration, Typed},
};
use bevy_asset_loader::loading_state::{LoadingState, LoadingStateAppExt};

pub mod playback;
pub use playback::{PlayBackCommand, Playback};

pub mod menu;
pub use menu::MenuPlugin;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
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
pub trait POD:
    Reflect
    + FromReflect
    + Debug
    + Clone
    + PartialEq
    + Eq
    + Hash
    + Send
    + Sync
    + Typed
    + GetTypeRegistration
    + Serialize
    + DeserializeOwned
    + 'static
{
}
impl<
        T: Reflect
            + FromReflect
            + Debug
            + Clone
            + PartialEq
            + Eq
            + Hash
            + Send
            + Sync
            + Typed
            + GetTypeRegistration
            + Serialize
            + DeserializeOwned
            + 'static,
    > POD for T
{
}

pub trait Simulation: Sync + Send + 'static {
    type State: POD + Resource + Default;
    type Actions: POD + Event;
    type Event: POD + Event;

    fn step(
        &mut self,
        tick: Tick,
        state: Self::State,
        actions: &[&Self::Actions],
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
            .init_resource::<JournalConfig>()
            .init_resource::<S::State>()
            .insert_resource(SimulationResource { simulation })
            .add_event::<S::Event>()
            .add_event::<S::Actions>()
            .add_systems(
                OnEnter(AppState::InGame),
                (
                    playback::setup_playback_resource,
                    JournalConfig::init_journal_file::<S>,
                ),
            )
            .add_systems(
                Update,
                playback::ensure_playback_resource.run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                FixedUpdate,
                (
                    Playback::inc_tick.in_set(SimSystemSet::Tick),
                    run_sim_step::<S>.in_set(SimSystemSet::Step),
                ),
            )
            .configure_sets(
                FixedUpdate,
                (
                    // Simulation systems only run on tick
                    SimSystemSet::Tick,
                    SimSystemSet::PreStep,
                    SimSystemSet::Step,
                    SimSystemSet::PostStep,
                )
                    .chain()
                    .run_if(Playback::should_step)
                    .run_if(in_state(AppState::InGame)),
            )
            .configure_sets(
                Update,
                (SimSystemSet::HandleCommands, SimSystemSet::PerFrame)
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

fn run_sim_step<S: Simulation>(
    playback: Res<Playback>,
    mut state: ResMut<S::State>,
    mut actions: EventReader<S::Actions>,
    mut events: EventWriter<S::Event>,
    mut simulation: ResMut<SimulationResource<S>>,
    journal_config: Res<JournalConfig>,
) {
    let actions = actions.read().collect::<Vec<_>>();
    let (new_state, new_events) =
        simulation
            .simulation
            .step(playback.tick, state.clone(), &actions);
    *state = new_state;

    JournalConfig::write_update::<S>(journal_config, &state, &actions, &new_events);

    for event in new_events {
        events.write(event);
    }
}

#[derive(Resource)]
struct JournalConfig {
    pub path: PathBuf,
    pub enabled: bool,
}

#[derive(Reflect, Clone, Serialize, Deserialize)]
#[serde(bound(
    serialize = "S::State: serde::Serialize, S::Actions: serde::Serialize, S::Event: serde::Serialize",
    deserialize = "S::State: serde::de::DeserializeOwned, S::Actions: serde::de::DeserializeOwned, S::Event: serde::de::DeserializeOwned"
))]
struct Journal<S: Simulation>(pub Vec<JournalLine<S::State, S::Actions, S::Event>>);

#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[serde(bound(
    serialize = "S: serde::Serialize, A: serde::Serialize, E: serde::Serialize",
    deserialize = "S: serde::de::DeserializeOwned, A: serde::de::DeserializeOwned, E: serde::de::DeserializeOwned"
))]
struct JournalLine<S: POD, A: POD, E: POD> {
    pub tick: Tick,
    pub state: S,
    pub actions: Vec<A>,
    pub events: Vec<E>,
}

impl JournalConfig {
    fn init_journal_file<S: Simulation>(
        journal_config: Res<JournalConfig>,
        init_state: Res<S::State>,
    ) {
        if !journal_config.enabled {
            return;
        }

        let file = std::fs::File::create(journal_config.path.clone()).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        let line = JournalLine::<S::State, S::Actions, S::Event> {
            tick: Tick(0),
            state: init_state.clone(),
            actions: Vec::new(),
            events: Vec::new(),
        };
        serde_json::to_writer(&mut writer, &line).unwrap();
        writer.write_all(b"\n").unwrap();
        writer.flush().unwrap();
    }

    fn write_update<'a, S: Simulation>(
        journal_config: Res<JournalConfig>,
        state: &S::State,
        actions: &[&S::Actions],
        events: impl IntoIterator<Item = &'a S::Event>,
    ) {
        dbg!("here");
        if !journal_config.enabled {
            debug!("Journal is disabled");
            info!("Journal is disabled");
            dbg!("journal is disabled");
            return;
        }

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(journal_config.path.clone())
            .unwrap();

        let mut writer = std::io::BufWriter::new(file);

        let line = JournalLine::<S::State, S::Actions, S::Event> {
            tick: Tick(0),
            state: state.clone(),
            actions: actions
                .into_iter()
                .map(|a: &&S::Actions| (*a).clone())
                .collect::<Vec<_>>(),
            events: events.into_iter().cloned().collect(),
        };

        info!(tick = line.tick.0, "Writing update to journal");
        serde_json::to_writer(&mut writer, &line).unwrap();

        writer.write_all(b"\n").unwrap();
        writer.flush().unwrap();
    }

    fn load_journal<'a, S: Simulation>(&self) -> Journal<S> {
        let file = std::fs::File::open(self.path.clone()).unwrap();
        let reader = std::io::BufReader::new(file);
        let mut lines = Vec::new();
        for line in reader.lines() {
            let line = line.unwrap();
            let line = serde_json::from_str(&line).unwrap();
            lines.push(line);
        }
        debug!("Loaded {} lines from journal", lines.len());
        Journal(lines)
    }
}

impl Default for JournalConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("journal.json"),
            enabled: true,
        }
    }
}
