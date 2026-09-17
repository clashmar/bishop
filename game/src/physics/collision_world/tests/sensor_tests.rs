use super::*;

fn aabb(width: f32, height: f32) -> Collider {
    Collider {
        shape: ColliderShape::Aabb { width, height },
        offset: Vec2::ZERO,
    }
}

fn circle(radius: f32) -> Collider {
    Collider {
        shape: ColliderShape::Circle { radius },
        offset: Vec2::ZERO,
    }
}

#[test]
fn collision_world_sensor_overlaps_when_body_overlaps_sensor_returns_sensor_entity() {
    let mut ecs = Ecs::default();
    let body = ecs
        .create_entity()
        .with_current_room(RoomId(1))
        .with(Transform {
            position: Vec2::ZERO,
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .finish();
    let sensor = ecs
        .create_entity()
        .with_current_room(RoomId(1))
        .with(Transform {
            position: Vec2::new(4.0, 0.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(aabb(8.0, 8.0))
        .with(Sensor)
        .finish();

    let world = empty_world();
    let room = world.get_room(RoomId(1)).unwrap();
    let collision_world = CollisionWorld::new(&ecs, room, &world);

    assert_eq!(
        collision_world.check_sensor_overlaps(body, Vec2::ZERO, aabb(8.0, 8.0), Pivot::TopLeft),
        vec![sensor]
    );
}

#[test]
fn collision_world_sensor_overlaps_when_aabbs_overlap_but_shapes_do_not_returns_empty() {
    let mut ecs = Ecs::default();
    let body = ecs
        .create_entity()
        .with_current_room(RoomId(1))
        .with(Transform {
            position: Vec2::ZERO,
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .finish();
    ecs.create_entity()
        .with_current_room(RoomId(1))
        .with(Transform {
            position: Vec2::new(14.0, 14.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(circle(8.0))
        .with(Sensor)
        .finish();

    let world = empty_world();
    let room = world.get_room(RoomId(1)).unwrap();
    let collision_world = CollisionWorld::new(&ecs, room, &world);

    assert!(
        collision_world
            .check_sensor_overlaps(body, Vec2::ZERO, circle(8.0), Pivot::TopLeft)
            .is_empty()
    );
}

#[test]
fn collision_world_sweep_move_when_sensor_is_in_path_does_not_block() {
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
        .with_current_room(RoomId(1))
        .with(Transform {
            position: Vec2::new(12.0, 0.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(aabb(8.0, 8.0))
        .with(Sensor)
        .finish();

    let world = empty_world();
    let room = world.get_room(RoomId(1)).unwrap();
    let collision_world = CollisionWorld::new(&ecs, room, &world);
    let sweep = collision_world.sweep_move(
        mover,
        Vec2::ZERO,
        Vec2::new(16.0, 0.0),
        aabb(8.0, 8.0),
        Pivot::TopLeft,
    );

    assert!(!sweep.blocked_x);
    assert_eq!(sweep.allowed_delta.x, 16.0);
}
