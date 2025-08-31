mod cleanup;
mod loader;
pub mod model;

use crate::scenario::model::{Item, Pawn, Zone};
use bevy::prelude::*;
use simkit_core::ids::{IdAllocator, IdIndex, ItemId, PawnId, ZoneId};
use simkit_core::AppState;

pub use loader::LoadedScenarioMeta;

pub struct ScenarioPlugin;

impl Plugin for ScenarioPlugin {
    fn build(&self, app: &mut App) {
        app
            // Ensure ID allocators and indices are present
            .init_resource::<IdAllocator<PawnId>>()
            .init_resource::<IdIndex<PawnId>>()
            .init_resource::<IdAllocator<ItemId>>()
            .init_resource::<IdIndex<ItemId>>()
            .init_resource::<IdAllocator<ZoneId>>()
            .init_resource::<IdIndex<ZoneId>>()
            // Load scenario and world on enter (live mode)
            .add_systems(OnEnter(AppState::InGame), loader::load_scenario)
            // In headless mode (no states), ensure loading occurs at Startup
            .add_systems(Startup, loader::load_scenario_if_headless)
            // Cleanup on exit
            .add_systems(OnExit(AppState::InGame), cleanup::cleanup_world)
            // Maintain IdIndex on add/remove
            .add_systems(
                Update,
                (index_on_add_pawn, index_on_add_item, index_on_add_zone),
            )
            .add_systems(
                Update,
                (
                    index_on_remove_pawn,
                    index_on_remove_item,
                    index_on_remove_zone,
                ),
            );
    }
}

fn index_on_add_pawn(mut idx: ResMut<IdIndex<PawnId>>, q: Query<(Entity, &Pawn), Added<Pawn>>) {
    for (_e, p) in &q {
        idx.insert(p.0, _e);
    }
}
fn index_on_add_item(mut idx: ResMut<IdIndex<ItemId>>, q: Query<(Entity, &Item), Added<Item>>) {
    for (_e, it) in &q {
        idx.insert(it.id, _e);
    }
}
fn index_on_add_zone(mut idx: ResMut<IdIndex<ZoneId>>, q: Query<(Entity, &Zone), Added<Zone>>) {
    for (_e, z) in &q {
        idx.insert(z.id, _e);
    }
}

fn index_on_remove_pawn(mut idx: ResMut<IdIndex<PawnId>>, mut removed: RemovedComponents<Pawn>) {
    for e in removed.read() {
        if let Some((k, _)) = idx.0.iter().find(|(_k, v)| **v == e).map(|(k, v)| (*k, *v)) {
            idx.0.remove(&k);
        }
    }
}
fn index_on_remove_item(mut idx: ResMut<IdIndex<ItemId>>, mut removed: RemovedComponents<Item>) {
    for e in removed.read() {
        if let Some((k, _)) = idx.0.iter().find(|(_k, v)| **v == e).map(|(k, v)| (*k, *v)) {
            idx.0.remove(&k);
        }
    }
}
fn index_on_remove_zone(mut idx: ResMut<IdIndex<ZoneId>>, mut removed: RemovedComponents<Zone>) {
    for e in removed.read() {
        if let Some((k, _)) = idx.0.iter().find(|(_k, v)| **v == e).map(|(k, v)| (*k, *v)) {
            idx.0.remove(&k);
        }
    }
}
