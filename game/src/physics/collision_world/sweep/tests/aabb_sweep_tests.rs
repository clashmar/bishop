use super::*;
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
    let cw = CollisionWorld::new(&ecs, room, &world);
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
    let cw = CollisionWorld::new(&ecs, room, &world);
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
    let cw = CollisionWorld::new(&ecs, room, &world);
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
    let cw = CollisionWorld::new(&ecs, room, &world);
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

