use super::*;

impl JobKind {
    pub fn build_plan_from_world(
        &self,
        world: &World,
        pawn_id: &PawnId,
        items: &ItemQuery<&ItemRelation>,
        fixtures: &FixtureQuery,
    ) -> Result<VecDeque<ToilKind>, String> {
        let pawn = world.comp(pawn_id);
        let pawn_tile = world.comp(pawn_id);
        let tasks = world.resource::<TaskBoard>();
        let fixture_tile_index = world.resource::<TileMapIndex<FixtureId>>();

        self.build_plan(
            pawn,
            pawn_tile,
            tasks,
            items,
            fixtures,
            fixture_tile_index,
        )
    }

    pub fn build_plan(
        &self,
        pawn: &Pawn,
        pawn_tile: &TileId,
        tasks: &TaskBoard,
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

            Ok(VecDeque::from_iter([
                move_to_adj(*pawn_tile, *fixture_tile),
                ToilKind::Harvest {
                    fixture_id: *to_harvest,
                },
            ]))
        }
        TaskSpec::Plant(target_tile_id, item_kind) => {
            // Check if tile is a plantable tile
            if let Some(fixture_id) = fixture_tile_index.get(*target_tile_id) {
                return Err(format!(
                    "Tile {:?} is not a plantable tile. Contains fixture {:?}",
                    target_tile_id, fixture_id
                ));
            }

            let (mut plan, seed) = build_acquire_item_plan(
                pawn, pawn_tile, item_kind, items, fixtures,
            )
            .ok_or_else(|| format!("Failed to acquire item {:?}", item_kind))?;

            let seed_pos = match items.get(&seed).1 {
                ItemRelation::CarriedBy(_) => *pawn_tile,
                ItemRelation::InFixture(fixture_id) => {
                    *fixtures.get(fixture_id).1.0
                }
                ItemRelation::OnGround(tile_id) => *tile_id,
            };

            plan.push_back(move_to_adj(seed_pos, *target_tile_id));
            plan.push_back(ToilKind::Plant {
                seed_id: seed,
                tile_id: *target_tile_id,
            });
            Ok(plan)
        }
        TaskSpec::Build(building_spec) => {
            // Check if a fixture already exists there
            //    no  => move_to, place construction site
            //    yes => if it's a construction site continue, else site is not
            // valid to build on
            assert!(!building_spec.required_items.is_empty());
            assert!(
                building_spec.required_items.iter().all(|(_, x)| *x > 0),
                "All requied items in list must have non-zero qtys"
            );

            let mut plan = VecDeque::new();
            match fixture_tile_index.get(building_spec.top_left) {
                Some(fixture_id) => {
                    let fixture = fixtures.get(&fixture_id).0;

                    if fixture.kind != FixtureKind::ConstructionSite {
                        return Err(format!(
                            "Tile {:?} is not a valid build site. Contains \
                             fixture {:?}",
                            building_spec.top_left, fixture_id
                        ));
                    }
                }
                None => {
                    plan.push_back(move_to_adj(
                        *pawn_tile,
                        building_spec.top_left,
                    ));
                    plan.push_back(ToilKind::PlaceConstructionSite {
                        building_spec: building_spec.clone(),
                    })
                }
            }

            // Fetch all required items
            for (item_kind, qty) in &building_spec.required_items {
                for _ in 0..*qty {
                    let (plan_, item_id) = build_acquire_item_plan(
                        pawn, pawn_tile, item_kind, items, fixtures,
                    )
                    .ok_or_else(|| {
                        format!("Failed to acquire item {:?}", item_kind)
                    })?;

                    plan.extend(plan_);
                    plan.push_back(move_to_adj(
                        *pawn_tile,
                        building_spec.top_left,
                    ));
                    plan.push_back(ToilKind::StoreItem {
                        item_id,
                        target_fixture_id: None,
                        fixture_pos: building_spec.top_left,
                    });
                }
            }

            // Build

            todo!()
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
) -> Result<VecDeque<ToilKind>, String> {
    let Some((mut plan, item_id)) = build_acquire_item_plan(
        pawn,
        pawn_tile,
        &ItemKind::Berry,
        items,
        fixtures,
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
) -> Option<(VecDeque<ToilKind>, ItemId)> {
    if let Some(item_id) = pawn.inventory.find(*item_kind) {
        return Some((VecDeque::new(), item_id));
    }

    let on_ground = nearest_item_on_ground(item_kind, pawn_pos, items);
    let fixture = nearest_fixture_with_item(item_kind, pawn_pos, fixtures);
    let (item_id, _dist) = closer_option_item_locator(on_ground, fixture)?;

    let item_pos = match items.get(&item_id).1 {
        ItemRelation::CarriedBy(_) => *pawn_pos,
        ItemRelation::InFixture(fixture_id) => *fixtures.get(fixture_id).1.0,
        ItemRelation::OnGround(tile_id) => *tile_id,
    };

    Some((
        VecDeque::from_iter([
            ToilKind::MoveTo {
                target: item_pos,
                path: manhattan_path(*pawn_pos, item_pos),
            },
            ToilKind::PickUpItem { item_id },
        ]),
        item_id,
    ))
}

pub fn move_to_adj(start: TileId, end: TileId) -> ToilKind {
    let path = manhattan_path_adj(start, end);
    ToilKind::MoveTo {
        target: *path.back().unwrap(),
        path,
    }
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
