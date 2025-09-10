use bevy::prelude::*;
use simkit_core::{KitSystemSet, fixed_point::Q40p24};

use crate::{StepSystemLabel, model::*};

pub struct EnvironmentStepPlugin;

impl Plugin for EnvironmentStepPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            StepSystemLabel::default(),
            (decrement_needs, decrement_harvest_countdown)
                .in_set(KitSystemSet::PostStep),
        );
    }
}

fn decrement_needs(mut pawns: Query<&mut Pawn>) {
    debug!("Decrementing needs");
    for mut pawn in pawns.iter_mut() {
        pawn.hunger = (pawn.hunger - Q40p24::ONE).max(Q40p24::ZERO);
        pawn.sleep = (pawn.sleep - Q40p24::ONE).max(Q40p24::ZERO);
    }
}

fn decrement_harvest_countdown(mut fixtures: Query<&mut HarvestCountdown>) {
    debug!("Decrementing harvest countdown");
    for mut harvest_countdown in fixtures.iter_mut() {
        harvest_countdown.0 = harvest_countdown.0.saturating_sub(1);
    }
}
