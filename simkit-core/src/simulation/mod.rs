pub mod journal;

use std::{collections::VecDeque, sync::Arc};

use bevy::prelude::*;
pub use journal::{Journal, JournalConfig, JournalLine};

use crate::{AppState, KitSystemSet, Tick, POD};

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

pub struct SimPlugin<S: Simulation> {
    pub simulation: Arc<std::sync::Mutex<Option<S>>>,
}

impl<S: Simulation> SimPlugin<S> {
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

impl<S: Simulation> Plugin for SimPlugin<S> {
    fn build(&self, app: &mut App) {
        let mut simulation = self.simulation.lock().unwrap();
        let simulation = std::mem::take(&mut *simulation).unwrap();

        app.init_resource::<JournalConfig>()
            .init_resource::<S::State>()
            .insert_resource(SimulationResource { simulation })
            .add_event::<S::Event>()
            .add_event::<S::Actions>()
            .add_systems(
                OnEnter(AppState::InGame),
                (JournalConfig::init_journal_file::<S>,),
            )
            .add_systems(
                FixedUpdate,
                run_sim_step::<S>.in_set(KitSystemSet::Step),
            );
    }
}

fn run_sim_step<S: Simulation>(
    tick: Res<Tick>,
    mut state: ResMut<S::State>,
    mut actions: EventReader<S::Actions>,
    mut events: EventWriter<S::Event>,
    mut simulation: ResMut<SimulationResource<S>>,
    journal_config: Res<JournalConfig>,
) {
    let actions = actions.read().collect::<Vec<_>>();
    let (new_state, new_events) =
        simulation.simulation.step(*tick, state.clone(), &actions);
    *state = new_state;

    JournalConfig::write_update::<S>(
        journal_config,
        &state,
        &actions,
        &new_events,
    );

    for event in new_events {
        events.write(event);
    }
}
