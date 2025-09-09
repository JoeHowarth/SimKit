use std::collections::VecDeque;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use simkit_core::{
    simulation::{SimPlugin, Simulation},
    KitCorePlugin,
    Tick,
};

struct MySimulation;

#[derive(
    Debug,
    Clone,
    Reflect,
    Default,
    Resource,
    Hash,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
)]
struct MyState(i32);

#[derive(
    Debug,
    Clone,
    Reflect,
    Event,
    Default,
    Hash,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
)]
struct MyAction(i32);

#[derive(
    Debug, Clone, Reflect, Event, Hash, PartialEq, Eq, Serialize, Deserialize,
)]
struct MyEvent(i32);

impl Simulation for MySimulation {
    type State = MyState;
    type Actions = MyAction;
    type Event = MyEvent;

    fn step(
        &mut self,
        tick: Tick,
        state: Self::State,
        _actions: &[&Self::Actions],
    ) -> (Self::State, VecDeque<Self::Event>) {
        info!("Stepping: {}", tick.0);
        let mut queue = VecDeque::new();
        queue.push_back(MyEvent(state.0 * 2 + 1));

        (MyState(state.0 + 1), queue)
    }
}

pub fn camera_setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        bevy_pancam::PanCam {
            move_keys: bevy_pancam::DirectionKeys::wasd(),
            grab_buttons: vec![MouseButton::Right, MouseButton::Left],
            min_scale: 0.25,
            max_scale: 5.0,
            ..default()
        },
    ));
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(KitCorePlugin);
    app.add_plugins(SimPlugin::new(MySimulation));

    app.add_systems(Startup, camera_setup);
    app.run();
}
