use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::tiles::TileMap;
use engine_core::worlds::*;

use crate::physics::collision_world::*;

/// Returns a sweep-test world with a shorter room floor span.
pub(super) fn world_with_bottom_border() -> World {
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

/// Asserts that capsule jump motion survives horizontal circle pressure.
pub(super) fn assert_capsule_pressing_into_circle_jump_keeps_vertical_motion(
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
    let cw = CollisionWorld::new(&ecs, room, &world);
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
