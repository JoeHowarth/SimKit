#![allow(clippy::type_complexity)]
use std::{fmt::Debug, hash::Hash};

use bevy::{
    ecs::schedule::ScheduleLabel,
    platform::collections::HashMap,
    prelude::*,
};
use bevy_asset_loader::loading_state::{LoadingState, LoadingStateAppExt};

pub mod fixed_point;
pub mod grid;
pub mod ids;
pub mod menu;
pub mod pathfinding;
pub mod playback;
pub mod pod;
pub mod simulation;
pub mod snapshot;

pub use menu::MenuPlugin;
pub use playback::{PlayBackCommand, Playback};
pub use pod::POD;

use crate::playback::setup_playback_resource;

crate::pod! {
#[derive(Copy, Resource)]
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

#[derive(Resource)]
struct StepSystemLabelResource<S: ScheduleLabel + Clone>(S);

pub struct KitCoreBase<S: ScheduleLabel + Clone = FixedUpdate> {
    pub use_states: bool,
    pub with_menu: bool,
    pub step_system_label: S,
}

impl Default for KitCoreBase {
    fn default() -> Self {
        Self {
            use_states: true,
            with_menu: true,
            step_system_label: FixedUpdate,
        }
    }
}

impl<S: ScheduleLabel + Clone> Plugin for KitCoreBase<S> {
    fn build(&self, app: &mut App) {
        if self.with_menu {
            app.add_plugins(MenuPlugin);
        }

        app.register_type::<Tick>()
            .register_type::<GameId>()
            .register_type::<KitCommandType>()
            .register_type::<KitCommand>()
            .register_type::<Playback>()
            .add_event::<KitCommand>()
            .insert_resource(StepSystemLabelResource(
                self.step_system_label.clone(),
            ))
            .init_resource::<Playback>()
            .init_resource::<KeyCodeToCommandMap>();

        if self.use_states {
            app.insert_state(AppState::AssetLoading)
                .add_loading_state(
                    LoadingState::new(AppState::AssetLoading)
                        .continue_to_state(AppState::Menu),
                )
                .add_systems(
                    OnEnter(AppState::InGame),
                    (playback::setup_playback_resource,),
                );
        } else {
            app.add_systems(Startup, playback::setup_playback_resource);
        }

        configure_sets(app, self.step_system_label.clone(), self.use_states);

        if self.use_states {
            app.add_systems(
                Update,
                playback::ensure_playback_resource
                    .run_if(in_state(AppState::InGame)),
            );
        } else {
            app.add_systems(Update, playback::ensure_playback_resource);
        }
    }
}

pub fn configure_sets<S: ScheduleLabel + Clone>(
    app: &mut App,
    step_system_label: S,
    use_states: bool,
) {
    let mut fixed = (
        KitSystemSet::Tick,
        KitSystemSet::PreStep,
        KitSystemSet::Step,
        KitSystemSet::PostStep,
    )
        .chain();
    if use_states {
        fixed = fixed
            .run_if(in_state(AppState::InGame))
            .run_if(Playback::should_step);
    }
    app.configure_sets(step_system_label.clone(), fixed);
    app.add_systems(
        step_system_label,
        Playback::inc_tick.in_set(KitSystemSet::Tick),
    );
    setup_playback_resource(app.world_mut().commands());

    let mut update =
        (KitSystemSet::HandleCommands, KitSystemSet::PerFrame).chain();
    if use_states {
        update = update.run_if(in_state(AppState::InGame));
    }
    app.configure_sets(Update, update);
}

pub struct KitCorePlugin;
impl Plugin for KitCorePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(KitCoreBase::default());
    }
}

pub struct KitCoreHeadlessPlugin;
impl Plugin for KitCoreHeadlessPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(KitCoreBase {
            use_states: false,
            with_menu: false,
            step_system_label: Update,
        });
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
                    PlayBack(PlayBackCommand::TimePerTickMultiplier(
                        1.2.into(),
                    )),
                ),
                (
                    BracketRight,
                    PlayBack(PlayBackCommand::TimePerTickMultiplier(
                        0.8.into(),
                    )),
                ),
            ]),
        }
    }
}
