use std::path::PathBuf;

use bevy::prelude::*;
use simkit_core::{grid::TileId, ids::IdIndex};

use stitchlands::{
    invariants::validate_world,
    model::{
        components::{Fixture, Item, ItemRelation, Pawn},
        ids::{FixtureId, ItemId, PawnId, TaskId},
    },
    scenario::{model::ScenarioDef},
    scenario::testutil::{app_with_scenario, load_toml},
};

#[test]
fn scenario_toml_loads_and_passes_invariants() {
    // Load TOML scenario file
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/small.toml");
    let scenario: ScenarioDef = load_toml(&path);

    // Build app, load scenario, then run invariants validation
    let mut app = app_with_scenario(scenario);

    // Explicit checks for small.toml
    {
        let world = app.world_mut();

        // Pawns: id 1 at (1,1), id 2 at (0,5)
        let (pawns_info, p1_pos, p2_pos) = {
            let mut qp = world.query::<(&Pawn, &TileId)>();
            let mut info: Vec<(PawnId, TileId)> = Vec::new();
            let mut p1p = None;
            let mut p2p = None;
            for (p, pos) in qp.iter(world) {
                info.push((p.id, *pos));
                if p.id.0 == 1 {
                    p1p = Some(*pos);
                } else if p.id.0 == 2 {
                    p2p = Some(*pos);
                }
            }
            (info, p1p.expect("pawn id=1"), p2p.expect("pawn id=2"))
        };
        assert_eq!(pawns_info.len(), 3, "expected three pawns (Sam, Billy, Ava)");
        assert_eq!(p1_pos, TileId::new(1, 1));
        assert_eq!(p2_pos, TileId::new(0, 5));

        // Fixture: id 1 at (2,2)
        let (fixtures_info, f1_pos) = {
            let mut qf = world.query::<(&Fixture, &TileId)>();
            let mut info: Vec<(FixtureId, TileId)> = Vec::new();
            let mut f1p = None;
            for (f, pos) in qf.iter(world) {
                info.push((f.id, *pos));
                if f.id.0 == 1 {
                    f1p = Some(*pos);
                }
            }
            (info, f1p.expect("fixture id=1 present"))
        };
        assert_eq!(f1_pos, TileId::new(2, 2));

        // Ground item from map.tiles at (3,2)
        let ground = {
            let mut qi = world.query::<(&Item, &ItemRelation)>();
            let mut g: Vec<(ItemId, TileId)> = Vec::new();
            for (it, rel) in qi.iter(world) {
                if let ItemRelation::OnGround(p) = rel {
                    g.push((it.id, *p));
                }
            }
            g
        };
        assert_eq!(ground.len(), 1, "expected exactly one ground item");
        let (ground_id, ground_pos) = ground[0];
        assert_eq!(ground_pos, TileId::new(3, 2));
        // Ensure no pawn/fixture inventory contains the ground item
        {
            let mut qp = world.query::<&Pawn>();
            for p in qp.iter(world) {
                assert!(!p.inventory.contains(&ground_id));
            }
        }
        {
            let mut qf = world.query::<&Fixture>();
            for f in qf.iter(world) {
                assert!(!f.inventory.contains(&ground_id));
            }
        }

        // Carried items: Item 1 by Pawn 1, Item 2 by Pawn 2
        let (ent1, ent2, ent2002) = {
            let idx = world.resource::<IdIndex<ItemId>>();
            (idx.get(&ItemId(1)), idx.get(&ItemId(2)), idx.get(&ItemId(2002)))
        };
        let mut qrel = world.query::<&ItemRelation>();
        let rel1 = qrel.get(world, ent1).expect("item#1 relation");
        assert_eq!(*rel1, ItemRelation::CarriedBy(PawnId(1)));
        let rel2 = qrel.get(world, ent2).expect("item#2 relation");
        assert_eq!(*rel2, ItemRelation::CarriedBy(PawnId(2)));
        // Fixture inventory: Item 2002 in fixture 1
        let relf = qrel.get(world, ent2002).expect("item#2002 relation");
        assert_eq!(*relf, ItemRelation::InFixture(FixtureId(1)));
    }

    // Validate invariants
    let errs = validate_world(app.world_mut());
    assert!(errs.is_empty(), "invariants failed: {:?}", errs);
}
