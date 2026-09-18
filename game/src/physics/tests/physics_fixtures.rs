use bishop::prelude::Vec2;
use engine_core::ecs::{
    Active,
    Collider,
    ColliderShape,
    Ecs,
    Entity,
    Grounded,
    KinematicAxis,
    KinematicDirection,
    KinematicMotion,
    KinematicMotionMode,
    PhysicsBody,
    Pivot,
    SubPixel,
    Transform,
    Velocity,
};
use engine_core::tiles::TileMap;
use engine_core::worlds::{Room, RoomId, RoomVariant, World};

pub(crate) const DT: f32 = 1.0 / 60.0;
pub(crate) const GRID_SIZE: f32 = 16.0;
pub(crate) const TEST_ROOM_ID: RoomId = RoomId(1);

/// Creates a one-room zero-gravity world for physics tests.
pub(crate) fn single_room_test_world() -> World {
    let room = Room {
        id: TEST_ROOM_ID,
        position: Vec2::ZERO,
        variants: vec![RoomVariant {
            tilemap: TileMap::new(10, 10),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut world = World::default();
    world.grid_size = GRID_SIZE;
    world.gravity = 0.0;
    world.current_room_id = Some(room.id);
    world.add_room(room);
    world
}

/// Creates an AABB collider with zero offset for physics tests.
pub(crate) fn aabb_collider(width: f32, height: f32) -> Collider {
    Collider {
        shape: ColliderShape::Aabb { width, height },
        offset: Vec2::ZERO,
    }
}

/// Spawns an active AABB physics body in a physics test room.
pub(crate) fn spawn_aabb_physics_body(
    ecs: &mut Ecs,
    room_id: RoomId,
    position: Vec2,
    size: Vec2,
    velocity: Vec2,
    grounded: bool,
) -> Entity {
    ecs.create_entity()
        .with_current_room(room_id)
        .with(Transform {
            position,
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(aabb_collider(size.x, size.y))
        .with(Velocity {
            x: velocity.x,
            y: velocity.y,
        })
        .with(PhysicsBody)
        .with(Grounded(grounded))
        .with(SubPixel::default())
        .with(Active::default())
        .finish()
}

/// Creates constant horizontal kinematic motion for physics tests.
pub(crate) fn horizontal_constant(speed: f32) -> KinematicMotion {
    KinematicMotion {
        mode: KinematicMotionMode::Constant,
        axis: KinematicAxis::Horizontal,
        direction: if speed >= 0.0 {
            KinematicDirection::Positive
        } else {
            KinematicDirection::Negative
        },
        speed: speed.abs(),
        travel_distance: 0.0,
    }
}
