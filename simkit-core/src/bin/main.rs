use std::collections::VecDeque;

use bevy::prelude::*;
use simkit_core::{CorePlugin, Simulation};

struct MySimulation;

#[derive(Debug, Clone, Reflect, Default, Resource, Hash, PartialEq, Eq)]
struct MyState;

#[derive(Debug, Clone, Reflect, Event, Default, Hash, PartialEq, Eq)]
struct MyAction;

#[derive(Debug, Clone, Reflect, Event, Hash, PartialEq, Eq)]
struct MyEvent;

impl Simulation for MySimulation {
    type State = MyState;
    type Actions = MyAction;
    type Event = MyEvent;

    fn step(&mut self, state: Self::State, actions: VecDeque<&Self::Actions>) -> (Self::State, VecDeque<Self::Event>) {
        info!("Stepping");
        (state, VecDeque::new())
    }
}
fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(CorePlugin::new(MySimulation));
    app.run();
}
