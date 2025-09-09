use bevy::{platform::collections::HashSet, prelude::*};
use simkit_core::{
    grid::{index::TileMapIndex, TileId},
    ids::IdIndex,
};

use crate::{
    model::{
        components::{Fixture, FixtureKind, Item, ItemRelation, Pawn},
        ids::{FixtureId, ItemId, PawnId},
    },
    world::WorldGrid,
};

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvariantMode {
    Panic,
    Warn,
    Silent,
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvariantConfig {
    pub mode: InvariantMode,
}

impl Default for InvariantConfig {
    fn default() -> Self {
        Self {
            mode: InvariantMode::Panic,
        }
    }
}

pub struct WorldInvariantPlugin;

impl Plugin for WorldInvariantPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InvariantConfig>()
            .add_systems(PostUpdate, run_invariant_checks);
    }
}

fn run_invariant_checks(world: &mut World) {
    // Run validation, emit or panic based on config
    let cfg = world
        .get_resource::<InvariantConfig>()
        .cloned()
        .unwrap_or_default();
    let errors = validate_world(world);
    if errors.is_empty() {
        return;
    }
    match cfg.mode {
        InvariantMode::Panic => {
            panic!(
                "Invariant violations ({}):\n{}",
                errors.len(),
                errors.join("\n")
            );
        }
        InvariantMode::Warn => {
            for e in errors {
                warn!("Invariant violation: {}", e);
            }
        }
        InvariantMode::Silent => {}
    }
}

pub fn validate_world(world: &mut World) -> Vec<String> {
    let mut errors = Vec::new();

    // Collect inventories for cross-checks
    let mut pawn_inventory_items = std::collections::HashMap::new();
    {
        let mut q = world.query::<&Pawn>();
        for pawn in q.iter(world) {
            for (iid, _) in pawn.inventory.0.iter().copied() {
                pawn_inventory_items.insert(iid, pawn.id);
            }
        }
    }
    let mut fixture_inventory_items = std::collections::HashMap::new();
    {
        let mut q = world.query::<&Fixture>();
        for fix in q.iter(world) {
            for (iid, _) in fix.inventory.0.iter().copied() {
                fixture_inventory_items.insert(iid, fix.id);
            }
        }
    }

    // Pawn invariants (collect first to avoid borrow conflicts)
    let pawn_infos: Vec<(Entity, PawnId, Option<TileId>)> = {
        let mut out = Vec::new();
        let mut q = world.query::<(Entity, &Pawn, Option<&TileId>)>();
        for (e, pawn, pos_opt) in q.iter(world) {
            out.push((e, pawn.id, pos_opt.copied()));
        }
        out
    };
    {
        let pawn_index = world.resource::<IdIndex<PawnId>>();
        let pawn_tile_index = world.resource::<TileMapIndex<PawnId>>();
        let grid = world.get_resource::<WorldGrid>().cloned();
        for (e, pid, pos_opt) in pawn_infos.iter().copied() {
            if pos_opt.is_none() {
                errors.push(format!("Pawn {:?} missing TileId", pid));
            }
            let ent = pawn_index.get(&pid);
            if ent != e {
                errors.push(format!(
                    "Pawn {:?} IdIndex mismatch: index={:?} entity={:?}",
                    pid, ent, e
                ));
            }
            if let Some(pos) = pos_opt {
                if let Some(grid) = &grid {
                    if pos.x < 0
                        || pos.y < 0
                        || pos.x >= grid.cfg.width as i32
                        || pos.y >= grid.cfg.height as i32
                    {
                        errors.push(format!(
                            "Pawn {:?} out of bounds at {:?}",
                            pid, pos
                        ));
                    }
                }
                if pawn_tile_index.get(pos) != Some(pid) {
                    errors.push(format!(
                        "Pawn {:?} tile index mismatch at {:?}",
                        pid, pos
                    ));
                }
            }
        }
    }

    // Fixture invariants (collect first)
    let fixture_infos: Vec<(
        Entity,
        FixtureId,
        FixtureKind,
        Option<u32>,
        Option<TileId>,
    )> = {
        let mut out = Vec::new();
        let mut q = world.query::<(Entity, &Fixture, Option<&TileId>)>();
        for (e, fixture, pos_opt) in q.iter(world) {
            out.push((
                e,
                fixture.id,
                fixture.kind.clone(),
                fixture.harvest_countdown,
                pos_opt.copied(),
            ));
        }
        out
    };
    {
        let fixture_index = world.resource::<IdIndex<FixtureId>>();
        let fixture_tile_index = world.resource::<TileMapIndex<FixtureId>>();
        let grid = world.get_resource::<WorldGrid>().cloned();
        for (e, fid, kind, countdown, pos_opt) in fixture_infos.iter().cloned()
        {
            if pos_opt.is_none() {
                errors.push(format!("Fixture {:?} missing TileId", fid));
            }
            let ent = fixture_index.get(&fid);
            if ent != e {
                errors.push(format!(
                    "Fixture {:?} IdIndex mismatch: index={:?} entity={:?}",
                    fid, ent, e
                ));
            }
            if matches!(kind, FixtureKind::BerryBush) && countdown.is_none() {
                errors.push(format!(
                    "BerryBush {:?} has None harvest_countdown; expected \
                     Some(>0 or reset)",
                    fid
                ));
            }
            if let Some(pos) = pos_opt {
                if let Some(grid) = &grid {
                    if pos.x < 0
                        || pos.y < 0
                        || pos.x >= grid.cfg.width as i32
                        || pos.y >= grid.cfg.height as i32
                    {
                        errors.push(format!(
                            "Fixture {:?} out of bounds at {:?}",
                            fid, pos
                        ));
                    }
                }
                if fixture_tile_index.get(pos) != Some(fid) {
                    errors.push(format!(
                        "Fixture {:?} tile index mismatch at {:?}",
                        fid, pos
                    ));
                }
            }
        }
    }

    // Item invariants (collect first)
    let item_infos: Vec<(Entity, ItemId, u32, ItemRelation)> = {
        let mut q = world.query::<(Entity, &Item, &ItemRelation)>();
        q.iter(world)
            .map(|(e, i, &r)| (e, i.id, i.qty, r))
            .collect()
    };
    {
        let item_index = world.resource::<IdIndex<ItemId>>();
        let item_tile_index = world.resource::<TileMapIndex<ItemId>>();
        let grid = world.get_resource::<WorldGrid>().cloned();
        for (e, iid, qty, relation) in item_infos.iter() {
            let ent = item_index.get(iid);
            if ent != *e {
                errors.push(format!(
                    "Item {:?} IdIndex mismatch: index={:?} entity={:?}",
                    iid, ent, e
                ));
            }
            if *qty == 0 {
                errors.push(format!("Item {:?} has qty=0", iid));
            }

            match relation {
                ItemRelation::OnGround(p) => {
                    let p = *p;
                    if let Some(grid) = &grid {
                        if p.x < 0
                            || p.y < 0
                            || p.x >= grid.cfg.width as i32
                            || p.y >= grid.cfg.height as i32
                        {
                            errors.push(format!(
                                "Item {:?} out of bounds at {:?}",
                                iid, p
                            ));
                        }
                    }
                    if item_tile_index.get(p) != Some(*iid) {
                        errors.push(format!(
                            "Item {:?} tile index mismatch at {:?}",
                            iid, p
                        ));
                    }
                    if pawn_inventory_items.contains_key(iid)
                        || fixture_inventory_items.contains_key(iid)
                    {
                        errors.push(format!(
                            "Item {:?} on ground but appears in an inventory",
                            iid
                        ));
                    }
                }
                ItemRelation::CarriedBy(pid) => {
                    match pawn_inventory_items.get(iid) {
                        Some(owner) if *owner == *pid => {}
                        Some(owner) => errors.push(format!(
                            "Item {:?} carried by {:?} but appears in pawn \
                             {:?} inventory",
                            iid, pid, owner
                        )),
                        None => errors.push(format!(
                            "Item {:?} carried by {:?} but not in pawn \
                             inventory",
                            iid, pid
                        )),
                    }
                    if fixture_inventory_items.contains_key(iid) {
                        errors.push(format!(
                            "Item {:?} carried by {:?} but appears in a \
                             fixture inventory",
                            iid, pid
                        ));
                    }
                }
                ItemRelation::InFixture(fid) => {
                    match fixture_inventory_items.get(iid) {
                        Some(owner) if *owner == *fid => {}
                        Some(owner) => errors.push(format!(
                            "Item {:?} in fixture {:?} but appears in fixture \
                             {:?} inventory",
                            iid, fid, owner
                        )),
                        None => errors.push(format!(
                            "Item {:?} in fixture {:?} but not in its \
                             inventory",
                            iid, fid
                        )),
                    }
                    if pawn_inventory_items.contains_key(iid) {
                        errors.push(format!(
                            "Item {:?} in fixture {:?} but appears in a pawn \
                             inventory",
                            iid, fid
                        ));
                    }
                }
            }
        }

        // Additional cross-check: scan the entire item tile index to ensure it
        // is consistent with ItemRelation state for each item.
        let mut seen: std::collections::HashMap<ItemId, TileId> =
            std::collections::HashMap::new();
        let cfg = item_tile_index.0.cfg;
        for y in 0..(cfg.height as i32) {
            for x in 0..(cfg.width as i32) {
                let t = TileId::new(x, y);
                if let Some(Some(id)) = item_tile_index.0.get(t).copied() {
                    if let Some(prev) = seen.insert(id, t) {
                        errors.push(format!(
                            "Item {:?} appears multiple times in tile index \
                             at {:?} and {:?}",
                            id, prev, t
                        ));
                    }
                    let ent = item_index.get(&id);
                    // Relation must be present and OnGround at the same tile
                    match world.get::<ItemRelation>(ent) {
                        Some(ItemRelation::OnGround(pos)) => {
                            if *pos != t {
                                errors.push(format!(
                                    "Item {:?} in tile index at {:?} but \
                                     relation OnGround at {:?}",
                                    id, t, pos
                                ));
                            }
                        }
                        Some(other) => errors.push(format!(
                            "Item {:?} in tile index at {:?} but relation is \
                             {:?}",
                            id, t, other
                        )),
                        None => errors.push(format!(
                            "Item {:?} in tile index at {:?} but missing \
                             ItemRelation",
                            id, t
                        )),
                    }
                }
            }
        }

        // Reconcile counts: every OnGround relation must have a corresponding
        // tile index entry, and every tile index entry must correspond to an
        // OnGround relation.
        let onground_ids: HashSet<ItemId> = item_infos
            .iter()
            .filter_map(|(_, iid, _, rel)| match rel {
                ItemRelation::OnGround(_) => Some(*iid),
                _ => None,
            })
            .collect();
        let index_ids: HashSet<ItemId> = seen.keys().copied().collect();
        for id in onground_ids.iter() {
            if !index_ids.contains(id) {
                errors.push(format!(
                    "Item {:?} relation OnGround but missing from tile index",
                    id
                ));
            }
        }
        for id in index_ids.iter() {
            if !onground_ids.contains(id) {
                errors.push(format!(
                    "Item {:?} present in tile index but relation is not \
                     OnGround",
                    id
                ));
            }
        }

        // Inventory reconciliation
        // Ensure no item appears in both pawn and fixture inventories
        for (iid, pid) in pawn_inventory_items.iter() {
            if let Some(fid) = fixture_inventory_items.get(iid) {
                errors.push(format!(
                    "Item {:?} appears in both pawn {:?} and fixture {:?} \
                     inventories",
                    iid, pid, fid
                ));
            }
        }

        // For each item listed in a pawn inventory, ensure entity+relation
        // exist
        for (iid, pid) in pawn_inventory_items.iter() {
            let ent_opt = item_index.0.get(iid).and_then(|e| *e);
            match ent_opt {
                None => errors.push(format!(
                    "Item {:?} in pawn {:?} inventory but missing from IdIndex",
                    iid, pid
                )),
                Some(ent) => match world.get::<ItemRelation>(ent) {
                    Some(ItemRelation::CarriedBy(owner)) if *owner == *pid => {}
                    Some(rel) => errors.push(format!(
                        "Item {:?} in pawn {:?} inventory but relation is {:?}",
                        iid, pid, rel
                    )),
                    None => errors.push(format!(
                        "Item {:?} in pawn {:?} inventory but missing \
                         ItemRelation",
                        iid, pid
                    )),
                },
            }
            if index_ids.contains(iid) {
                errors.push(format!(
                    "Item {:?} in pawn {:?} inventory but present in tile \
                     index",
                    iid, pid
                ));
            }
        }

        // For each item listed in a fixture inventory, ensure entity+relation
        // exist
        for (iid, fid) in fixture_inventory_items.iter() {
            let ent_opt = item_index.0.get(iid).and_then(|e| *e);
            match ent_opt {
                None => errors.push(format!(
                    "Item {:?} in fixture {:?} inventory but missing from \
                     IdIndex",
                    iid, fid
                )),
                Some(ent) => match world.get::<ItemRelation>(ent) {
                    Some(ItemRelation::InFixture(owner)) if *owner == *fid => {}
                    Some(rel) => errors.push(format!(
                        "Item {:?} in fixture {:?} inventory but relation is \
                         {:?}",
                        iid, fid, rel
                    )),
                    None => errors.push(format!(
                        "Item {:?} in fixture {:?} inventory but missing \
                         ItemRelation",
                        iid, fid
                    )),
                },
            }
            if index_ids.contains(iid) {
                errors.push(format!(
                    "Item {:?} in fixture {:?} inventory but present in tile \
                     index",
                    iid, fid
                ));
            }
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use simkit_core::{
        fixed_point::Q40p24,
        grid::{Grid2D, GridConfig},
    };

    use super::*;
    use crate::model::{components::ItemRelation::*, *};

    fn setup_indices(app: &mut App, width: u32, height: u32) {
        app.insert_resource(WorldGrid {
            cfg: GridConfig { width, height },
            walkable: Grid2D::new(GridConfig { width, height }, true),
        })
        .insert_resource(TileMapIndex::<PawnId>::new(GridConfig {
            width,
            height,
        }))
        .insert_resource(TileMapIndex::<ItemId>::new(GridConfig {
            width,
            height,
        }))
        .insert_resource(TileMapIndex::<FixtureId>::new(GridConfig {
            width,
            height,
        }))
        .init_resource::<IdIndex<PawnId>>()
        .init_resource::<IdIndex<ItemId>>()
        .init_resource::<IdIndex<FixtureId>>();
    }

    #[test]
    fn valid_world_passes() {
        let mut app = App::new();
        setup_indices(&mut app, 5, 5);

        // Spawn a pawn at (1,1) with a carried item
        let pawn_id = {
            let mut idx = app.world_mut().resource_mut::<IdIndex<PawnId>>();
            idx.alloc(None)
        };
        let carried_item_id = {
            let mut idx = app.world_mut().resource_mut::<IdIndex<ItemId>>();
            idx.alloc(None)
        };
        let pawn_entity = app
            .world_mut()
            .spawn((
                crate::WorldTag,
                Pawn {
                    id: pawn_id,
                    inventory: Inventory(vec![(
                        carried_item_id,
                        crate::model::components::ItemKind::Berry,
                    )]),
                    sleep: Q40p24::ONE,
                    hunger: Q40p24::ONE,
                },
                TileId::new(1, 1),
            ))
            .id();
        app.world_mut()
            .resource_mut::<IdIndex<PawnId>>()
            .insert(pawn_id, pawn_entity);
        app.world_mut()
            .resource_mut::<TileMapIndex<PawnId>>()
            .move_id(None, TileId::new(1, 1), pawn_id);

        let item_entity = app
            .world_mut()
            .spawn((
                crate::WorldTag,
                Item {
                    id: carried_item_id,
                    kind: crate::model::components::ItemKind::Berry,
                    qty: 1,
                },
                ItemRelation::CarriedBy(pawn_id),
            ))
            .id();
        app.world_mut()
            .resource_mut::<IdIndex<ItemId>>()
            .insert(carried_item_id, item_entity);

        // Spawn a fixture at (2,2) with an inventory item and be a BerryBush
        // with countdown
        let fixture_id = {
            let mut idx = app.world_mut().resource_mut::<IdIndex<FixtureId>>();
            idx.alloc(None)
        };
        let fixture_item_id = {
            let mut idx = app.world_mut().resource_mut::<IdIndex<ItemId>>();
            idx.alloc(None)
        };
        let fixture_entity = app
            .world_mut()
            .spawn((
                crate::WorldTag,
                Fixture {
                    id: fixture_id,
                    kind: FixtureKind::BerryBush,
                    inventory: Inventory(vec![(
                        fixture_item_id,
                        crate::model::components::ItemKind::Berry,
                    )]),
                    harvest_countdown: Some(100),
                },
                TileId::new(2, 2),
            ))
            .id();
        app.world_mut()
            .resource_mut::<IdIndex<FixtureId>>()
            .insert(fixture_id, fixture_entity);
        app.world_mut()
            .resource_mut::<TileMapIndex<FixtureId>>()
            .move_id(None, TileId::new(2, 2), fixture_id);

        let fixture_item_entity = app
            .world_mut()
            .spawn((
                crate::WorldTag,
                Item {
                    id: fixture_item_id,
                    kind: crate::model::components::ItemKind::Berry,
                    qty: 1,
                },
                InFixture(fixture_id),
            ))
            .id();
        app.world_mut()
            .resource_mut::<IdIndex<ItemId>>()
            .insert(fixture_item_id, fixture_item_entity);

        // Spawn a ground item at (3,3)
        let ground_item_id = {
            let mut idx = app.world_mut().resource_mut::<IdIndex<ItemId>>();
            idx.alloc(None)
        };
        let ground_item_entity = app
            .world_mut()
            .spawn((
                crate::WorldTag,
                Item {
                    id: ground_item_id,
                    kind: crate::model::components::ItemKind::Berry,
                    qty: 1,
                },
                OnGround(TileId::new(3, 3)),
            ))
            .id();
        app.world_mut()
            .resource_mut::<IdIndex<ItemId>>()
            .insert(ground_item_id, ground_item_entity);
        app.world_mut()
            .resource_mut::<TileMapIndex<ItemId>>()
            .move_id(None, TileId::new(3, 3), ground_item_id);

        let errs = validate_world(app.world_mut());
        assert!(errs.is_empty(), "unexpected errors: {:?}", errs);
    }

    #[test]
    fn detects_carried_item_not_in_inventory() {
        let mut app = App::new();
        setup_indices(&mut app, 3, 3);

        // Item is CarriedBy but not present in pawn inventory
        let pid = {
            app.world_mut()
                .resource_mut::<IdIndex<PawnId>>()
                .alloc(None)
        };
        let iid = {
            app.world_mut()
                .resource_mut::<IdIndex<ItemId>>()
                .alloc(None)
        };
        let pawn_e = app
            .world_mut()
            .spawn((
                crate::WorldTag,
                Pawn {
                    id: pid,
                    inventory: Inventory(vec![]),
                    sleep: Q40p24::ONE,
                    hunger: Q40p24::ONE,
                },
                TileId::new(0, 0),
            ))
            .id();
        app.world_mut()
            .resource_mut::<IdIndex<PawnId>>()
            .insert(pid, pawn_e);
        app.world_mut()
            .resource_mut::<TileMapIndex<PawnId>>()
            .move_id(None, TileId::new(0, 0), pid);

        let item_e = app
            .world_mut()
            .spawn((
                crate::WorldTag,
                Item {
                    id: iid,
                    kind: crate::model::components::ItemKind::Berry,
                    qty: 1,
                },
                CarriedBy(pid),
            ))
            .id();
        app.world_mut()
            .resource_mut::<IdIndex<ItemId>>()
            .insert(iid, item_e);

        let errs = validate_world(app.world_mut());
        assert!(
            errs.iter().any(|e| e.contains("carried")
                && e.contains("not in pawn inventory")),
            "unexpected errors: {:?}",
            errs
        );
    }

    #[test]
    fn detects_tilemap_relation_mismatch() {
        let mut app = App::new();
        setup_indices(&mut app, 3, 3);

        // Create a pawn (not strictly necessary for this test but consistent)
        let pid = {
            app.world_mut()
                .resource_mut::<IdIndex<PawnId>>()
                .alloc(None)
        };
        let pawn_e = app
            .world_mut()
            .spawn((
                crate::WorldTag,
                Pawn {
                    id: pid,
                    inventory: Inventory::default(),
                    sleep: Q40p24::ONE,
                    hunger: Q40p24::ONE,
                },
                TileId::new(0, 0),
            ))
            .id();
        app.world_mut()
            .resource_mut::<IdIndex<PawnId>>()
            .insert(pid, pawn_e);

        // Spawn an item with CarriedBy, but put it into the tile index at (1,1)
        let iid = {
            app.world_mut()
                .resource_mut::<IdIndex<ItemId>>()
                .alloc(None)
        };
        let item_e = app
            .world_mut()
            .spawn((
                crate::WorldTag,
                Item {
                    id: iid,
                    kind: crate::model::components::ItemKind::Berry,
                    qty: 1,
                },
                CarriedBy(pid),
            ))
            .id();
        app.world_mut()
            .resource_mut::<IdIndex<ItemId>>()
            .insert(iid, item_e);

        // Incorrectly place it in the tile index
        app.world_mut()
            .resource_mut::<TileMapIndex<ItemId>>()
            .move_id(None, TileId::new(1, 1), iid);

        let errs = validate_world(app.world_mut());
        assert!(
            errs.iter()
                .any(|e| e.contains("in tile index")
                    && e.contains("relation is")),
            "unexpected errors: {:?}",
            errs
        );
    }

    #[test]
    fn detects_onground_missing_tile_index() {
        let mut app = App::new();
        setup_indices(&mut app, 3, 3);

        let iid = {
            app.world_mut()
                .resource_mut::<IdIndex<ItemId>>()
                .alloc(None)
        };
        let pos = TileId::new(1, 2);
        let item_e = app
            .world_mut()
            .spawn((
                crate::WorldTag,
                Item {
                    id: iid,
                    kind: crate::model::components::ItemKind::Berry,
                    qty: 1,
                },
                OnGround(pos),
            ))
            .id();
        app.world_mut()
            .resource_mut::<IdIndex<ItemId>>()
            .insert(iid, item_e);

        // Intentionally do NOT update TileMapIndex<ItemId> for this OnGround
        // item

        let errs = validate_world(app.world_mut());
        assert!(
            errs.iter()
                .any(|e| e.contains("OnGround") && e.contains("tile index")),
            "unexpected errors: {:?}",
            errs
        );
    }

    #[test]
    fn detects_inventory_missing_entity_and_dual_inventory() {
        let mut app = App::new();
        setup_indices(&mut app, 3, 3);

        // Create a pawn and a fixture
        let pid = app
            .world_mut()
            .resource_mut::<IdIndex<PawnId>>()
            .alloc(None);
        let pawn_e = app
            .world_mut()
            .spawn((
                crate::WorldTag,
                Pawn {
                    id: pid,
                    inventory: Inventory::default(),
                    sleep: Q40p24::ONE,
                    hunger: Q40p24::ONE,
                },
                TileId::new(0, 0),
            ))
            .id();
        app.world_mut()
            .resource_mut::<IdIndex<PawnId>>()
            .insert(pid, pawn_e);

        let fid = app
            .world_mut()
            .resource_mut::<IdIndex<FixtureId>>()
            .alloc(None);
        let fixture_e = app
            .world_mut()
            .spawn((
                crate::WorldTag,
                Fixture {
                    id: fid,
                    kind: FixtureKind::Stockpile,
                    inventory: Inventory::default(),
                    harvest_countdown: None,
                },
                TileId::new(1, 1),
            ))
            .id();
        app.world_mut()
            .resource_mut::<IdIndex<FixtureId>>()
            .insert(fid, fixture_e);

        // Case 1: Inventory references missing entity (IdIndex entry without
        // Entity)
        let missing_iid = app
            .world_mut()
            .resource_mut::<IdIndex<ItemId>>()
            .alloc(None);
        // Put into pawn inventory without spawning the Item
        {
            let mut pawn = app.world_mut().get_mut::<Pawn>(pawn_e).unwrap();
            pawn.inventory.0.push((missing_iid, ItemKind::Berry));
        }

        // Case 2: Same item in both inventories (create a real item id)
        let dual_iid = app
            .world_mut()
            .resource_mut::<IdIndex<ItemId>>()
            .alloc(None);
        {
            let mut pawn = app.world_mut().get_mut::<Pawn>(pawn_e).unwrap();
            pawn.inventory.0.push((dual_iid, ItemKind::Berry));
        }
        {
            let mut fix =
                app.world_mut().get_mut::<Fixture>(fixture_e).unwrap();
            fix.inventory.0.push((dual_iid, ItemKind::Berry));
        }
        // Spawn entity for dual_iid but give it conflicting relation (OnGround)
        let dual_e = app
            .world_mut()
            .spawn((
                crate::WorldTag,
                Item {
                    id: dual_iid,
                    kind: ItemKind::Berry,
                    qty: 1,
                },
                OnGround(TileId::new(2, 2)),
            ))
            .id();
        app.world_mut()
            .resource_mut::<IdIndex<ItemId>>()
            .insert(dual_iid, dual_e);

        let errs = validate_world(app.world_mut());
        // Expect errors about missing IdIndex entity for missing_iid, dual
        // inventory presence, and relation mismatch for dual_iid
        assert!(errs.iter().any(|e| e.contains("missing from IdIndex")));
        assert!(errs.iter().any(
            |e| e.contains("appears in both pawn") && e.contains("fixture")
        ));
        assert!(errs
            .iter()
            .any(|e| e.contains("inventory") && e.contains("relation is")));
    }
}
