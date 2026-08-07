use super::*;

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
        bounds: InteriorZoneBounds::new(0, 0, 32, 64),
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
            bounds: InteriorZoneBounds::new(0, 0, 32, 64),
        },
        InteriorZone {
            id: InteriorZoneId(2),
            bounds: InteriorZoneBounds::new(32, 0, 32, 64),
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

