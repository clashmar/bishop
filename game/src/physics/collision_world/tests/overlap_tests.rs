use super::*;
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

    assert!(!sweep.blocked_x);
}


#[test]
fn collision_world_check_overlaps_returns_empty_when_no_sensors() {
    let ecs = Ecs::default();
    let world = empty_world();
    let room = world.get_room(RoomId(1)).unwrap();
    let cw = CollisionWorld::new(&ecs, room, &world);
    let overlaps = cw.check_overlaps(
        Vec2::ZERO,
        Collider::default(),
        Pivot::TopLeft,
    );
    assert!(overlaps.is_empty());
}