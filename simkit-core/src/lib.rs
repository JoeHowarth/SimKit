use std::{collections::VecDeque, fmt::Debug, hash::Hash, marker::PhantomData, sync::Arc};

use bevy::{
    platform::collections::HashMap,
    prelude::*,
    reflect::{reflect_remote, GetTypeRegistration, Typed},
};
use bevy_asset_loader::loading_state::{LoadingState, LoadingStateAppExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub mod fixed_point;
pub mod menu;
pub mod playback;
pub mod pod;
pub mod simulation;

pub use menu::MenuPlugin;
pub use playback::{PlayBackCommand, Playback};
pub use pod::POD;

crate::pod! {
#[derive(Copy)]
pub struct Tick(pub i32);

pub struct GameId(pub String);

pub enum KitCommandType {
    PlayBack(PlayBackCommand),
    LaunchGame,
    LoadSave(GameId),
    Save,
    ExitToMenu,
}

#[derive(Event)]
pub struct KitCommand {
    pub command_type: KitCommandType,
    pub tick_sent: Tick,
}

#[derive(States)]
pub enum AppState {
    InGame,
    Menu,
    AssetLoading,
}

// System sets to run before and after the sim step
#[derive(SystemSet)]
pub enum KitSystemSet {
    Tick,
    PreStep,
    Step,
    PostStep,
    HandleCommands,
    PerFrame,
}
}

pub struct KitCoreBase {
    pub use_states: bool,
    pub with_menu: bool,
}

impl Default for KitCoreBase {
    fn default() -> Self {
        Self { use_states: true, with_menu: true }
    }
}

impl Plugin for KitCoreBase {
    fn build(&self, app: &mut App) {
        if self.with_menu { app.add_plugins(MenuPlugin); }

        app.register_type::<Tick>()
            .register_type::<GameId>()
            .register_type::<KitCommandType>()
            .register_type::<KitCommand>()
            .register_type::<Playback>()
            .add_event::<KitCommand>()
            .init_resource::<Playback>()
            .init_resource::<KeyCodeToCommandMap>();

        if self.use_states {
            app.insert_state(AppState::AssetLoading)
                .add_loading_state(LoadingState::new(AppState::AssetLoading).continue_to_state(AppState::Menu))
                .add_systems(OnEnter(AppState::InGame), (playback::setup_playback_resource,));
        } else {
            app.add_systems(Startup, playback::setup_playback_resource);
        }

        let mut fixed = (
            KitSystemSet::Tick,
            KitSystemSet::PreStep,
            KitSystemSet::Step,
            KitSystemSet::PostStep,
        ).chain().run_if(Playback::should_step);
        if self.use_states { fixed = fixed.run_if(in_state(AppState::InGame)); }
        app.configure_sets(FixedUpdate, fixed);

        let mut update = (KitSystemSet::HandleCommands, KitSystemSet::PerFrame).chain();
        if self.use_states { update = update.run_if(in_state(AppState::InGame)); }
        app.configure_sets(Update, update);

        if self.use_states {
            app.add_systems(Update, playback::ensure_playback_resource.run_if(in_state(AppState::InGame)));
        } else {
            app.add_systems(Update, playback::ensure_playback_resource);
        }

        app.add_systems(FixedUpdate, (Playback::inc_tick.in_set(KitSystemSet::Tick),));
    }
}

pub struct KitCorePlugin;
impl Plugin for KitCorePlugin {
    fn build(&self, app: &mut App) { app.add_plugins(KitCoreBase::default()); }
}

pub struct KitCoreHeadlessPlugin;
impl Plugin for KitCoreHeadlessPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(KitCoreBase { use_states: false, with_menu: false });
    }
}

#[derive(Resource)]
pub struct KeyCodeToCommandMap {
    pub map: HashMap<KeyCode, KitCommandType>,
}

impl KeyCodeToCommandMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl Default for KeyCodeToCommandMap {
    fn default() -> Self {
        use KeyCode::*;
        use KitCommandType::*;
        Self {
            map: HashMap::from_iter([
                (Escape, ExitToMenu),
                (Enter, LaunchGame),
                (KeyP, PlayBack(PlayBackCommand::TogglePaused)),
                (
                    BracketLeft,
                    PlayBack(PlayBackCommand::TimePerTickMultiplier(1.2.into())),
                ),
                (
                    BracketRight,
                    PlayBack(PlayBackCommand::TimePerTickMultiplier(0.8.into())),
                ),
            ]),
        }
    }
}
