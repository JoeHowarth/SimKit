use bevy::prelude::*;
use simkit_core::{grid::index::TileMapIndex, ids::IdIndex};

use super::loader::LoadedScenarioMeta;
use crate::{model::ids::*, world::WorldGrid, WorldTag};

pub fn cleanup_world(
    mut commands: Commands,
    tagged: Query<Entity, With<WorldTag>>,
) {
    // Despawn all runtime-tagged entities
    for e in tagged.iter() {
        commands.entity(e).despawn();
    }

    // Clear/reset resources we own
    commands
        .insert_resource::<LoadedScenarioMeta>(LoadedScenarioMeta::default());
    // commands.insert_resource::<IdAllocator<PawnId>>(Default::default());
    // commands.insert_resource::<IdAllocator<ItemId>>(Default::default());
    // commands.insert_resource::<IdAllocator<ZoneId>>(Default::default());
    commands.insert_resource::<IdIndex<PawnId>>(Default::default());
    commands.insert_resource::<IdIndex<ItemId>>(Default::default());
    commands.insert_resource::<IdIndex<FixtureId>>(Default::default());
    commands.insert_resource::<IdIndex<TaskId>>(Default::default());

    // Remove world-grid and occupancy resources
    commands.remove_resource::<WorldGrid>();
    commands.remove_resource::<TileMapIndex<PawnId>>();
    commands.remove_resource::<TileMapIndex<ItemId>>();
    commands.remove_resource::<TileMapIndex<FixtureId>>();
}
