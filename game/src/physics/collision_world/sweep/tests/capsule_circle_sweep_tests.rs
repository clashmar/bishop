use super::*;
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
    let cw = CollisionWorld::new(&ecs, room, &world);
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
    let cw = CollisionWorld::new(&ecs, room, &world);
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
    let cw = CollisionWorld::new(&ecs, room, &world);
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
    let cw = CollisionWorld::new(&ecs, room, &world);
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
    let cw = CollisionWorld::new(&ecs, room, &world);
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
    let cw = CollisionWorld::new(&ecs, room, &world);
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
    let cw = CollisionWorld::new(&ecs, room, &world);
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


