use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::tiles::TileMap;
use engine_core::worlds::*;

use crate::physics::physics_system::update_physics;

const DT: f32 = 1.0 / 60.0;
const EPSILON: f32 = 0.0001;

fn gravity_test_world(grid_size: f32, gravity: f32) -> World {
    let room = Room {
        id: RoomId(1),
        position: Vec2::ZERO,
        variants: vec![RoomVariant {
            tilemap: TileMap::new(8, 8),
            ..Default::default()
        }],
        ..Default::default()
    };

    let mut world = World::default();
    world.grid_size = grid_size;
    world.gravity = gravity;
    world.current_room_id = Some(room.id);
    world.add_room(room);
    world
}

fn spawn_falling_body(ecs: &mut Ecs) -> Entity {
    spawn_falling_body_with_gravity_scale(ecs, GravityScale::default())
}

fn vertical_velocity(ecs: &Ecs, entity: Entity) -> f32 {
    ecs.get::<Velocity>(entity).unwrap().y
}

#[test]
fn physics_body_zero_world_gravity_keeps_vertical_velocity() {
    let world = gravity_test_world(16.0, 0.0);
    let mut ecs = Ecs::default();
    let entity = spawn_falling_body(&mut ecs);

    update_physics(&mut ecs, &world, DT);

    assert!(vertical_velocity(&ecs, entity).abs() < EPSILON);
}

#[test]
fn physics_body_world_gravity_uses_grid_size_conversion() {
    let grid_size = 16.0;
    let gravity = 25.0;
    let world = gravity_test_world(grid_size, gravity);
    let mut ecs = Ecs::default();
    let entity = spawn_falling_body(&mut ecs);

    update_physics(&mut ecs, &world, DT);

    let expected = gravity * grid_size * DT;
    assert!((vertical_velocity(&ecs, entity) - expected).abs() < EPSILON);
}

#[test]
fn physics_body_negative_world_gravity_accelerates_upward() {
    let grid_size = 16.0;
    let gravity = -25.0;
    let world = gravity_test_world(grid_size, gravity);
    let mut ecs = Ecs::default();
    let entity = spawn_falling_body(&mut ecs);

    update_physics(&mut ecs, &world, DT);

    let expected = gravity * grid_size * DT;
    assert!((vertical_velocity(&ecs, entity) - expected).abs() < EPSILON);
}

fn spawn_falling_body_with_gravity_scale(ecs: &mut Ecs, gravity_scale: GravityScale) -> Entity {
    ecs.create_entity()
        .with(Transform {
            position: Vec2::new(32.0, 32.0),
            pivot: Pivot::BottomCenter,
            ..Default::default()
        })
        .with(Velocity::default())
        .with(Collider {
            shape: ColliderShape::Capsule {
                radius: 3.0,
                height: 10.0,
            },
            offset: Vec2::ZERO,
        })
        .with(PhysicsBody)
        .with(Grounded(false))
        .with(SubPixel::default())
        .with(gravity_scale)
        .with_current_room(RoomId(1))
        .with(Active::default())
        .finish()
}

#[test]
fn physics_body_zero_gravity_scale_keeps_vertical_velocity() {
    let grid_size = 16.0;
    let gravity = 25.0;
    let world = gravity_test_world(grid_size, gravity);
    let mut ecs = Ecs::default();
    let entity = spawn_falling_body_with_gravity_scale(&mut ecs, GravityScale(0.0));

    update_physics(&mut ecs, &world, DT);

    assert!(vertical_velocity(&ecs, entity).abs() < EPSILON);
}

#[test]
fn physics_body_gravity_scale_multiplies_world_gravity() {
    let grid_size = 16.0;
    let gravity = 25.0;
    let scale = 2.0;
    let world = gravity_test_world(grid_size, gravity);
    let mut ecs = Ecs::default();
    let entity = spawn_falling_body_with_gravity_scale(&mut ecs, GravityScale(scale));

    update_physics(&mut ecs, &world, DT);

    let expected = gravity * grid_size * scale * DT;
    assert!((vertical_velocity(&ecs, entity) - expected).abs() < EPSILON);
}
