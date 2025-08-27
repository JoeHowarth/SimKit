use crate::{CommandType, SimCommand, Tick};
use bevy::{prelude::*, time::Stopwatch};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Resource)]
pub struct Playback {
    /// The current tick
    pub tick: Tick,
    /// Time per tick
    pub time_per_tick: Duration,
    pub is_paused: bool,
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            tick: Tick(0),
            time_per_tick: Duration::from_millis(100),
            is_paused: false,
        }
    }
}

impl Playback {
    pub fn should_step(playback: Res<Playback>, time: Res<SimStepTimer>) -> bool {
        dbg!(
            playback.is_paused,
            time.0.elapsed().as_millis(),
            playback.time_per_tick.as_millis()
        );
        !playback.is_paused && time.0.elapsed() > playback.time_per_tick
    }

    pub fn inc_tick(mut playback: ResMut<Playback>, mut time: ResMut<SimStepTimer>) {
        playback.tick.0 += 1;
        time.0 = Instant::now();
    }
}

#[derive(Resource)]
pub struct SimStepTimer(pub Instant);

#[derive(Debug, Clone, Reflect)]
pub enum PlayBackCommand {
    SetTimePerTick(Duration),
    SetPaused(bool),
    TogglePaused,
}

pub fn setup_playback_resource(mut commands: Commands) {
    commands.insert_resource(Playback {
        tick: Tick(0),
        time_per_tick: Duration::from_millis(10),
        is_paused: false,
    });
}

pub fn ensure_playback_resource(
    mut playback: ResMut<Playback>,
    mut event: EventReader<SimCommand>,
) {
    for sim_command in event.read() {
        match sim_command.command_type {
            CommandType::PlayBack(PlayBackCommand::SetTimePerTick(time_per_tick)) => {
                info!("Setting time per tick to {:?}", time_per_tick);
                playback.time_per_tick = time_per_tick;
            }
            CommandType::PlayBack(PlayBackCommand::SetPaused(paused)) => {
                info!("Setting paused to {:?}", paused);
                playback.is_paused = paused;
            }
            CommandType::PlayBack(PlayBackCommand::TogglePaused) => {
                info!(
                    before = playback.is_paused,
                    after = !playback.is_paused,
                    "Toggling paused"
                );
                playback.is_paused = !playback.is_paused;
            }
            _ => {}
        };
    }
}
