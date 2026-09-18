use bishop::prelude::*;
use crate::physics::collision_world::CollisionWorld;
use crate::physics::collisions::{CollisionPair, collect_collision_pairs};
use crate::physics::kinematic::{
    KinematicFrameMotion,
    carry_delta,
    resolve_kinematic_contacts,
    supporting_kinematic,
    update_kinematic_bodies,
};
use crate::physics::runtime::PhysicsRuntime;
use crate::physics::sensors::{SensorPair, collect_sensor_pairs};
use engine_core::ecs::{
    Active,
    Collider,
    CurrentRoom,
    Ecs,
    Entity,
    GravityScale,
    Grounded,
    Kinematic,
    MotionBody,
    PhysicsBody,
    Pivot,
    Sensor,
    SubPixel,
    Transform,
    Velocity,
    apply_quantized_delta,
    quantize_motion,
    true_position,
    update_entity_position,
};
use engine_core::worlds::{entity_in_world, Room, RoomId, World};
use std::collections::{HashMap, HashSet};

pub(crate) const SUPPORT_SNAP_DISTANCE: f32 = 0.5;

#[derive(Clone, Copy)]
struct PhysicsStep {
    gravity: f32,
    dt: f32,
}

struct CurrentPhysicsPairs<'a> {
    sensors: &'a mut HashSet<SensorPair>,
    collisions: &'a mut HashSet<CollisionPair>,
}

/// Applies physics and records generic physics events for later consumers.
pub(crate) fn update_physics(
    ecs: &mut Ecs,
    world: &World,
    dt: f32,
    runtime: &mut PhysicsRuntime,
) {
    update_motion_bodies(ecs, world, dt);

    let bodies_by_room = active_physics_bodies_by_room(ecs);
    let active_room_ids = active_physics_room_ids(ecs, world, &bodies_by_room);
    let step = PhysicsStep {
        gravity: world.gravity * world.grid_size,
        dt,
    };
    let mut current_sensor_pairs = HashSet::new();
    let mut current_collision_pairs = HashSet::new();

    for room_id in active_room_ids {
        simulate_physics_room(
            ecs,
            world,
            room_id,
            bodies_by_room.get(&room_id).map(Vec::as_slice),
            step,
            runtime,
            CurrentPhysicsPairs {
                sensors: &mut current_sensor_pairs,
                collisions: &mut current_collision_pairs,
            },
        );
    }

    runtime.finish_sensor_frame(current_sensor_pairs);
    runtime.finish_collision_frame(current_collision_pairs);
}

fn active_physics_bodies_by_room(ecs: &Ecs) -> HashMap<RoomId, Vec<Entity>> {
    let mut bodies_by_room: HashMap<RoomId, Vec<Entity>> = HashMap::new();
    for &entity in ecs.get_store::<PhysicsBody>().data.keys() {
        if !ecs.get::<Active>(entity).is_some_and(Active::is_enabled) {
            continue;
        }
        if let Some(room) = ecs.get::<CurrentRoom>(entity) {
            bodies_by_room.entry(room.room_id).or_default().push(entity);
        }
    }
    bodies_by_room
}

fn active_physics_room_ids(
    ecs: &Ecs,
    world: &World,
    bodies_by_room: &HashMap<RoomId, Vec<Entity>>,
) -> HashSet<RoomId> {
    let mut active_room_ids = bodies_by_room.keys().copied().collect::<HashSet<_>>();
    for room in world.rooms() {
        if room_has_active_kinematic(ecs, room) {
            active_room_ids.insert(room.id);
        }
    }
    active_room_ids
}

fn room_has_active_kinematic(ecs: &Ecs, room: &Room) -> bool {
    ecs.entities_in_room(room.id).iter().any(|entity| {
        ecs.has::<Kinematic>(*entity)
            && ecs.get::<Active>(*entity).is_some_and(Active::is_enabled)
    })
}

fn simulate_physics_room(
    ecs: &mut Ecs,
    world: &World,
    room_id: RoomId,
    room_entities: Option<&[Entity]>,
    step: PhysicsStep,
    runtime: &mut PhysicsRuntime,
    current_pairs: CurrentPhysicsPairs<'_>,
) {
    let Some(room) = world.get_room(room_id) else {
        return;
    };

    let collision_world = CollisionWorld::new(ecs, room, world);
    let moved_kinematics = update_kinematic_bodies(ecs, room_id, &collision_world, step.dt);
    let contact_kinematics = contact_kinematics(ecs, &moved_kinematics);
    let blocking_kinematics = blocking_kinematics(&contact_kinematics);
    let dynamic_collision_world = collision_world.clone().with_kinematics(&blocking_kinematics);

    if let Some(room_entities) = room_entities {
        move_dynamic_bodies(
            ecs,
            room_entities,
            step,
            &contact_kinematics,
            &dynamic_collision_world,
        );
    }

    resolve_kinematic_contacts(
        ecs,
        room_id,
        &contact_kinematics,
        &dynamic_collision_world,
        runtime.events_mut(),
    );
    let sensor_collision_world = collision_world.with_current_sensors(ecs);
    collect_sensor_pairs(ecs, room_id, &sensor_collision_world, current_pairs.sensors);
    collect_collision_pairs(ecs, room, current_pairs.collisions);
}

fn contact_kinematics(ecs: &Ecs, moved_kinematics: &[KinematicFrameMotion]) -> Vec<KinematicFrameMotion> {
    moved_kinematics
        .iter()
        .copied()
        .filter(|motion| !ecs.has::<Sensor>(motion.entity))
        .collect()
}

fn blocking_kinematics(contact_kinematics: &[KinematicFrameMotion]) -> Vec<KinematicFrameMotion> {
    contact_kinematics
        .iter()
        .copied()
        .filter(|motion| motion.contact_behavior.is_solid())
        .collect()
}

fn move_dynamic_bodies(
    ecs: &mut Ecs,
    room_entities: &[Entity],
    step: PhysicsStep,
    contact_kinematics: &[KinematicFrameMotion],
    dynamic_collision_world: &CollisionWorld,
) {
    for &entity in room_entities {
        move_dynamic_body(
            ecs,
            entity,
            step,
            contact_kinematics,
            dynamic_collision_world,
        );
    }
}

fn move_dynamic_body(
    ecs: &mut Ecs,
    entity: Entity,
    step: PhysicsStep,
    contact_kinematics: &[KinematicFrameMotion],
    dynamic_collision_world: &CollisionWorld,
) {
    let was_grounded = ecs.get::<Grounded>(entity).is_some_and(|grounded| grounded.0);
    let Some(transform) = ecs.get::<Transform>(entity).copied() else {
        return;
    };
    let Some(mut velocity) = ecs.get::<Velocity>(entity).copied() else {
        return;
    };
    let collider = ecs.get::<Collider>(entity).copied().unwrap_or_default();
    let mut sub_pixel = ecs.get::<SubPixel>(entity).copied().unwrap_or_default();
    let mut position = transform.position;

    let support = if was_grounded {
        supporting_kinematic(
            true_position(position, sub_pixel),
            collider,
            transform.pivot,
            contact_kinematics,
        )
    } else {
        None
    };

    if let Some(motion) = support {
        apply_carrier_velocity(ecs, motion.entity, &mut velocity);
    }

    let gravity_scale = ecs.get::<GravityScale>(entity).copied().unwrap_or_default().0;
    velocity.y += step.gravity * gravity_scale * step.dt;

    let delta = Vec2::new(velocity.x * step.dt, velocity.y * step.dt);
    if let Some(motion) = support {
        let carry = carry_delta(motion, delta.y.max(0.0));
        let (carried_position, carried_sub_pixel) = quantize_motion(position, sub_pixel, carry);
        position = carried_position;
        sub_pixel = carried_sub_pixel;
    }

    let sweep = dynamic_collision_world.sweep_move(
        entity,
        true_position(position, sub_pixel),
        delta,
        collider,
        transform.pivot,
    );
    let (new_position, mut new_sub_pixel) = quantize_motion(position, sub_pixel, sweep.allowed_delta);
    let was_falling = velocity.y >= 0.0;
    let blocked_y = sweep.blocked_y
        || (was_falling
            && was_grounded
            && is_supported_within_snap_distance(
                dynamic_collision_world,
                entity,
                true_position(new_position, new_sub_pixel),
                collider,
                transform.pivot,
            ));

    if sweep.blocked_x {
        velocity.x = 0.0;
        new_sub_pixel.x = 0.0;
    }
    if blocked_y {
        velocity.y = 0.0;
    }

    update_dynamic_body_state(
        ecs,
        entity,
        new_position,
        new_sub_pixel,
        velocity,
        blocked_y && was_falling,
    );
}

fn apply_carrier_velocity(ecs: &Ecs, carrier: Entity, velocity: &mut Velocity) {
    if velocity.y >= 0.0 {
        return;
    }
    if let Some(carrier_velocity) = ecs.get::<Velocity>(carrier).copied() {
        velocity.x += carrier_velocity.x;
        velocity.y += carrier_velocity.y;
    }
}

fn update_dynamic_body_state(
    ecs: &mut Ecs,
    entity: Entity,
    position: Vec2,
    sub_pixel: SubPixel,
    velocity: Velocity,
    grounded: bool,
) {
    update_entity_position(ecs, entity, position);
    if let Some(existing) = ecs.get_mut::<Velocity>(entity) {
        *existing = velocity;
    }
    if let Some(existing) = ecs.get_mut::<SubPixel>(entity) {
        *existing = sub_pixel;
    }
    if let Some(existing) = ecs.get_mut::<Grounded>(entity) {
        existing.0 = grounded;
    }
}

fn is_supported_within_snap_distance(
    collision_world: &CollisionWorld,
    entity: Entity,
    position: Vec2,
    collider: Collider,
    pivot: Pivot,
) -> bool {
    collision_world
        .sweep_move(
            entity,
            position,
            Vec2::new(0.0, SUPPORT_SNAP_DISTANCE),
            collider,
            pivot,
        )
        .blocked_y
}

fn update_motion_bodies(ecs: &mut Ecs, world: &World, dt: f32) {
    let entities: Vec<_> = ecs
        .get_store::<MotionBody>()
        .data
        .keys()
        .filter(|entity| !ecs.has::<PhysicsBody>(**entity) && !ecs.has::<Kinematic>(**entity))
        .filter(|entity| entity_in_world(ecs, world, **entity))
        .copied()
        .collect();

    for entity in entities {
        let Some(transform) = ecs.get::<Transform>(entity).copied() else {
            continue;
        };
        let Some(velocity) = ecs.get::<Velocity>(entity).copied() else {
            continue;
        };

        let sub_pixel = ecs.get::<SubPixel>(entity).copied().unwrap_or_default();
        let delta = Vec2::new(velocity.x * dt, velocity.y * dt);
        apply_quantized_delta(ecs, entity, transform.position, sub_pixel, delta);
    }
}

