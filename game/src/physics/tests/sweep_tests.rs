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
    // Circle radius=5, AABB width=10. At x=0, right edge at x=10.
    // Obstacle at x=14. Allowed = 14 - 10 = 4.0
    assert!((sweep.allowed_delta.x - 4.0).abs() < 0.01);
}

#[test]
fn collision_world_sweep_move_x_then_y_resolution_order() {
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
    // Place a solid entity to the right — blocks X movement
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

    // X is resolved first: mover right edge at x=8, solid left edge at x=10.
    // Allowed = 10 - 8 = 2.0
    assert!((sweep.allowed_delta.x - 2.0).abs() < 0.01);
    assert!(sweep.blocked_x);
    // Y is resolved from post-X position (x=2). Mover x range 2..10,
    // solid x range 10..18. No overlap on X (just touching at x=10).
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