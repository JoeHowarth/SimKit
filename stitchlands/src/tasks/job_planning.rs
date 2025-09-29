use bevy::reflect::ConstParamInfo;

use super::*;
use crate::tasks::reservations::Reservations;

impl JobKind {
    pub fn build_plan_from_world(
        &self,
        world: &World,
        pawn_id: &PawnId,
        items: &ItemQuery<&ItemRelation>,
        fixtures: &FixtureQuery,
    ) -> Result<Plan, String> {
        self.build_plan(
            world.comp(pawn_id),
            world.comp(pawn_id),
            world.resource(),
            world.resource(),
            items,
            fixtures,
            world.resource(),
        )
    }

    pub fn build_plan(
        &self,
        pawn: &Pawn,
        pawn_tile: &TileId,
        tasks: &TaskBoard,
        reservations: &Reservations,
        items: &ItemQuery<&ItemRelation>,
        fixtures: &FixtureQuery,
        fixture_tile_index: &TileMapIndex<FixtureId>,
    ) -> Result<Plan, String> {
        match self {
            JobKind::Sleep => {
                build_sleep_plan(pawn_tile, fixtures, reservations)
            }
            JobKind::Eat => {
                build_eat_plan(pawn, pawn_tile, items, fixtures, reservations)
            }
            JobKind::Task(task_id, _) => build_plan_for_task(
                tasks.tasks.get(task_id).unwrap(),
                pawn,
                pawn_tile,
                items,
                fixtures,
                fixture_tile_index,
                reservations,
            ),
            JobKind::None => Err("No plan for none job".to_string()),
        }
    }
}

pub fn build_plan_for_task(
    task: &Task,
    pawn: &Pawn,
    pawn_tile: &TileId,
    items: &ItemQuery<&ItemRelation>,
    fixtures: &FixtureQuery,
    fixture_tile_index: &TileMapIndex<FixtureId>,
    reservations: &Reservations,
) -> Result<Plan, String> {
    let mut plan = Plan::new(reservations);
    match &task.spec {
        TaskSpec::Harvest { to_harvest, .. } => {
            let (_, (fixture_tile, harvestable, _)) = fixtures.get(to_harvest);

            // Check if fixture is ready to harvest
            if harvestable.is_none() || harvestable.unwrap().countdown > 0 {
                return Err(format!(
                    "Fixture {:?} is not ready to harvest",
                    to_harvest
                ));
            }

            plan.reservations.try_reserve(*to_harvest)?;
            plan.move_to_adj(*pawn_tile, *fixture_tile)?;
            plan.toils.push_back(ToilKind::Harvest {
                fixture_id: *to_harvest,
            });
            Ok(plan)
        }
        TaskSpec::Plant(target_tile_id, item_kind) => {
            // Check if tile is a plantable tile
            if let Some(fixture_id) = fixture_tile_index.get(*target_tile_id) {
                return Err(format!(
                    "Tile {:?} is not a plantable tile. Contains fixture {:?}",
                    target_tile_id, fixture_id
                ));
            }

            let (seed, seed_pos) = build_acquire_item_plan(
                pawn, pawn_tile, item_kind, items, fixtures, &mut plan,
            )?;

            plan.move_to_adj(seed_pos, *target_tile_id)?;
            plan.toils.push_back(ToilKind::Plant {
                seed_id: seed,
                tile_id: *target_tile_id,
            });
            Ok(plan)
        }
        TaskSpec::Build(building_spec) => {
            assert!(!building_spec.required_items.is_empty());
            assert!(
                building_spec.required_items.iter().all(|(_, x)| *x > 0),
                "All requied items in list must have non-zero qtys"
            );

            let construction_site_id =
                match fixture_tile_index.get(building_spec.top_left) {
                    Some(fixture_id) => {
                        let fixture = fixtures.get(&fixture_id).0;

                        if fixture.kind != FixtureKind::ConstructionSite {
                            return Err(format!(
                                "Tile {:?} is not a valid build site. \
                                 Contains fixture {:?}",
                                building_spec.top_left, fixture_id
                            ));
                        }
                        fixture_id
                    }
                    None => {
                        plan.move_to_adj(*pawn_tile, building_spec.top_left)?;
                        plan.toils.push_back(ToilKind::PlaceConstructionSite {
                            building_spec: building_spec.clone(),
                        });
                        return Ok(plan);
                    }
                };

            let construction_site = fixtures.get(&construction_site_id).0;
            plan.reservations.try_reserve(construction_site_id)?;

            for (item_kind, qty) in &building_spec.required_items {
                // Reserve all items in the construction site inventory
                if let Err(e) = construction_site
                    .inventory
                    .of_kind(*item_kind)
                    .try_for_each(|item_id| {
                        plan.reservations.try_reserve(item_id)
                    })
                {
                    panic!("Failed to reserve items: {}", e);
                }
            }

            for (item_kind, qty) in &building_spec.required_items {
                // If the construction site inventory has less than the required
                // quantity, acquire the item
                if construction_site.inventory.of_kind(*item_kind).count()
                    < *qty as usize
                {
                    let (item_id, item_pos) = build_acquire_item_plan(
                        pawn, pawn_tile, item_kind, items, fixtures, &mut plan,
                    )?;

                    plan.move_to_adj(item_pos, building_spec.top_left)?;
                    plan.toils.push_back(ToilKind::StoreItem {
                        item_id,
                        target_fixture_id: construction_site_id,
                    });

                    return Ok(plan);
                }
            }

            // Build
            plan.move_to_adj(*pawn_tile, building_spec.top_left)?;
            plan.toils.push_back(ToilKind::Build {
                fixture_id: construction_site_id,
            });

            Ok(plan)
        }
    }
}

pub fn build_sleep_plan(
    pawn_pos: &TileId,
    fixtures: &FixtureQuery,
    reservations: &Reservations,
) -> Result<Plan, String> {
    let sleeping_pad = fixtures
        .query
        .iter()
        .filter(|(fixture, _)| fixture.kind == FixtureKind::SleepingPad)
        .filter(|(fixture, (_, _, _))| !reservations.is_reserved(fixture.id))
        .min_by_key(|(_, (pos, _, _))| manhattan(*pawn_pos, **pos));

    let Some((sleeping_pad, (sleeping_pad_pos, _, _))) = sleeping_pad else {
        return Err(format!("No sleeping pad found for pawn {:?}", pawn_pos));
    };

    let mut plan = Plan::new(reservations);
    plan.move_to_adj(*pawn_pos, *sleeping_pad_pos)?;
    plan.reservations.try_reserve(sleeping_pad.id)?;
    plan.toils.push_back(ToilKind::Sleep {
        fixture_id: sleeping_pad.id,
    });
    Ok(plan)
}

pub fn build_eat_plan(
    pawn: &Pawn,
    pawn_tile: &TileId,
    items: &ItemQuery<&ItemRelation>,
    fixtures: &FixtureQuery,
    reservations: &Reservations,
) -> Result<Plan, String> {
    let mut plan = Plan::new(reservations);
    let (item_id, _item_pos) = build_acquire_item_plan(
        pawn,
        pawn_tile,
        &ItemKind::Berry,
        items,
        fixtures,
        &mut plan,
    )?;

    plan.toils.push_back(ToilKind::Consume { item_id });
    Ok(plan)
}

fn build_acquire_item_plan(
    pawn: &Pawn,
    pawn_pos: &TileId,
    item_kind: &ItemKind,
    items: &ItemQuery<&ItemRelation>,
    fixtures: &FixtureQuery,
    plan: &mut Plan,
) -> Result<(ItemId, TileId), String> {
    if let Some(item_id) = pawn.inventory.find(*item_kind) {
        plan.reservations.try_reserve(item_id)?;
        return Ok((item_id, *pawn_pos));
    }

    let res = &plan.reservations.handle;
    let on_ground = nearest_item_on_ground(item_kind, pawn_pos, items, res);
    let fixture = nearest_fixture_with_item(item_kind, pawn_pos, fixtures, res);
    let (item_id, _dist) = closer_option_item_locator(on_ground, fixture)
        .ok_or_else(|| format!("No item found for {:?}", item_kind))?;

    let item_pos = match items.get(&item_id).1 {
        ItemRelation::CarriedBy(_) => *pawn_pos,
        ItemRelation::InFixture(fixture_id) => *fixtures.get(fixture_id).1.0,
        ItemRelation::OnGround(tile_id) => *tile_id,
    };

    plan.move_to_adj(*pawn_pos, item_pos)?;
    plan.reservations.try_reserve(item_id)?;
    plan.toils.push_back(ToilKind::PickUpItem { item_id });
    Ok((item_id, item_pos))
}

impl Plan {
    pub fn move_to_adj(
        &mut self,
        start: TileId,
        end: TileId,
    ) -> Result<(), String> {
        let path = manhattan_path_adj(start, end);
        let Some(target) = path.back().copied() else {
            return Ok(());
        };
        self.reservations.try_reserve(target)?;
        self.toils.push_back(ToilKind::MoveTo { target, path });
        Ok(())
    }
}

pub fn move_to_adj(start: TileId, end: TileId) -> Option<ToilKind> {
    let path = manhattan_path_adj(start, end);
    let Some(target) = path.back().copied() else {
        return None;
    };
    Some(ToilKind::MoveTo { target, path })
}

pub fn manhattan_path_adj(start: TileId, end: TileId) -> VecDeque<TileId> {
    if start == end {
        // TODO: should handle map boundaries
        return VecDeque::from_iter([TileId::new(start.x + 1, start.y)]);
    }

    let mut path = manhattan_path(start, end);
    path.pop_back().unwrap();
    path
}

pub fn manhattan_path(start: TileId, end: TileId) -> VecDeque<TileId> {
    let mut path = VecDeque::new();

    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let mut x = start.x;
    let mut y = start.y;
    while x != end.x {
        x += dx.signum();
        path.push_back(TileId::new(x, y))
    }
    while y != end.y {
        y += dy.signum();
        path.push_back(TileId::new(x, y));
    }
    path
}
