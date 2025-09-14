use bevy::reflect::ConstParamInfo;

use super::*;

impl JobKind {
    pub fn build_plan_from_world(
        &self,
        world: &World,
        pawn_id: &PawnId,
        items: &ItemQuery<&ItemRelation>,
        fixtures: &FixtureQuery,
    ) -> Result<VecDeque<ToilKind>, String> {
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
        item_reservations: &ItemReservations,
        items: &ItemQuery<&ItemRelation>,
        fixtures: &FixtureQuery,
        fixture_tile_index: &TileMapIndex<FixtureId>,
    ) -> Result<VecDeque<ToilKind>, String> {
        match self {
            JobKind::Sleep => build_sleep_plan(pawn_tile, fixtures),
            JobKind::Eat => build_eat_plan(pawn, pawn_tile, items, fixtures),
            JobKind::Task(task_id, _) => build_plan_for_task(
                tasks.tasks.get(task_id).unwrap(),
                pawn,
                pawn_tile,
                items,
                fixtures,
                fixture_tile_index,
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
) -> Result<VecDeque<ToilKind>, String> {
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

            let dist = manhattan(*pawn_tile, *fixture_tile);
            // If already adjacent, no need to move
            if dist <= 1 {
                return Ok(VecDeque::from_iter([ToilKind::Harvest {
                    fixture_id: *to_harvest,
                }]));
            }

            let mut plan = VecDeque::with_capacity(2);
            if let Some(move_to) = move_to_adj(*pawn_tile, *fixture_tile) {
                plan.push_back(move_to);
            }
            plan.push_back(ToilKind::Harvest {
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

            let (mut plan, seed, seed_pos) = build_acquire_item_plan(
                pawn, pawn_tile, item_kind, items, fixtures,
            )
            .ok_or_else(|| format!("Failed to acquire item {:?}", item_kind))?;

            if let Some(move_to) = move_to_adj(seed_pos, *target_tile_id) {
                plan.push_back(move_to);
            }
            plan.push_back(ToilKind::Plant {
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
                        let mut plan = VecDeque::with_capacity(2);
                        if let Some(move_to) =
                            move_to_adj(*pawn_tile, building_spec.top_left)
                        {
                            plan.push_back(move_to);
                        }
                        plan.push_back(ToilKind::PlaceConstructionSite {
                            building_spec: building_spec.clone(),
                        });
                        return Ok(plan);
                    }
                };

            let construction_site = fixtures.get(&construction_site_id).0;

            for (item_kind, qty) in &building_spec.required_items {
                if construction_site.inventory.of_kind(*item_kind).count()
                    < *qty as usize
                {
                    let (mut plan, item_id, item_pos) =
                        build_acquire_item_plan(
                            pawn, pawn_tile, item_kind, items, fixtures,
                        )
                        .ok_or_else(|| {
                            format!("Failed to acquire item {:?}", item_kind)
                        })?;

                    if let Some(move_to) =
                        move_to_adj(item_pos, building_spec.top_left)
                    {
                        plan.push_back(move_to);
                    }
                    plan.push_back(ToilKind::StoreItem {
                        item_id,
                        target_fixture_id: construction_site_id,
                    });

                    return Ok(plan);
                }
            }

            // Build
            let mut plan = VecDeque::with_capacity(2);
            if let Some(move_to) =
                move_to_adj(*pawn_tile, building_spec.top_left)
            {
                plan.push_back(move_to);
            }
            plan.push_back(ToilKind::Build {
                fixture_id: construction_site_id,
            });

            Ok(plan)
        }
    }
}

pub fn build_sleep_plan(
    pawn_pos: &TileId,
    fixtures: &FixtureQuery,
) -> Result<VecDeque<ToilKind>, String> {
    let sleeping_pad = fixtures
        .query
        .iter()
        .filter(|(fixture, _)| fixture.kind == FixtureKind::SleepingPad)
        .min_by_key(|(_, (pos, _, _))| manhattan(*pawn_pos, **pos));

    let Some((sleeping_pad, (sleeping_pad_pos, _, _))) = sleeping_pad else {
        return Err(format!("No sleeping pad found for pawn {:?}", pawn_pos));
    };

    Ok(VecDeque::from_iter([
        ToilKind::MoveTo {
            target: *sleeping_pad_pos,
            path: manhattan_path(*pawn_pos, *sleeping_pad_pos),
        },
        ToilKind::Sleep {
            fixture_id: sleeping_pad.id,
        },
    ]))
}

pub fn build_eat_plan(
    pawn: &Pawn,
    pawn_tile: &TileId,
    items: &ItemQuery<&ItemRelation>,
    fixtures: &FixtureQuery,
    item_reservations: &Reserver,
) -> Result<VecDeque<ToilKind>, String> {
    let Some((mut plan, item_id, _)) = build_acquire_item_plan(
        pawn,
        pawn_tile,
        &ItemKind::Berry,
        items,
        fixtures,
        item_reservations,
    ) else {
        return Err(format!("Failed to acquire item {:?}", ItemKind::Berry));
    };
    plan.push_back(ToilKind::Consume { item_id });
    Ok(plan)
}

fn build_acquire_item_plan(
    pawn: &Pawn,
    pawn_pos: &TileId,
    item_kind: &ItemKind,
    items: &ItemQuery<&ItemRelation>,
    fixtures: &FixtureQuery,
    item_reservations: &Reserver,
) -> Option<(VecDeque<ToilKind>, ItemId, TileId)> {
    if let Some(item_id) = pawn.inventory.find(*item_kind) {
        item_reservations.reserve(item_id);
        return Some((VecDeque::new(), item_id, *pawn_pos));
    }

    let on_ground = nearest_item_on_ground(item_kind, pawn_pos, items);
    let fixture = nearest_fixture_with_item(item_kind, pawn_pos, fixtures);
    let (item_id, _dist) = closer_option_item_locator(on_ground, fixture)?;

    let item_pos = match items.get(&item_id).1 {
        ItemRelation::CarriedBy(_) => *pawn_pos,
        ItemRelation::InFixture(fixture_id) => *fixtures.get(fixture_id).1.0,
        ItemRelation::OnGround(tile_id) => *tile_id,
    };

    let mut plan = VecDeque::with_capacity(2);
    if let Some(move_to) = move_to_adj(*pawn_pos, item_pos) {
        plan.push_back(move_to);
    }
    plan.push_back(ToilKind::PickUpItem { item_id });
    item_reservations.reserve(item_id);

    Some((plan, item_id, item_pos))
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
