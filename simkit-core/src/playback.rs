use std::time::Duration;

use bevy::prelude::*;

use crate::{KitCommand, KitCommandType, Tick, fixed_point::FP64};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Resource)]
pub struct Playback {
    /// Time per tick
    pub time_per_tick: Duration,
    pub is_paused: bool,
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            time_per_tick: Duration::from_millis(100),
            is_paused: false,
        }
    }
}

impl Playback {
    pub fn should_step(playback: Option<Res<Playback>>) -> bool {
        let Some(playback) = playback else {
            return true;
        };

        if playback.is_paused {
            debug!("Playback is paused");
        }
        !playback.is_paused
    }

    pub fn inc_tick(mut tick: ResMut<Tick>) {
        tick.0 += 1;
    }
}

crate::pod! {
pub enum PlayBackCommand {
    SetTimePerTick(Duration),
    TimePerTickMultiplier(FP64),
    SetPaused(bool),
    TogglePaused,
}}

pub fn setup_playback_resource(mut commands: Commands) {
    commands.init_resource::<Playback>();
    commands.insert_resource(Tick(0));
}

pub fn ensure_playback_resource(
    mut playback: ResMut<Playback>,
    mut event: EventReader<KitCommand>,
    // get mutable fixed update timer for builin bevy FixedUpdate schedule
    mut fixed_update_timer: ResMut<Time<Fixed>>,
) {
    for sim_command in event.read() {
        let KitCommandType::PlayBack(cmd) = &sim_command.command_type else {
            continue;
        };
        match cmd {
            PlayBackCommand::SetTimePerTick(time_per_tick) => {
                info!("Setting time per tick to {:?}", time_per_tick);
                playback.time_per_tick = *time_per_tick;
            }
            PlayBackCommand::SetPaused(paused) => {
                info!("Setting paused to {:?}", paused);
                playback.is_paused = *paused;
            }
            PlayBackCommand::TogglePaused => {
                info!(
                    before = playback.is_paused,
                    after = !playback.is_paused,
                    "Toggling paused"
                );
                playback.is_paused = !playback.is_paused;
            }
            PlayBackCommand::TimePerTickMultiplier(mult) => {
                let millis = playback.time_per_tick.as_millis() as i64;
                let scaled = *mult * millis;
                playback.time_per_tick = Duration::from_millis(scaled.into());
            }
        }
    }

    fixed_update_timer.set_timestep(playback.time_per_tick);
}
