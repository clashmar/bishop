use super::*;
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
    let cw = CollisionWorld::new(&ecs, room, &world);
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
    let cw = CollisionWorld::new(&ecs, room, &world);
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
    let cw = CollisionWorld::new(&ecs, room, &world);
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

