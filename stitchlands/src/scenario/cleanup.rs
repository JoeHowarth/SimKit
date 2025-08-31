use bevy::prelude::*;

use crate::WorldTag;

use super::loader::LoadedScenarioMeta;
use simkit_core::ids::{IdAllocator, IdIndex, ItemId, PawnId, ZoneId};

pub fn cleanup_world(mut commands: Commands, tagged: Query<Entity, With<WorldTag>>) {
    // Despawn all runtime-tagged entities
    for e in tagged.iter() {
        commands.entity(e).despawn();
    }

    // Clear/reset resources we own
    commands.insert_resource::<LoadedScenarioMeta>(LoadedScenarioMeta::default());
    commands.insert_resource::<IdAllocator<PawnId>>(Default::default());
    commands.insert_resource::<IdAllocator<ItemId>>(Default::default());
    commands.insert_resource::<IdAllocator<ZoneId>>(Default::default());
    commands.insert_resource::<IdIndex<PawnId>>(Default::default());
    commands.insert_resource::<IdIndex<ItemId>>(Default::default());
    commands.insert_resource::<IdIndex<ZoneId>>(Default::default());
}
