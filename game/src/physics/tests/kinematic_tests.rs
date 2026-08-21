use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::tiles::TileMap;
use engine_core::worlds::*;

use crate::physics::collision_world::{shapes_overlap, CollisionWorld};
use crate::physics::events::{KinematicContactEvent, PhysicsEvent, PhysicsEvents};
use crate::physics::kinematic::{KinematicFrameMotion, resolve_kinematic_contacts};
use crate::physics::physics_system::{update_physics, update_physics_with_events};

const DT: f32 = 1.0 / 60.0;
const GRID_SIZE: f32 = 16.0;
const ROOM_HEIGHT_TILES: usize = 6;
const FLOOR_Y: f32 = ROOM_HEIGHT_TILES as f32 * GRID_SIZE;
const PLATFORM_WIDTH: f32 = 40.0;
const PLATFORM_HEIGHT: f32 = 40.0;
const JUMP_SPEED: f32 = 200.0;
const EPSILON: f32 = 0.0001;

fn room_with_bottom_border() -> Room {
    Room {
        id: RoomId(1),
        position: Vec2::ZERO,
        variants: vec![RoomVariant {
            tilemap: TileMap::new(8, ROOM_HEIGHT_TILES),
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn world_with_bottom_border() -> World {
    let room = room_with_bottom_border();
    let mut world = World::default();
    world.grid_size = GRID_SIZE;
    world.current_room_id = Some(room.id);
    world.add_room(room);
    world
}

fn horizontal_constant(speed: f32) -> KinematicMotion {
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

fn vertical_constant(speed: f32) -> KinematicMotion {
    KinematicMotion {
        mode: KinematicMotionMode::Constant,
        axis: KinematicAxis::Vertical,
        direction: if speed >= 0.0 {
            KinematicDirection::Positive
        } else {
            KinematicDirection::Negative
        },
        speed: speed.abs(),
        travel_distance: 0.0,
    }
}

fn horizontal_ping_pong(speed: f32, travel_distance: f32, direction: KinematicDirection) -> KinematicMotion {
    KinematicMotion {
        mode: KinematicMotionMode::PingPong,
        axis: KinematicAxis::Horizontal,
        direction,
        speed,
        travel_distance,
    }
}

fn spawn_kinematic_body(
    ecs: &mut Ecs,
    room_id: RoomId,
    position: Vec2,
    motion: KinematicMotion,
) -> Entity {
    spawn_kinematic_body_with_behavior(ecs, room_id, position, motion, KinematicContactBehavior::Stop)
}

fn spawn_kinematic_body_with_behavior(
    ecs: &mut Ecs,
    room_id: RoomId,
    position: Vec2,
    motion: KinematicMotion,
    contact_behavior: KinematicContactBehavior,
) -> Entity {
    spawn_kinematic_body_with_collider(
        ecs,
        room_id,
        position,
        Pivot::BottomCenter,
        Collider {
            shape: ColliderShape::Aabb {
                width: PLATFORM_WIDTH,
                height: PLATFORM_HEIGHT,
            },
            offset: Vec2::ZERO,
        },
        motion,
        contact_behavior,
    )
}

fn spawn_kinematic_body_with_collider(
    ecs: &mut Ecs,
    room_id: RoomId,
    position: Vec2,
    pivot: Pivot,
    collider: Collider,
    motion: KinematicMotion,
    contact_behavior: KinematicContactBehavior,
) -> Entity {
    let mut kinematic = Kinematic::default();
    kinematic.contact_behavior = contact_behavior;
    kinematic.motion = motion;
    kinematic.clear_runtime_state();

    ecs.create_entity()
        .with(Transform {
            position,
            pivot,
            ..Default::default()
        })
        .with(collider)
        .with(kinematic)
        .with_current_room(room_id)
        .finish()
}

fn spawn_aabb_player(ecs: &mut Ecs, room_id: RoomId, position: Vec2) -> Entity {
    spawn_aabb_player_with_grounded(ecs, room_id, position, true)
}

fn spawn_airborne_aabb_player(ecs: &mut Ecs, room_id: RoomId, position: Vec2) -> Entity {
    spawn_aabb_player_with_grounded(ecs, room_id, position, false)
}

fn spawn_aabb_player_with_grounded(
    ecs: &mut Ecs,
    room_id: RoomId,
    position: Vec2,
    grounded: bool,
) -> Entity {
    spawn_dynamic_body(
        ecs,
        room_id,
        position,
        Pivot::BottomCenter,
        Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            offset: Vec2::ZERO,
        },
        grounded,
    )
}

fn spawn_dynamic_body(
    ecs: &mut Ecs,
    room_id: RoomId,
    position: Vec2,
    pivot: Pivot,
    collider: Collider,
    grounded: bool,
) -> Entity {
    ecs.create_entity()
        .with(Transform {
            position,
            pivot,
            ..Default::default()
        })
        .with(Velocity::default())
        .with(collider)
        .with(PhysicsBody)
        .with(Grounded(grounded))
        .with(SubPixel::default())
        .with_current_room(room_id)
        .with(Active::default())
        .finish()
}

fn spawn_solid_aabb(
    ecs: &mut Ecs,
    room_id: RoomId,
    position: Vec2,
    width: f32,
    height: f32,
) {
    ecs.create_entity()
        .with(Transform {
            position,
            pivot: Pivot::BottomCenter,
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Aabb { width, height },
            offset: Vec2::ZERO,
        })
        .with(Solid(true))
        .with_current_room(room_id)
        .finish();
}

fn entity_position(ecs: &Ecs, entity: Entity) -> Vec2 {
    ecs.get::<Transform>(entity).unwrap().position
}

fn entity_sub_pixel(ecs: &Ecs, entity: Entity) -> (f32, f32) {
    let sub_pixel = ecs.get::<SubPixel>(entity).unwrap();
    (sub_pixel.x, sub_pixel.y)
}

fn overlaps(ecs: &Ecs, a: Entity, b: Entity) -> bool {
    let a_transform = ecs.get::<Transform>(a).unwrap();
    let a_sub_pixel = ecs.get::<SubPixel>(a).copied().unwrap_or_default();
    let a_collider = ecs.get::<Collider>(a).copied().unwrap_or_default();

    let b_transform = ecs.get::<Transform>(b).unwrap();
    let b_sub_pixel = ecs.get::<SubPixel>(b).copied().unwrap_or_default();
    let b_collider = ecs.get::<Collider>(b).copied().unwrap_or_default();

    shapes_overlap(
        true_position(a_transform.position, a_sub_pixel),
        a_collider,
        a_transform.pivot,
        true_position(b_transform.position, b_sub_pixel),
        b_collider,
        b_transform.pivot,
    )
}

#[test]
fn kinematic_body_vertical_velocity_unchanged_when_gravity_does_not_apply() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let platform = spawn_kinematic_body(
        &mut ecs,
        RoomId(1),
        Vec2::new(32.0, FLOOR_Y - 16.0),
        vertical_constant(-60.0),
    );

    update_physics(&mut ecs, &world, DT);

    assert!((ecs.get::<Velocity>(platform).unwrap().y + 60.0).abs() < EPSILON);
}

#[test]
fn kinematic_body_moves_by_velocity_with_sub_pixel_accumulation() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let platform = spawn_kinematic_body(
        &mut ecs,
        RoomId(1),
        Vec2::new(32.0, FLOOR_Y - 16.0),
        horizontal_constant(30.0),
    );

    update_physics(&mut ecs, &world, DT);

    assert_eq!(entity_position(&ecs, platform), Vec2::new(33.0, FLOOR_Y - 16.0));
    assert_eq!(entity_sub_pixel(&ecs, platform), (-0.5, 0.0));
}

#[test]
fn kinematic_body_blocked_by_wall_zeroes_blocked_axis_velocity() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    spawn_solid_aabb(&mut ecs, room_id, Vec2::new(92.0, FLOOR_Y - 16.0), 8.0, 16.0);
    let platform = spawn_kinematic_body(
        &mut ecs,
        room_id,
        Vec2::new(60.0, FLOOR_Y - 16.0),
        horizontal_constant(600.0),
    );

    update_physics(&mut ecs, &world, DT);

    assert_eq!(ecs.get::<Velocity>(platform).unwrap().x, 0.0);
}

#[test]
fn dynamic_standing_on_horizontal_platform_moves_with_it() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let platform = spawn_kinematic_body(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 16.0),
        horizontal_constant(120.0),
    );
    let rider = spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 16.0 - PLATFORM_HEIGHT),
    );

    update_physics(&mut ecs, &world, DT);

    assert_eq!(entity_position(&ecs, platform), Vec2::new(42.0, FLOOR_Y - 16.0));
    assert_eq!(entity_position(&ecs, rider), Vec2::new(42.0, FLOOR_Y - 16.0 - PLATFORM_HEIGHT));
    assert!(ecs.get::<Grounded>(rider).unwrap().0);
}

#[test]
fn dynamic_standing_on_rising_platform_is_lifted() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    spawn_kinematic_body(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 8.0),
        vertical_constant(-60.0),
    );
    let rider = spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 8.0 - PLATFORM_HEIGHT),
    );

    update_physics(&mut ecs, &world, DT);

    assert_eq!(entity_position(&ecs, rider), Vec2::new(40.0, FLOOR_Y - 8.0 - PLATFORM_HEIGHT - 1.0));
    assert!(ecs.get::<Grounded>(rider).unwrap().0);
}

#[test]
fn dynamic_standing_on_descending_platform_stays_grounded_when_support_keeps_up() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    spawn_kinematic_body(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 16.0),
        vertical_constant(60.0),
    );
    let rider = spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 16.0 - PLATFORM_HEIGHT),
    );

    update_physics(&mut ecs, &world, DT);

    assert!(ecs.get::<Grounded>(rider).unwrap().0);
    assert_eq!(ecs.get::<Velocity>(rider).unwrap().y, 0.0);
}

#[test]
fn two_dynamics_on_one_platform_are_both_carried() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    spawn_kinematic_body(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 16.0),
        horizontal_constant(120.0),
    );
    let left = spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(37.0, FLOOR_Y - 16.0 - PLATFORM_HEIGHT),
    );
    let right = spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(43.0, FLOOR_Y - 16.0 - PLATFORM_HEIGHT),
    );

    update_physics(&mut ecs, &world, DT);

    assert_eq!(entity_position(&ecs, left), Vec2::new(39.0, FLOOR_Y - 16.0 - PLATFORM_HEIGHT));
    assert_eq!(entity_position(&ecs, right), Vec2::new(45.0, FLOOR_Y - 16.0 - PLATFORM_HEIGHT));
}

#[test]
fn kinematic_bodies_ignore_each_other() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let left = spawn_kinematic_body(
        &mut ecs,
        room_id,
        Vec2::new(36.0, FLOOR_Y - 16.0),
        horizontal_constant(120.0),
    );
    let right = spawn_kinematic_body(
        &mut ecs,
        room_id,
        Vec2::new(44.0, FLOOR_Y - 16.0),
        horizontal_constant(-120.0),
    );

    update_physics(&mut ecs, &world, DT);

    assert_eq!(entity_position(&ecs, left), Vec2::new(38.0, FLOOR_Y - 16.0));
    assert_eq!(entity_position(&ecs, right), Vec2::new(42.0, FLOOR_Y - 16.0));
    assert!((ecs.get::<Velocity>(left).unwrap().x - 120.0).abs() < EPSILON);
    assert!((ecs.get::<Velocity>(right).unwrap().x + 120.0).abs() < EPSILON);
}

#[test]
fn dynamic_jumping_off_moving_platform_inherits_platform_velocity() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    spawn_kinematic_body(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 16.0),
        horizontal_constant(120.0),
    );
    let rider = spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 16.0 - PLATFORM_HEIGHT),
    );

    update_physics(&mut ecs, &world, DT);
    ecs.get_mut::<Velocity>(rider).unwrap().y = -JUMP_SPEED;
    update_physics(&mut ecs, &world, DT);

    assert!(ecs.get::<Velocity>(rider).unwrap().x > 0.0);
    assert!(ecs.get::<Velocity>(rider).unwrap().y < 0.0);
    assert!(!ecs.get::<Grounded>(rider).unwrap().0);
}

#[test]
fn dynamic_landing_on_moving_platform_sticks_and_inherits_velocity() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let platform = spawn_kinematic_body(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 16.0),
        horizontal_constant(60.0),
    );
    let rider = spawn_airborne_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 16.0 - PLATFORM_HEIGHT - 6.0),
    );

    for _ in 0..8 {
        update_physics(&mut ecs, &world, DT);
        if ecs.get::<Grounded>(rider).unwrap().0 {
            break;
        }
    }

    assert!(ecs.get::<Grounded>(rider).unwrap().0);
    let rider_x = entity_position(&ecs, rider).x;
    let platform_x = entity_position(&ecs, platform).x;
    update_physics(&mut ecs, &world, DT);
    assert!(ecs.get::<Grounded>(rider).unwrap().0);
    assert_eq!(entity_position(&ecs, rider).x - rider_x, entity_position(&ecs, platform).x - platform_x);
    assert_eq!(entity_position(&ecs, rider).y, entity_position(&ecs, platform).y - PLATFORM_HEIGHT);
}

#[test]
fn ping_pong_reverse_policy_reverses_on_wall_contact() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    spawn_solid_aabb(&mut ecs, room_id, Vec2::new(92.0, FLOOR_Y - 16.0), 8.0, 16.0);
    let platform = spawn_kinematic_body_with_behavior(
        &mut ecs,
        room_id,
        Vec2::new(60.0, FLOOR_Y - 16.0),
        horizontal_ping_pong(600.0, 128.0, KinematicDirection::Positive),
        KinematicContactBehavior::Reverse,
    );

    update_physics(&mut ecs, &world, DT);
    let blocked_x = entity_position(&ecs, platform).x;
    update_physics(&mut ecs, &world, DT);

    assert!(entity_position(&ecs, platform).x < blocked_x);
    assert!(ecs.get::<Velocity>(platform).unwrap().x < 0.0);
}

#[test]
fn dynamic_on_ping_pong_platform_stays_attached_through_reversal() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let platform = spawn_kinematic_body(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 16.0),
        horizontal_ping_pong(120.0, 2.0, KinematicDirection::Positive),
    );
    let rider = spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 16.0 - PLATFORM_HEIGHT),
    );

    update_physics(&mut ecs, &world, DT);
    let carried_x = entity_position(&ecs, rider).x;
    let platform_x = entity_position(&ecs, platform).x;
    update_physics(&mut ecs, &world, DT);

    assert!(ecs.get::<Grounded>(rider).unwrap().0);
    assert!(entity_position(&ecs, rider).x < carried_x);
    assert!(entity_position(&ecs, platform).x < platform_x);
}

#[test]
fn dynamic_loses_contact_when_platform_outpaces_gravity() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    spawn_kinematic_body(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 16.0),
        vertical_constant(240.0),
    );
    let rider = spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 16.0 - PLATFORM_HEIGHT),
    );

    update_physics(&mut ecs, &world, DT);

    assert!(!ecs.get::<Grounded>(rider).unwrap().0);
}

#[test]
fn dynamic_loses_contact_when_supporting_platform_despawns() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let platform = spawn_kinematic_body(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 16.0),
        horizontal_constant(120.0),
    );
    let rider = spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 16.0 - PLATFORM_HEIGHT),
    );

    update_physics(&mut ecs, &world, DT);
    ecs.clear_current_room(platform);
    update_physics(&mut ecs, &world, DT);

    assert!(!ecs.get::<Grounded>(rider).unwrap().0);
}

#[test]
fn side_collision_does_not_push_stop_kinematic_backwards() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let platform = spawn_kinematic_body_with_behavior(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 16.0),
        horizontal_constant(-120.0),
        KinematicContactBehavior::Stop,
    );
    let player = spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(16.0, FLOOR_Y - 16.0),
    );
    ecs.get_mut::<Velocity>(player).unwrap().x = 120.0;

    update_physics(&mut ecs, &world, DT);

    assert!(entity_position(&ecs, platform).x <= 40.0);
}

#[test]
fn side_collision_does_not_leave_player_inside_stop_kinematic() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let platform = spawn_kinematic_body_with_behavior(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 16.0),
        horizontal_constant(-120.0),
        KinematicContactBehavior::Stop,
    );
    let player = spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(16.0, FLOOR_Y - 16.0),
    );
    ecs.get_mut::<Velocity>(player).unwrap().x = 120.0;

    update_physics(&mut ecs, &world, DT);

    assert!(!overlaps(&ecs, player, platform));
}

#[test]
fn same_direction_side_contact_does_not_stop_kinematic() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let platform = spawn_kinematic_body_with_behavior(
        &mut ecs,
        room_id,
        Vec2::new(40.0, FLOOR_Y - 16.0),
        horizontal_constant(120.0),
        KinematicContactBehavior::Stop,
    );
    let player = spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(-8.0, FLOOR_Y - 16.0),
    );
    ecs.get_mut::<Velocity>(player).unwrap().x = 240.0;

    for _ in 0..12 {
        update_physics(&mut ecs, &world, DT);
    }

    assert!(entity_position(&ecs, platform).x > 40.0 + 12.0 * 2.0 - 0.5);
    assert!(ecs.get::<Velocity>(platform).unwrap().x > 0.0);
}

#[test]
fn trailing_same_direction_overlap_does_not_stop_kinematic() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let platform = spawn_kinematic_body_with_behavior(
        &mut ecs,
        room_id,
        Vec2::new(42.0, FLOOR_Y - 16.0),
        horizontal_constant(120.0),
        KinematicContactBehavior::Stop,
    );
    ecs.add_component_to_entity(platform, Velocity { x: 120.0, y: 0.0 });

    let player = spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(19.0, FLOOR_Y - 16.0),
    );
    ecs.get_mut::<Velocity>(player).unwrap().x = 240.0;

    let collider = ecs.get::<Collider>(platform).copied().unwrap();
    let room = world.get_room(room_id).unwrap();
    let collision_world = CollisionWorld::new(&ecs, room, &world);
    let mut events = PhysicsEvents::default();
    resolve_kinematic_contacts(
        &mut ecs,
        room_id,
        &[KinematicFrameMotion {
            entity: platform,
            start_position: Vec2::new(40.0, FLOOR_Y - 16.0),
            delta: Vec2::new(2.0, 0.0),
            collider,
            pivot: Pivot::BottomCenter,
            contact_behavior: KinematicContactBehavior::Stop,
        }],
        &collision_world,
        &mut events,
    );

    assert!(ecs.get::<Velocity>(platform).unwrap().x > 0.0);
}

#[test]
fn side_collision_does_not_push_stop_kinematic_through_room_bounds() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let platform = spawn_kinematic_body_with_behavior(
        &mut ecs,
        room_id,
        Vec2::new(20.0, FLOOR_Y - 16.0),
        horizontal_constant(120.0),
        KinematicContactBehavior::Stop,
    );
    let player = spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(44.0, FLOOR_Y - 16.0),
    );
    ecs.get_mut::<Velocity>(player).unwrap().x = -120.0;

    update_physics(&mut ecs, &world, DT);

    assert!(entity_position(&ecs, platform).x >= 20.0);
}

#[test]
fn kinematic_stop_policy_stops_when_dynamic_is_pinned_against_wall() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    spawn_solid_aabb(&mut ecs, room_id, Vec2::new(56.0, FLOOR_Y - 16.0), 8.0, 16.0);
    let platform = spawn_kinematic_body_with_behavior(
        &mut ecs,
        room_id,
        Vec2::new(34.0, FLOOR_Y - 16.0),
        horizontal_constant(720.0),
        KinematicContactBehavior::Stop,
    );
    spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(48.0, FLOOR_Y - 16.0),
    );

    update_physics(&mut ecs, &world, DT);

    assert_eq!(ecs.get::<Velocity>(platform).unwrap().x, 0.0);
}

#[test]
fn trigger_policy_emits_contact_without_pushing_dynamic() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let rider = spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(48.0, FLOOR_Y - 16.0),
    );
    let platform = spawn_kinematic_body_with_behavior(
        &mut ecs,
        room_id,
        Vec2::new(34.0, FLOOR_Y - 16.0),
        horizontal_constant(720.0),
        KinematicContactBehavior::Trigger,
    );

    let mut events = PhysicsEvents::default();
    update_physics_with_events(&mut ecs, &world, DT, &mut events);

    assert_eq!(entity_position(&ecs, rider).x, 48.0);
    assert!(events.drain().iter().any(|event| {
        matches!(
            event,
            PhysicsEvent::KinematicContact(KinematicContactEvent::Contact { kinematic, dynamic })
                if *kinematic == platform && *dynamic == rider
        )
    }));
}

#[test]
fn kinematic_crush_policy_pushes_dynamic_when_not_pinned() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let rider = spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(48.0, FLOOR_Y - 16.0),
    );
    let platform = spawn_kinematic_body_with_behavior(
        &mut ecs,
        room_id,
        Vec2::new(34.0, FLOOR_Y - 16.0),
        horizontal_constant(720.0),
        KinematicContactBehavior::Crush,
    );

    let mut events = PhysicsEvents::default();
    update_physics_with_events(&mut ecs, &world, DT, &mut events);

    assert!(entity_position(&ecs, rider).x > 48.0);
    assert!(!events.drain().iter().any(|event| {
        matches!(
            event,
            PhysicsEvent::KinematicContact(KinematicContactEvent::Crushed { kinematic, dynamic })
                if *kinematic == platform && *dynamic == rider
        )
    }));
}

#[test]
fn kinematic_crush_policy_does_not_shove_airborne_dynamic_on_vertical_contact() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let rider = spawn_dynamic_body(
        &mut ecs,
        room_id,
        Vec2::new(0.0, 7.0),
        Pivot::TopLeft,
        Collider {
            shape: ColliderShape::Aabb {
                width: 4.0,
                height: 4.0,
            },
            offset: Vec2::ZERO,
        },
        false,
    );
    let platform = spawn_kinematic_body_with_collider(
        &mut ecs,
        room_id,
        Vec2::ZERO,
        Pivot::TopLeft,
        Collider {
            shape: ColliderShape::Aabb {
                width: 8.0,
                height: 8.0,
            },
            offset: Vec2::ZERO,
        },
        KinematicMotion::default(),
        KinematicContactBehavior::Crush,
    );
    let room = world.get_room(room_id).unwrap();
    let collision_world = CollisionWorld::new(&ecs, room, &world);
    let rider_before = entity_position(&ecs, rider);
    let collider = ecs.get::<Collider>(platform).copied().unwrap();

    let mut events = PhysicsEvents::default();
    resolve_kinematic_contacts(
        &mut ecs,
        room_id,
        &[KinematicFrameMotion {
            entity: platform,
            start_position: Vec2::ZERO,
            delta: Vec2::new(0.0, 2.0),
            collider,
            pivot: Pivot::TopLeft,
            contact_behavior: KinematicContactBehavior::Crush,
        }],
        &collision_world,
        &mut events,
    );

    assert_eq!(entity_position(&ecs, rider), rider_before);
    assert!(events.drain().is_empty());
}

#[test]
fn kinematic_crush_policy_reports_crushed_dynamic() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    spawn_solid_aabb(&mut ecs, room_id, Vec2::new(56.0, FLOOR_Y - 16.0), 8.0, 16.0);
    let rider = spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(48.0, FLOOR_Y - 16.0),
    );
    let platform = spawn_kinematic_body_with_behavior(
        &mut ecs,
        room_id,
        Vec2::new(34.0, FLOOR_Y - 16.0),
        horizontal_constant(720.0),
        KinematicContactBehavior::Crush,
    );

    let mut events = PhysicsEvents::default();
    update_physics_with_events(&mut ecs, &world, DT, &mut events);

    assert!(events.drain().iter().any(|event| {
        matches!(
            event,
            PhysicsEvent::KinematicContact(KinematicContactEvent::Crushed { kinematic, dynamic })
                if *kinematic == platform && *dynamic == rider
        )
    }));
}

#[test]
fn kinematic_eject_policy_pushes_dynamic_sideways() {
    let world = world_with_bottom_border();
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let rider = spawn_aabb_player(
        &mut ecs,
        room_id,
        Vec2::new(48.0, FLOOR_Y - 16.0),
    );
    spawn_kinematic_body_with_behavior(
        &mut ecs,
        room_id,
        Vec2::new(34.0, FLOOR_Y - 16.0),
        horizontal_constant(720.0),
        KinematicContactBehavior::Eject,
    );

    update_physics(&mut ecs, &world, DT);

    assert!(entity_position(&ecs, rider).x > 48.0);
}

#[test]
fn kinematic_eject_policy_ignores_circle_aabb_overlap_without_shape_contact() {
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let rider = spawn_dynamic_body(
        &mut ecs,
        room_id,
        Vec2::new(15.0, 0.0),
        Pivot::TopLeft,
        Collider {
            shape: ColliderShape::Aabb {
                width: 4.0,
                height: 4.0,
            },
            offset: Vec2::ZERO,
        },
        false,
    );
    let platform = spawn_kinematic_body_with_collider(
        &mut ecs,
        room_id,
        Vec2::ZERO,
        Pivot::TopLeft,
        Collider {
            shape: ColliderShape::Circle { radius: 8.0 },
            offset: Vec2::ZERO,
        },
        KinematicMotion::default(),
        KinematicContactBehavior::Eject,
    );
    let collider = ecs.get::<Collider>(platform).copied().unwrap();
    let start = entity_position(&ecs, platform);
    let rider_before = entity_position(&ecs, rider);
    let world = world_with_bottom_border();
    let room = world.get_room(room_id).unwrap();
    let collision_world = CollisionWorld::new(&ecs, room, &world);

    let mut events = PhysicsEvents::default();
    resolve_kinematic_contacts(
        &mut ecs,
        room_id,
        &[KinematicFrameMotion {
            entity: platform,
            start_position: start,
            delta: Vec2::ZERO,
            collider,
            pivot: Pivot::TopLeft,
            contact_behavior: KinematicContactBehavior::Eject,
        }],
        &collision_world,
        &mut events,
    );

    assert_eq!(entity_position(&ecs, rider), rider_before);
}
