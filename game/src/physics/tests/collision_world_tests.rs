use super::*;
use engine_core::game::Game;
use engine_core::tiles::{TileMap, TileRegistry, apply_tile_definition_to_entity, tile_definition_component_snapshot};

fn world_with_solid(aabb: (Vec2, Vec2)) -> CollisionWorld {
    let shape = ColliderShape::Aabb {
        width: aabb.1.x - aabb.0.x,
        height: aabb.1.y - aabb.0.y,
    };
    CollisionWorld {
        solids: vec![SolidObj {
            aabb,
            shape,
            shape_pos: aabb.0,
            entity: None,
            layer: None,
            interior_zone: None,
        }],
        entity_layers: Default::default(),
        back_interior_zones: Vec::new(),
    }
}

fn dummy_entity() -> Entity {
    Entity::null()
}

fn empty_room() -> Room {
    Room {
        id: RoomId(1),
        position: Vec2::ZERO,
        variants: vec![RoomVariant {
            tilemap: TileMap::new(8, 8),
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn empty_world() -> World {
    world_with_room(empty_room())
}

fn world_with_room(room: Room) -> World {
    let mut world = World::default();
    world.grid_size = 16.0;
    world.current_room_id = Some(room.id);
    world.add_room(room);
    world
}

fn room_with_back_zones(interior_zones: Vec<InteriorZone>) -> Room {
    Room {
        id: RoomId(1),
        position: Vec2::ZERO,
        size: Vec2::new(4.0, 4.0),
        variants: vec![RoomVariant {
            tilemap: TileMap::new(4, 4),
            layers: RoomLayers {
                back: Some(BackRoomLayer {
                    interior_zones,
                    ..Default::default()
                }),
            },
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[test]
fn collision_world_sweep_move_aabb_blocked_by_tile_entity_solid_component() {
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let mover = ecs
        .create_entity()
        .with_current_room(room_id)
        .with(Transform {
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .finish();

    let mut tile_registry = TileRegistry::default();
    let tile_id = tile_registry.insert(engine_core::tiles::TileDef {
        sprite_id: SpriteId(1),
        components: vec![],
    });

    ecs.create_entity()
        .with(TilePlacement::new(tile_id, 1, 0))
        .with(Solid(true))
        .with_current_room(room_id)
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::ZERO,
        Vec2::new(16.0, 0.0),
        Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(sweep.blocked_x);
}

#[test]
fn collision_world_back_layer_mover_is_blocked_by_back_layer_tile() {
    let room_id = RoomId(1);
    let mut game = Game::default();
    let mover = game
        .ecs
        .create_entity()
        .with_current_room_layer(room_id, RoomLayer::Back)
        .with(Transform {
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .finish();

    let tile_id = game.tile_registry.insert(engine_core::tiles::TileDef {
        sprite_id: SpriteId(1),
        components: vec![],
    });

    game.ecs
        .create_entity()
        .with(TilePlacement::new(tile_id, 1, 0))
        .with(Solid(true))
        .with_current_room_layer(room_id, RoomLayer::Back)
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let sweep = CollisionWorld::new(&game.ecs, room, &world).sweep_move(
        mover,
        Vec2::ZERO,
        Vec2::new(16.0, 0.0),
        Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(sweep.blocked_x);
}

#[test]
fn collision_world_front_layer_mover_does_not_collide_with_back_layer_tile() {
    let room_id = RoomId(1);
    let mut game = Game::default();
    let mover = game
        .ecs
        .create_entity()
        .with_current_room(room_id)
        .with(Transform {
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .finish();

    let tile_id = game.tile_registry.insert(engine_core::tiles::TileDef {
        sprite_id: SpriteId(1),
        components: vec![],
    });

    game.ecs
        .create_entity()
        .with(TilePlacement::new(tile_id, 1, 0))
        .with(Solid(true))
        .with_current_room_layer(room_id, RoomLayer::Back)
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let sweep = CollisionWorld::new(&game.ecs, room, &world).sweep_move(
        mover,
        Vec2::ZERO,
        Vec2::new(16.0, 0.0),
        Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(!sweep.blocked_x);
}

#[test]
fn collision_world_front_layer_entity_is_not_constrained_by_back_layer_interior_bounds() {
    let room = room_with_back_zones(vec![InteriorZone {
        id: InteriorZoneId(1),
        bounds: Rect::new(0.0, 0.0, 32.0, 64.0),
    }]);
    let world = world_with_room(room.clone());
    let mut ecs = Ecs::default();
    let mover = ecs
        .create_entity()
        .with_current_room(room.id)
        .with(Transform {
            position: Vec2::new(24.0, 0.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .finish();

    let sweep = CollisionWorld::new(&ecs, &room, &world).sweep_move(
        mover,
        Vec2::new(24.0, 0.0),
        Vec2::new(8.0, 0.0),
        Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert_eq!(sweep.allowed_delta.x, 8.0);
    assert!(!sweep.blocked_x);
}

#[test]
fn collision_world_front_layer_exit_does_not_open_back_layer_border_gap() {
    let mut room = empty_room();
    room.exits.push(Exit {
        position: vec2(room.current_variant().tilemap.width as f32, 1.0),
        direction: ExitDirection::Right,
        layer: RoomLayer::Front,
        target_room_id: Some(RoomId(2)),
    });
    let world = world_with_room(room.clone());
    let mut ecs = Ecs::default();
    let mover = ecs
        .create_entity()
        .with_current_room_layer(room.id, RoomLayer::Back)
        .with(Transform {
            position: Vec2::new(112.0, 16.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .finish();

    let sweep = CollisionWorld::new(&ecs, &room, &world).sweep_move(
        mover,
        Vec2::new(112.0, 16.0),
        Vec2::new(16.0, 0.0),
        Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(sweep.blocked_x);
    assert!(sweep.allowed_delta.x < 16.0);
}

#[test]
fn collision_world_adjacent_back_zones_do_not_allow_crossing_shared_edge() {
    let room = room_with_back_zones(vec![
        InteriorZone {
            id: InteriorZoneId(1),
            bounds: Rect::new(0.0, 0.0, 32.0, 64.0),
        },
        InteriorZone {
            id: InteriorZoneId(2),
            bounds: Rect::new(32.0, 0.0, 32.0, 64.0),
        },
    ]);
    let world = world_with_room(room.clone());
    let mut ecs = Ecs::default();
    let mover = ecs
        .create_entity()
        .with_current_room_layer(room.id, RoomLayer::Back)
        .with(Transform {
            position: Vec2::new(24.0, 0.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .finish();

    let sweep = CollisionWorld::new(&ecs, &room, &world).sweep_move(
        mover,
        Vec2::new(24.0, 0.0),
        Vec2::new(8.0, 0.0),
        Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert_eq!(sweep.allowed_delta.x, 0.0);
    assert!(sweep.blocked_x);
}

#[test]
fn collision_world_sweep_move_aabb_blocked_by_definition_owned_solid_component() {
    let room_id = RoomId(1);
    let mut game = Game::default();
    let mover = game
        .ecs
        .create_entity()
        .with_current_room(room_id)
        .with(Transform {
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .finish();

    let tile_id = game.tile_registry.insert(engine_core::tiles::TileDef {
        sprite_id: SpriteId(1),
        components: vec![tile_definition_component_snapshot(Solid(true))],
    });

    let entity = game
        .ecs
        .create_entity()
        .with(TilePlacement::new(tile_id, 1, 0))
        .with_current_room(room_id)
        .finish();
    {
        let mut ctx = game.ctx_mut();
        apply_tile_definition_to_entity(&mut ctx, entity, tile_id);
    }

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&game.ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::ZERO,
        Vec2::new(16.0, 0.0),
        Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(sweep.blocked_x);
}

#[test]
fn collision_world_when_solid_tile_placement_is_removed_then_cell_no_longer_blocks() {
    let room_id = RoomId(1);
    let mut game = Game::default();
    let mover = game
        .ecs
        .create_entity()
        .with_current_room(room_id)
        .with(Transform {
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .finish();

    let tile_id = game.tile_registry.insert(engine_core::tiles::TileDef {
        sprite_id: SpriteId(1),
        components: vec![],
    });

    let entity = game
        .ecs
        .create_entity()
        .with(TilePlacement::new(tile_id, 1, 0))
        .with(Solid(true))
        .with_current_room(room_id)
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let blocked = CollisionWorld::new(&game.ecs, room, &world).sweep_move(
        mover,
        Vec2::ZERO,
        Vec2::new(16.0, 0.0),
        Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );
    assert!(blocked.blocked_x);

    {
        let mut ctx = game.ctx_mut();
        Ecs::remove_entity(&mut ctx, entity);
    }

    let unblocked = CollisionWorld::new(&game.ecs, room, &world).sweep_move(
        mover,
        Vec2::ZERO,
        Vec2::new(16.0, 0.0),
        Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );
    assert!(!unblocked.blocked_x);
}

#[test]
fn collision_world_when_definition_removes_solid_then_linked_placements_no_longer_block() {
    let room_id = RoomId(1);
    let mut game = Game::default();
    let mover = game
        .ecs
        .create_entity()
        .with_current_room(room_id)
        .with(Transform {
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .finish();

    let tile_id = game.tile_registry.insert(engine_core::tiles::TileDef {
        sprite_id: SpriteId(1),
        components: vec![tile_definition_component_snapshot(Solid(true))],
    });

    let entity = game
        .ecs
        .create_entity()
        .with(TilePlacement::new(tile_id, 1, 0))
        .with_current_room(room_id)
        .finish();
    {
        let mut ctx = game.ctx_mut();
        apply_tile_definition_to_entity(&mut ctx, entity, tile_id);
    }

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let blocked = CollisionWorld::new(&game.ecs, room, &world).sweep_move(
        mover,
        Vec2::ZERO,
        Vec2::new(16.0, 0.0),
        Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );
    assert!(blocked.blocked_x);

    game.tile_registry.replace(
        tile_id,
        engine_core::tiles::TileDef {
            sprite_id: SpriteId(1),
            components: vec![],
        },
    );
    game.sync_tile_definition(tile_id);

    let unblocked = CollisionWorld::new(&game.ecs, room, &world).sweep_move(
        mover,
        Vec2::ZERO,
        Vec2::new(16.0, 0.0),
        Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );
    assert!(!unblocked.blocked_x);
}

#[test]
fn collision_world_when_room_contains_many_tile_placements_then_solid_count_matches_expected() {
    const GRID_SIDE: usize = 8;

    let room_id = RoomId(1);
    let mut game = Game::default();
    let solid_tile_id = game.tile_registry.insert(engine_core::tiles::TileDef {
        sprite_id: SpriteId(1),
        components: vec![tile_definition_component_snapshot(Solid(true))],
    });
    let empty_tile_id = game.tile_registry.insert(engine_core::tiles::TileDef {
        sprite_id: SpriteId(2),
        components: vec![],
    });

    for y in 0..GRID_SIDE {
        for x in 0..GRID_SIDE {
            let tile_id = if (x + y) % 2 == 0 {
                solid_tile_id
            } else {
                empty_tile_id
            };
            let entity = game
                .ecs
                .create_entity()
                .with(TilePlacement::new(tile_id, x, y))
                .with_current_room(room_id)
                .finish();
            let mut ctx = game.ctx_mut();
            apply_tile_definition_to_entity(&mut ctx, entity, tile_id);
        }
    }

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let collision_world = CollisionWorld::new(&game.ecs, room, &world);
    let solid_tiles = collision_world
        .solids
        .iter()
        .filter(|solid| solid.entity.is_some())
        .count();

    assert_eq!(solid_tiles, GRID_SIDE * GRID_SIDE / 2);
}

#[test]
fn circle_cast_corner_misses_when_too_far() {
    let result = circle_cast_corner(
        Vec2::new(0.0, -10.0),
        Vec2::new(10.0, 0.0),
        8.0,
        Vec2::new(0.0, 0.0),
    );
    assert!(result.is_none());
}

#[test]
fn circle_cast_corner_entry_in_past_when_already_inside() {
    let result = circle_cast_corner(
        Vec2::new(0.0, -7.0),
        Vec2::new(10.0, 0.0),
        8.0,
        Vec2::new(0.0, 0.0),
    );
    assert!(result.is_none());
}

#[test]
fn circle_cast_corner_hits_diagonal_approach() {
    let result = circle_cast_corner(
        Vec2::new(-10.0, -10.0),
        Vec2::new(10.0, 10.0),
        8.0,
        Vec2::new(0.0, 0.0),
    );
    let t = match result {
        Some(t) => t,
        None => panic!("circle should hit the corner"),
    };
    assert!(t > 0.4 && t < 0.5, "expected t ≈ 0.434, got {t}");
}

#[test]
fn sweep_circle_blocked_by_corner_from_above() {
    let world = world_with_solid((
        Vec2::new(0.0, 0.0),
        Vec2::new(16.0, 16.0),
    ));
    let result = world.sweep_circle(
        Vec2::new(-10.0, -7.0),
        8.0,
        Vec2::new(10.0, 0.0),
        dummy_entity(),
        RoomLayer::Front,
        None,
    );
    assert!(result.blocked_x, "circle should be blocked horizontally by corner");
    assert!(result.t_x < 1.0, "t_x should be less than 1.0");
}

#[test]
fn sweep_circle_not_blocked_when_passing_above_obstacle() {
    let world = world_with_solid((
        Vec2::new(0.0, 0.0),
        Vec2::new(16.0, 16.0),
    ));
    let result = world.sweep_circle(
        Vec2::new(0.0, -17.0),
        8.0,
        Vec2::new(10.0, 0.0),
        dummy_entity(),
        RoomLayer::Front,
        None,
    );
    assert!(!result.blocked_x, "circle should pass above obstacle");
    assert!(!result.blocked_y, "circle should pass above obstacle");
}

#[test]
fn sweep_capsule_not_pushed_up_when_walking_into_wall() {
    let world = world_with_solid((
        Vec2::new(16.0, -8.0),
        Vec2::new(24.0, 0.0),
    ));
    let result = world.sweep_capsule(
        Vec2::new(0.0, -17.0),
        8.0,
        16.0,
        Vec2::new(10.0, 0.0),
        dummy_entity(),
        RoomLayer::Front,
        None,
    );
    assert!(result.blocked_x, "capsule should be blocked horizontally");
    assert!(
        result.push_y.abs() < 0.01,
        "capsule should not be pushed up, got push_y={}",
        result.push_y
    );
}

#[test]
fn circle_depenetration_pushes_along_dominant_axis_only() {
    let world = world_with_solid((
        Vec2::new(16.0, -8.0),
        Vec2::new(24.0, 0.0),
    ));
    let result = world.sweep_circle(
        Vec2::new(10.0, -8.5),
        8.0,
        Vec2::new(2.0, 0.0),
        dummy_entity(),
        RoomLayer::Front,
        None,
    );
    assert!(
        result.push_x < -0.01,
        "circle should be pushed left, got push_x={}",
        result.push_x
    );
    assert!(
        result.push_y.abs() < 0.01,
        "circle should not be pushed up, got push_y={}",
        result.push_y
    );
}

#[test]
fn capsule_walking_into_wall_multi_frame_no_climb() {
    let world = CollisionWorld {
        solids: vec![
            SolidObj {
                aabb: (Vec2::new(16.0, -8.0), Vec2::new(24.0, 0.0)),
                shape: ColliderShape::Aabb {
                    width: 8.0,
                    height: 8.0,
                },
                shape_pos: Vec2::new(16.0, -8.0),
                entity: None,
                layer: None,
                interior_zone: None,
            },
            SolidObj {
                aabb: (Vec2::new(0.0, 0.0), Vec2::new(32.0, 16.0)),
                shape: ColliderShape::Aabb {
                    width: 32.0,
                    height: 16.0,
                },
                shape_pos: Vec2::new(0.0, 0.0),
                entity: None,
                layer: None,
                interior_zone: None,
            },
        ],
        entity_layers: Default::default(),
        back_interior_zones: Vec::new(),
    };
    let mut center = Vec2::new(0.0, -16.0);
    let radius = 8.0;
    let height = 16.0;
    let dt = 1.0 / 60.0;
    let gravity = 800.0;
    let walk_speed = 120.0;
    let mut vel_y = 0.0f32;
    let mut sub_pixel = SubPixel::default();
    let entity = dummy_entity();

    for _frame in 0..120 {
        vel_y += gravity * dt;
        let delta = Vec2::new(walk_speed * dt, vel_y * dt);
        let true_pos = center + Vec2::new(sub_pixel.x, sub_pixel.y);

        let sweep = world.sweep_capsule(
            true_pos,
            radius,
            height,
            delta,
            entity,
            RoomLayer::Front,
            None,
        );
        let result = sweep.finish(delta);

        let new_true = true_pos + result.allowed_delta;
        let new_int = new_true.round();
        sub_pixel.x = new_true.x - new_int.x;
        sub_pixel.y = new_true.y - new_int.y;
        center = new_int;

        if result.blocked_x {
            sub_pixel.x = 0.0;
        }
        if result.blocked_y {
            vel_y = 0.0;
            sub_pixel.y = 0.0;
        }

        assert!(
            center.y >= -16.0,
            "frame {_frame}: capsule climbed to y={}, started at y=-16",
            center.y
        );
        let body_right = center.x + radius;
        assert!(
            body_right <= 16.0 + 0.01,
            "frame {_frame}: capsule body right edge at {body_right}, obstacle left at 16"
        );
    }
}
