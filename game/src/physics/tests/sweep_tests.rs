use bishop::prelude::*;
use engine_core::assets::SpriteManager;
use engine_core::ecs::*;
use engine_core::tiles::TileMap;
use engine_core::worlds::*;

use crate::physics::collision_world::*;

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
    let room = empty_room();
    let mut world = World::default();
    world.grid_size = 16.0;
    world.current_room_id = Some(room.id);
    world.add_room(room);
    world
}

fn world_with_bottom_border() -> World {
    let room = Room {
        id: RoomId(1),
        position: Vec2::ZERO,
        variants: vec![RoomVariant {
            tilemap: TileMap::new(8, 4),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut world = World::default();
    world.grid_size = 16.0;
    world.current_room_id = Some(room.id);
    world.add_room(room);
    world
}

fn assert_capsule_pressing_into_circle_jump_keeps_vertical_motion(
    start_x: f32,
    delta_x: f32,
) {
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let mover = ecs
        .create_entity()
        .with_current_room(room_id)
        .with(Transform {
            pivot: Pivot::BottomCenter,
            ..Default::default()
        })
        .finish();
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(52.0, 64.0),
            pivot: Pivot::BottomCenter,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Circle { radius: 8.0 },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = world_with_bottom_border();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::new(start_x, 60.88889),
        Vec2::new(delta_x, -2.888889),
        Collider {
            shape: ColliderShape::Capsule {
                radius: 3.0,
                height: 10.0,
            },
            ..Default::default()
        },
        Pivot::BottomCenter,
    );

    assert!(
        !sweep.blocked_y,
        "capsule should keep upward motion while pressing into circle: {:?}",
        sweep.allowed_delta
    );
    assert!(
        sweep.allowed_delta.y < -2.8,
        "expected full upward motion, got {:?}",
        sweep.allowed_delta
    );
}

#[test]
fn collision_world_sweep_move_aabb_blocked_by_solid_entity() {
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
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(12.0, 0.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
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
fn collision_world_sweep_move_other_room_entity_does_not_block() {
    let mut ecs = Ecs::default();
    let mover = ecs
        .create_entity()
        .with_current_room(RoomId(1))
        .with(Transform {
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .finish();
    ecs.create_entity()
        .with_current_room(RoomId(2))
        .with(Transform {
            position: Vec2::new(12.0, 0.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = empty_world();
    let room = world.get_room(RoomId(1)).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
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

    assert!(!sweep.blocked_x);
}

#[test]
fn collision_world_sweep_move_circle_blocked_by_solid_entity() {
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
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(14.0, 0.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::ZERO,
        Vec2::new(20.0, 0.0),
        Collider {
            shape: ColliderShape::Circle { radius: 5.0 },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(sweep.blocked_x);
    assert!((sweep.allowed_delta.x - 4.0).abs() < 0.01);
}

#[test]
fn collision_world_sweep_move_circle_vs_solid_circle_uses_true_arc_boundary() {
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
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(16.0, 16.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Circle { radius: 8.0 },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::new(0.0, 12.0),
        Vec2::new(20.0, 0.0),
        Collider {
            shape: ColliderShape::Circle { radius: 4.0 },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(sweep.blocked_x);
    assert!(
        sweep.allowed_delta.x > 10.5 && sweep.allowed_delta.x < 11.6,
        "expected arc contact around 11.06px, got {}",
        sweep.allowed_delta.x
    );
}

#[test]
fn collision_world_sweep_move_capsule_vs_solid_circle_uses_true_arc_boundary() {
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
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(16.0, 16.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Circle { radius: 8.0 },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::new(0.0, 2.0),
        Vec2::new(20.0, 0.0),
        Collider {
            shape: ColliderShape::Capsule {
                radius: 4.0,
                height: 10.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(sweep.blocked_x);
    assert!(
        sweep.allowed_delta.x > 10.5 && sweep.allowed_delta.x < 11.6,
        "expected capsule arc contact around 11.06px, got {}",
        sweep.allowed_delta.x
    );
}

#[test]
fn collision_world_sweep_move_aabb_vs_solid_circle_blocks_on_arc() {
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
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(16.0, 16.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Circle { radius: 8.0 },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::new(8.0, 0.0),
        Vec2::new(0.0, 40.0),
        Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(sweep.blocked_y);
    assert!(
        (sweep.allowed_delta.y - 16.0).abs() < 0.01,
        "expected top-left box to catch the circle at y=16, got {}",
        sweep.allowed_delta.y
    );
}

#[test]
fn collision_world_sweep_move_circle_beside_solid_circle_keeps_vertical_motion() {
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
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(44.0, 48.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Circle { radius: 8.0 },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::new(38.0, 58.0),
        Vec2::new(0.0, -8.0),
        Collider {
            shape: ColliderShape::Circle { radius: 3.0 },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(!sweep.blocked_x);
    assert!(
        !sweep.blocked_y,
        "circle vertical motion should stay free beside solid circle: {:?}",
        sweep.allowed_delta
    );
    assert!(
        sweep.allowed_delta.y < -7.9,
        "expected full upward motion, got {:?}",
        sweep.allowed_delta
    );
}

#[test]
fn collision_world_sweep_move_capsule_walk_state_beside_circle_keeps_vertical_motion() {
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
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(44.0, 48.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Circle { radius: 8.0 },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::new(38.0, 48.0),
        Vec2::new(0.0, -8.0),
        Collider {
            shape: ColliderShape::Capsule {
                radius: 3.0,
                height: 10.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(!sweep.blocked_x);
    assert!(
        !sweep.blocked_y,
        "capsule walk-state vertical motion should stay free beside solid circle: {:?}",
        sweep.allowed_delta
    );
    assert!(
        sweep.allowed_delta.y < -7.9,
        "expected full upward motion, got {:?}",
        sweep.allowed_delta
    );
}

#[test]
fn collision_world_sweep_move_capsule_second_jump_frame_beside_circle_keeps_vertical_motion() {
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
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(44.0, 48.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Circle { radius: 8.0 },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::new(38.0, 45.0),
        Vec2::new(0.0, -2.9),
        Collider {
            shape: ColliderShape::Capsule {
                radius: 3.0,
                height: 10.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(!sweep.blocked_x);
    assert!(
        !sweep.blocked_y,
        "capsule second jump frame should stay free beside solid circle: {:?}",
        sweep.allowed_delta
    );
    assert!(
        sweep.allowed_delta.y < -2.8,
        "expected full upward motion, got {:?}",
        sweep.allowed_delta
    );
}

#[test]
fn collision_world_sweep_move_capsule_subpixel_second_jump_frame_beside_circle_keeps_vertical_motion() {
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
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(44.0, 48.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Circle { radius: 8.0 },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::new(38.0, 44.88889),
        Vec2::new(0.0, -2.888889),
        Collider {
            shape: ColliderShape::Capsule {
                radius: 3.0,
                height: 10.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(!sweep.blocked_x);
    assert!(
        !sweep.blocked_y,
        "capsule subpixel second jump frame should stay free beside solid circle: {:?}",
        sweep.allowed_delta
    );
    assert!(
        sweep.allowed_delta.y < -2.8,
        "expected full upward motion, got {:?}",
        sweep.allowed_delta
    );
}

#[test]
fn collision_world_sweep_move_capsule_subpixel_second_jump_frame_with_floor_and_circle_keeps_vertical_motion() {
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let mover = ecs
        .create_entity()
        .with_current_room(room_id)
        .with(Transform {
            pivot: Pivot::BottomCenter,
            ..Default::default()
        })
        .finish();
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(52.0, 64.0),
            pivot: Pivot::BottomCenter,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Circle { radius: 8.0 },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = world_with_bottom_border();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::new(41.0, 60.88889),
        Vec2::new(0.0, -2.888889),
        Collider {
            shape: ColliderShape::Capsule {
                radius: 3.0,
                height: 10.0,
            },
            ..Default::default()
        },
        Pivot::BottomCenter,
    );

    assert!(!sweep.blocked_x);
    assert!(
        !sweep.blocked_y,
        "capsule subpixel second jump frame with floor and circle should stay free: {:?}",
        sweep.allowed_delta
    );
    assert!(
        sweep.allowed_delta.y < -2.8,
        "expected full upward motion, got {:?}",
        sweep.allowed_delta
    );
}

#[test]
fn collision_world_sweep_move_capsule_pressing_right_into_circle_while_jumping_keeps_vertical_motion() {
    assert_capsule_pressing_into_circle_jump_keeps_vertical_motion(
        41.0,
        1.6666667,
    );
}

#[test]
fn collision_world_sweep_move_capsule_pressing_left_into_circle_while_jumping_keeps_vertical_motion() {
    assert_capsule_pressing_into_circle_jump_keeps_vertical_motion(
        63.0,
        -1.6666667,
    );
}

#[test]
fn collision_world_sweep_move_aabb_beside_circle_keeps_vertical_motion() {
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
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(16.0, 16.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Circle { radius: 8.0 },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::new(8.0, 20.0),
        Vec2::new(0.0, -8.0),
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
    assert!(!sweep.blocked_y, "vertical motion should stay free beside circle: {:?}", sweep.allowed_delta);
    assert!(sweep.allowed_delta.y < -7.9, "expected full upward motion, got {:?}", sweep.allowed_delta);
}

#[test]
fn collision_world_sweep_move_capsule_beside_circle_keeps_vertical_motion() {
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
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(16.0, 16.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Circle { radius: 8.0 },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::new(8.0, 19.0),
        Vec2::new(0.0, -8.0),
        Collider {
            shape: ColliderShape::Capsule {
                radius: 4.0,
                height: 10.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(!sweep.blocked_x);
    assert!(!sweep.blocked_y, "capsule vertical motion should stay free beside circle: {:?}", sweep.allowed_delta);
    assert!(sweep.allowed_delta.y < -7.9, "expected full upward motion, got {:?}", sweep.allowed_delta);
}

#[test]
fn collision_world_sweep_move_aabb_on_circle_keeps_horizontal_motion() {
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
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(16.0, 16.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Circle { radius: 8.0 },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::new(20.0, 8.0),
        Vec2::new(4.0, 1.0),
        Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(!sweep.blocked_x, "horizontal motion should stay free on top contact: {:?}", sweep.allowed_delta);
    assert!(sweep.allowed_delta.x > 3.9, "expected full horizontal motion, got {:?}", sweep.allowed_delta);
    assert!(sweep.blocked_y);
}

#[test]
fn collision_world_sweep_move_capsule_on_circle_keeps_horizontal_motion() {
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
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(16.0, 16.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Circle { radius: 8.0 },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::new(20.0, -2.0),
        Vec2::new(4.0, 1.0),
        Collider {
            shape: ColliderShape::Capsule {
                radius: 4.0,
                height: 10.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(!sweep.blocked_x, "capsule should keep horizontal motion on top contact: {:?}", sweep.allowed_delta);
    assert!(sweep.allowed_delta.x > 3.9, "expected full horizontal motion, got {:?}", sweep.allowed_delta);
    assert!(sweep.blocked_y);
}

#[test]
fn collision_world_sweep_move_aabb_corner_touch_does_not_block_perpendicular_axis() {
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
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(10.0, 0.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::ZERO,
        Vec2::new(20.0, 20.0),
        Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!((sweep.allowed_delta.x - 2.0).abs() < 0.01);
    assert!(sweep.blocked_x);
    assert!(!sweep.blocked_y);
}

#[test]
fn collision_world_sweep_move_capsule_left_contact_does_not_slide_through_tile() {
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
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(10.0, 10.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 16.0,
            },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::new(12.0, 0.0),
        Vec2::new(-20.0, 0.0),
        Collider {
            shape: ColliderShape::Capsule {
                radius: 4.0,
                height: 10.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(sweep.blocked_x);
    assert!(sweep.allowed_delta.x >= 0.0);
}

#[test]
fn collision_world_sweep_move_capsule_embedded_in_tile_can_move_outward() {
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
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(10.0, 10.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 16.0,
            },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::new(12.0, 0.0),
        Vec2::new(20.0, 0.0),
        Collider {
            shape: ColliderShape::Capsule {
                radius: 4.0,
                height: 10.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(!sweep.blocked_x, "capsule should move outward when embedded in a tile: {:?}", sweep.allowed_delta);
    assert!(sweep.allowed_delta.x > 19.9, "expected full release motion, got {:?}", sweep.allowed_delta);
}

#[test]
fn collision_world_sweep_move_capsule_corner_support_blocks_downward_motion() {
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
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position: Vec2::new(0.0, 32.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Aabb {
                width: 16.0,
                height: 16.0,
            },
            ..Default::default()
        })
        .with(Solid(true))
        .finish();

    let world = empty_world();
    let room = world.get_room(room_id).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let sweep = cw.sweep_move(
        mover,
        Vec2::new(12.0, 14.0),
        Vec2::new(0.0, 1.0),
        Collider {
            shape: ColliderShape::Capsule {
                radius: 4.0,
                height: 10.0,
            },
            ..Default::default()
        },
        Pivot::TopLeft,
    );

    assert!(sweep.blocked_y);
    assert!(sweep.allowed_delta.y <= 0.0);
}

#[test]
fn collision_world_check_overlaps_returns_empty_when_no_sensors() {
    let ecs = Ecs::default();
    let world = empty_world();
    let room = world.get_room(RoomId(1)).unwrap();
    let cw = CollisionWorld::new(&SpriteManager::default(), &ecs, room, &world);
    let overlaps = cw.check_overlaps(
        Vec2::ZERO,
        Collider::default(),
        Pivot::TopLeft,
    );
    assert!(overlaps.is_empty());
}