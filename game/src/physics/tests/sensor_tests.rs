use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::tiles::TileMap;
use engine_core::worlds::*;

use crate::physics::events::{PhysicsEvent, SensorEvent};
use crate::physics::physics_system::update_physics;
use crate::physics::runtime::PhysicsRuntime;
use crate::physics::tests::physics_fixtures::{horizontal_constant, DT, GRID_SIZE};


fn test_world() -> World {
    let room = Room {
        id: RoomId(1),
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

fn aabb(width: f32, height: f32) -> Collider {
    Collider {
        shape: ColliderShape::Aabb { width, height },
        offset: Vec2::ZERO,
    }
}

fn spawn_sensor(ecs: &mut Ecs, position: Vec2) -> Entity {
    ecs.create_entity()
        .with_current_room(RoomId(1))
        .with(Transform {
            position,
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(aabb(16.0, 16.0))
        .with(Sensor)
        .finish()
}

fn spawn_dynamic_body(ecs: &mut Ecs, position: Vec2, velocity: Vec2) -> Entity {
    ecs.create_entity()
        .with_current_room(RoomId(1))
        .with(Transform {
            position,
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(aabb(8.0, 8.0))
        .with(Velocity {
            x: velocity.x,
            y: velocity.y,
        })
        .with(PhysicsBody)
        .with(Grounded(false))
        .with(SubPixel::default())
        .with(Active::default())
        .finish()
}

fn sensor_events(runtime: &mut PhysicsRuntime) -> Vec<SensorEvent> {
    runtime
        .events_mut()
        .drain()
        .into_iter()
        .filter_map(|event| match event {
            PhysicsEvent::Sensor(sensor_event) => Some(sensor_event),
            _ => None,
        })
        .collect()
}

#[test]
fn sensor_events_when_dynamic_enters_stays_and_exits_emit_expected_transitions() {
    let world = test_world();
    let mut ecs = Ecs::default();
    let sensor = spawn_sensor(&mut ecs, Vec2::new(16.0, 0.0));
    let body = spawn_dynamic_body(&mut ecs, Vec2::ZERO, Vec2::new(960.0, 0.0));
    let mut runtime = PhysicsRuntime::default();

    update_physics(&mut ecs, &world, DT, &mut runtime);
    assert_eq!(sensor_events(&mut runtime), vec![SensorEvent::Enter { body, sensor }]);

    if let Some(velocity) = ecs.get_mut::<Velocity>(body) {
        velocity.x = 0.0;
    }
    update_physics(&mut ecs, &world, DT, &mut runtime);
    assert_eq!(sensor_events(&mut runtime), vec![SensorEvent::Stay { body, sensor }]);

    if let Some(transform) = ecs.get_mut::<Transform>(body) {
        transform.position = Vec2::new(48.0, 0.0);
    }
    update_physics(&mut ecs, &world, DT, &mut runtime);
    assert_eq!(sensor_events(&mut runtime), vec![SensorEvent::Exit { body, sensor }]);
}

#[test]
fn sensor_events_when_kinematic_enters_stays_and_exits_emit_expected_transitions() {
    let world = test_world();
    let mut ecs = Ecs::default();
    let sensor = spawn_sensor(&mut ecs, Vec2::new(16.0, 0.0));
    let mut kinematic = Kinematic::default();
    kinematic.motion = horizontal_constant(960.0);
    let body = ecs.create_entity()
        .with_current_room(RoomId(1))
        .with(Transform {
            position: Vec2::ZERO,
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(aabb(8.0, 8.0))
        .with(Velocity::default())
        .with(kinematic)
        .with(Active::default())
        .finish();
    let mut runtime = PhysicsRuntime::default();

    update_physics(&mut ecs, &world, DT, &mut runtime);
    assert_eq!(sensor_events(&mut runtime), vec![SensorEvent::Enter { body, sensor }]);

    if let Some(kinematic) = ecs.get_mut::<Kinematic>(body) {
        kinematic.motion = KinematicMotion::default();
    }
    update_physics(&mut ecs, &world, DT, &mut runtime);
    assert_eq!(sensor_events(&mut runtime), vec![SensorEvent::Stay { body, sensor }]);

    if let Some(transform) = ecs.get_mut::<Transform>(body) {
        transform.position = Vec2::new(48.0, 0.0);
    }
    update_physics(&mut ecs, &world, DT, &mut runtime);
    assert_eq!(sensor_events(&mut runtime), vec![SensorEvent::Exit { body, sensor }]);
}

#[test]
fn sensor_events_when_sensor_volume_moves_uses_final_sensor_position() {
    let world = test_world();
    let mut ecs = Ecs::default();
    let body = spawn_dynamic_body(&mut ecs, Vec2::new(16.0, 0.0), Vec2::ZERO);
    let mut kinematic = Kinematic::default();
    kinematic.motion = horizontal_constant(960.0);
    let sensor = ecs.create_entity()
        .with_current_room(RoomId(1))
        .with(Transform {
            position: Vec2::ZERO,
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(aabb(8.0, 8.0))
        .with(Velocity::default())
        .with(kinematic)
        .with(Sensor)
        .with(Active::default())
        .finish();
    let mut runtime = PhysicsRuntime::default();

    update_physics(&mut ecs, &world, DT, &mut runtime);

    assert_eq!(sensor_events(&mut runtime), vec![SensorEvent::Enter { body, sensor }]);
}

#[test]
fn sensor_events_when_sensor_has_physics_body_does_not_trigger_another_sensor() {
    let world = test_world();
    let mut ecs = Ecs::default();
    let target_sensor = spawn_sensor(&mut ecs, Vec2::new(16.0, 0.0));
    let moving_sensor = spawn_dynamic_body(&mut ecs, Vec2::ZERO, Vec2::new(960.0, 0.0));
    ecs.replace_component(moving_sensor, Sensor);
    let mut runtime = PhysicsRuntime::default();

    update_physics(&mut ecs, &world, DT, &mut runtime);

    assert_ne!(moving_sensor, target_sensor);
    assert!(sensor_events(&mut runtime).is_empty());
}

#[test]
fn sensor_events_when_body_crosses_sensor_does_not_block_or_ground_body() {
    let world = test_world();
    let mut ecs = Ecs::default();
    spawn_sensor(&mut ecs, Vec2::new(16.0, 0.0));
    let body = spawn_dynamic_body(&mut ecs, Vec2::ZERO, Vec2::new(960.0, 0.0));
    let mut runtime = PhysicsRuntime::default();

    update_physics(&mut ecs, &world, DT, &mut runtime);

    assert_eq!(ecs.get::<Transform>(body).unwrap().position.x, 16.0);
    assert!(!ecs.get::<Grounded>(body).unwrap().0);
}
