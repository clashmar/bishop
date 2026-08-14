use super::*;
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
    let cw = CollisionWorld::new(&ecs, room, &world);
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
    let cw = CollisionWorld::new(&ecs, room, &world);
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
    let cw = CollisionWorld::new(&ecs, room, &world);
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

