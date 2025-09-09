use bevy::prelude::*;
use simkit_core::{fixed_point::Q40p24, KitSystemSet};

use crate::{model::*, StepSystemLabel};

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

fn decrement_harvest_countdown(mut fixtures: Query<&mut Fixture>) {
    debug!("Decrementing harvest countdown");
    for mut fixture in fixtures.iter_mut() {
        if fixture.harvest_countdown.is_none() {
            continue;
        }
        let harvest_countdown = fixture.harvest_countdown.as_mut().unwrap();
        *harvest_countdown = harvest_countdown.saturating_sub(1);
    }
}
