use bishop::prelude::Vec2;
use crate::physics::collisions::{CollisionPair, CollisionPairTracker};
use crate::physics::events::{CollisionEvent, PhysicsEvent, PhysicsEvents};
use crate::physics::physics_system::update_physics;
use crate::physics::runtime::PhysicsRuntime;
use crate::physics::tests::physics_fixtures::{
    DT,
    TEST_ROOM_ID,
    aabb_collider,
    horizontal_constant,
    single_room_test_world as test_world,
    spawn_aabb_physics_body,
};
use engine_core::ecs::{
    Active,
    Ecs,
    Entity,
    Kinematic,
    KinematicContactBehavior,
    PhysicsBody,
    Pivot,
    Sensor,
    Transform,
};
use engine_core::worlds::RoomId;
use std::collections::HashSet;

fn collision_events(events: &mut PhysicsEvents) -> Vec<CollisionEvent> {
    events
        .drain()
        .into_iter()
        .filter_map(|event| match event {
            PhysicsEvent::Collision(collision) => Some(collision),
            _ => None,
        })
        .collect()
}

fn spawn_physics_body(ecs: &mut Ecs, room_id: RoomId, position: Vec2) -> Entity {
    spawn_physics_body_with_velocity(ecs, room_id, position, Vec2::ZERO)
}

fn spawn_physics_body_with_velocity(
    ecs: &mut Ecs,
    room_id: RoomId,
    position: Vec2,
    velocity: Vec2,
) -> Entity {
    spawn_aabb_physics_body(
        ecs,
        room_id,
        position,
        Vec2::new(16.0, 16.0),
        velocity,
        false,
    )
}

fn spawn_kinematic_body(ecs: &mut Ecs, room_id: RoomId, position: Vec2) -> Entity {
    ecs.create_entity()
        .with(Transform {
            position,
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(aabb_collider(16.0, 16.0))
        .with(Kinematic::default())
        .with_current_room(room_id)
        .with(Active::default())
        .finish()
}

fn collision_event_list(runtime: &mut PhysicsRuntime) -> Vec<CollisionEvent> {
    runtime
        .events_mut()
        .drain()
        .into_iter()
        .filter_map(|event| match event {
            PhysicsEvent::Collision(collision) => Some(collision),
            _ => None,
        })
        .collect()
}

#[test]
fn collision_pair_when_entities_are_reversed_is_canonical() {
    let first = Entity(1);
    let second = Entity(2);

    assert_eq!(CollisionPair::new(first, second), CollisionPair::new(second, first));
}

#[test]
fn collision_tracker_when_pair_is_new_emits_enter() {
    let first = Entity(1);
    let second = Entity(2);
    let pair = CollisionPair::new(first, second);
    let mut tracker = CollisionPairTracker::default();
    let mut events = PhysicsEvents::default();

    tracker.finish_frame(HashSet::from([pair]), &mut events);

    assert_eq!(
        collision_events(&mut events),
        vec![CollisionEvent::Enter { first, second }]
    );
}

#[test]
fn collision_tracker_when_pair_is_removed_emits_exit() {
    let first = Entity(1);
    let second = Entity(2);
    let pair = CollisionPair::new(first, second);
    let mut tracker = CollisionPairTracker::default();
    let mut events = PhysicsEvents::default();

    tracker.finish_frame(HashSet::from([pair]), &mut events);
    events.drain();
    tracker.finish_frame(HashSet::new(), &mut events);

    assert_eq!(
        collision_events(&mut events),
        vec![CollisionEvent::Exit { first, second }]
    );
}

#[test]
fn collision_tracker_when_pair_persists_does_not_emit_stay() {
    let first = Entity(1);
    let second = Entity(2);
    let pair = CollisionPair::new(first, second);
    let mut tracker = CollisionPairTracker::default();
    let mut events = PhysicsEvents::default();

    tracker.finish_frame(HashSet::from([pair]), &mut events);
    events.drain();
    tracker.finish_frame(HashSet::from([pair]), &mut events);

    assert!(collision_events(&mut events).is_empty());
}

#[test]
fn collision_pairs_when_entities_overlap_collects_canonical_pair() {
    let world = test_world();
    let room = world.get_room(TEST_ROOM_ID).unwrap();
    let mut ecs = Ecs::default();
    let first = spawn_physics_body(&mut ecs, TEST_ROOM_ID, Vec2::new(0.0, 0.0));
    let second = spawn_kinematic_body(&mut ecs, TEST_ROOM_ID, Vec2::new(8.0, 0.0));
    let mut pairs = HashSet::new();

    crate::physics::collisions::collect_collision_pairs(&ecs, room, &mut pairs);

    assert_eq!(pairs, HashSet::from([CollisionPair::new(first, second)]));
}

#[test]
fn collision_pairs_when_single_body_exists_does_not_collect_self_pair() {
    let world = test_world();
    let room = world.get_room(TEST_ROOM_ID).unwrap();
    let mut ecs = Ecs::default();
    spawn_physics_body(&mut ecs, TEST_ROOM_ID, Vec2::new(0.0, 0.0));
    let mut pairs = HashSet::new();

    crate::physics::collisions::collect_collision_pairs(&ecs, room, &mut pairs);

    assert!(pairs.is_empty());
}

#[test]
fn collision_pairs_when_entities_are_not_collision_bodies_excludes_them() {
    let world = test_world();
    let room = world.get_room(TEST_ROOM_ID).unwrap();
    let mut ecs = Ecs::default();
    let first = spawn_physics_body(&mut ecs, TEST_ROOM_ID, Vec2::new(0.0, 0.0));
    let sensor = ecs.create_entity()
        .with(Transform {
            position: Vec2::new(8.0, 0.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(aabb_collider(16.0, 16.0))
        .with(Sensor)
        .with(PhysicsBody)
        .with_current_room(TEST_ROOM_ID)
        .with(Active::default())
        .finish();
    let inactive = spawn_physics_body(&mut ecs, TEST_ROOM_ID, Vec2::new(8.0, 0.0));
    ecs.get_mut::<Active>(inactive).unwrap().active = false;
    let other_room = spawn_physics_body(
        &mut ecs,
        RoomId(TEST_ROOM_ID.0 + 1),
        Vec2::new(8.0, 0.0),
    );
    let decoration = ecs.create_entity()
        .with(Transform {
            position: Vec2::new(8.0, 0.0),
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(aabb_collider(16.0, 16.0))
        .with_current_room(TEST_ROOM_ID)
        .with(Active::default())
        .finish();
    let mut pairs = HashSet::new();

    crate::physics::collisions::collect_collision_pairs(&ecs, room, &mut pairs);

    assert_ne!(first, sensor);
    assert_ne!(first, inactive);
    assert_ne!(first, other_room);
    assert_ne!(first, decoration);
    assert!(pairs.is_empty());
}

#[test]
fn collision_events_when_pair_enters_and_exits_emit_expected_transitions() {
    let world = test_world();
    let mut ecs = Ecs::default();
    let first = spawn_physics_body(&mut ecs, TEST_ROOM_ID, Vec2::new(0.0, 0.0));
    let second = spawn_kinematic_body(&mut ecs, TEST_ROOM_ID, Vec2::new(8.0, 0.0));
    let mut runtime = PhysicsRuntime::default();

    update_physics(&mut ecs, &world, DT, &mut runtime);
    assert_eq!(
        collision_event_list(&mut runtime),
        vec![CollisionEvent::Enter { first, second }]
    );

    update_physics(&mut ecs, &world, DT, &mut runtime);
    assert!(collision_event_list(&mut runtime).is_empty());

    ecs.get_mut::<Transform>(second).unwrap().position = Vec2::new(48.0, 0.0);
    update_physics(&mut ecs, &world, DT, &mut runtime);
    assert_eq!(
        collision_event_list(&mut runtime),
        vec![CollisionEvent::Exit { first, second }]
    );
}

#[test]
fn collision_events_when_body_moves_into_overlap_use_final_position() {
    let world = test_world();
    let mut ecs = Ecs::default();
    let first = spawn_physics_body_with_velocity(
        &mut ecs,
        TEST_ROOM_ID,
        Vec2::new(32.0, 0.0),
        Vec2::new(960.0, 0.0),
    );
    let second = spawn_physics_body(&mut ecs, TEST_ROOM_ID, Vec2::new(56.0, 0.0));
    let mut runtime = PhysicsRuntime::default();

    update_physics(&mut ecs, &world, DT, &mut runtime);

    assert_eq!(ecs.get::<Transform>(first).unwrap().position.x, 48.0);
    assert_eq!(
        collision_event_list(&mut runtime),
        vec![CollisionEvent::Enter { first, second }]
    );
}

fn moving_crush_kinematic(ecs: &mut Ecs, room_id: RoomId, position: Vec2) -> Entity {
    let mut kinematic = Kinematic::default();
    kinematic.contact_behavior = KinematicContactBehavior::Crush;
    kinematic.motion = horizontal_constant(720.0);

    ecs.create_entity()
        .with(Transform {
            position,
            pivot: Pivot::TopLeft,
            ..Default::default()
        })
        .with(aabb_collider(16.0, 16.0))
        .with(kinematic)
        .with_current_room(room_id)
        .with(Active::default())
        .finish()
}

#[test]
fn collision_events_when_kinematic_crush_policy_pushes_dynamic_emits_squeeze() {
    let world = test_world();
    let mut ecs = Ecs::default();
    let dynamic = spawn_physics_body(&mut ecs, TEST_ROOM_ID, Vec2::new(48.0, 0.0));
    let kinematic = moving_crush_kinematic(&mut ecs, TEST_ROOM_ID, Vec2::new(34.0, 0.0));
    let mut runtime = PhysicsRuntime::default();

    update_physics(&mut ecs, &world, DT, &mut runtime);

    assert!(collision_event_list(&mut runtime).iter().any(|event| {
        matches!(
            event,
            CollisionEvent::Squeeze {
                kinematic: event_kinematic,
                dynamic: event_dynamic,
            } if *event_kinematic == kinematic && *event_dynamic == dynamic
        )
    }));
}
