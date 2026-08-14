use super::*;

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

