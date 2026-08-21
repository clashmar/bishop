use bishop::prelude::*;
use crate::physics::collision_world::CollisionWorld;
use crate::physics::events::PhysicsEvents;
use crate::physics::kinematic::{
    carry_delta,
    resolve_kinematic_contacts,
    supporting_kinematic,
    update_kinematic_bodies,
};
use engine_core::ecs::*;
use engine_core::worlds::*;
use std::collections::{HashMap, HashSet};

pub(crate) const SUPPORT_SNAP_DISTANCE: f32 = 0.5;

/// Applies fixed-step movement to `MotionBody`s and full collision physics to `PhysicsBody`s.
pub(crate) fn update_physics(
    ecs: &mut Ecs,
    world: &World,
    dt: f32,
) {
    let mut events = PhysicsEvents::default();
    update_physics_with_events(ecs, world, dt, &mut events);
    let _ = events.drain();
}

/// Applies physics and records generic physics events for later consumers.
pub(crate) fn update_physics_with_events(
    ecs: &mut Ecs,
    world: &World,
    dt: f32,
    events: &mut PhysicsEvents,
) {
    update_motion_bodies(ecs, world, dt);

    let entities: Vec<_> = ecs
        .get_store::<PhysicsBody>()
        .data
        .keys()
        .filter(|entity| ecs.get::<Active>(**entity).is_some_and(Active::is_enabled))
        .copied()
        .collect();

    let mut entities_by_room: HashMap<RoomId, Vec<Entity>> = HashMap::new();
    for entity in &entities {
        if let Some(room) = ecs.get::<CurrentRoom>(*entity) {
            entities_by_room.entry(room.room_id).or_default().push(*entity);
        }
    }

    let mut active_room_ids = entities_by_room.keys().copied().collect::<HashSet<_>>();
    for room in world.rooms() {
        if ecs
            .entities_in_room(room.id)
            .iter()
            .any(|entity| {
                ecs.has::<Kinematic>(*entity)
                    && ecs.get::<Active>(*entity).is_some_and(Active::is_enabled)
            })
        {
            active_room_ids.insert(room.id);
        }
    }

    for room_id in active_room_ids {
        let Some(room) = world.get_room(room_id) else {
            continue;
        };
        let collision_world = CollisionWorld::new(ecs, room, world);
        let moved_kinematics = update_kinematic_bodies(ecs, room_id, &collision_world, dt);
        let blocking_kinematics = moved_kinematics
            .iter()
            .copied()
            .filter(|motion| motion.contact_behavior.is_solid())
            .collect::<Vec<_>>();
        let dynamic_collision_world = collision_world.with_kinematics(&blocking_kinematics);
        let gravity = world.gravity * world.grid_size;

        let Some(room_entities) = entities_by_room.get(&room_id) else {
            continue;
        };

        for &entity in room_entities {
            let was_grounded = ecs.get::<Grounded>(entity).is_some_and(|grounded| grounded.0);
            let (mut pos_cur, pivot, mut vel_cur, collider) = {
                let Some(transform) = ecs.get::<Transform>(entity).copied() else {
                    continue;
                };
                let Some(velocity) = ecs.get::<Velocity>(entity).copied() else {
                    continue;
                };
                let collider = ecs.get::<Collider>(entity).copied().unwrap_or_default();
                (transform.position, transform.pivot, velocity, collider)
            };

            let mut sub_pixel = ecs.get::<SubPixel>(entity).copied().unwrap_or_default();

            let support = if was_grounded {
                supporting_kinematic(
                    true_position(pos_cur, sub_pixel),
                    collider,
                    pivot,
                    &moved_kinematics,
                )
            } else {
                None
            };

            if let Some(motion) = support {
                let carrier_velocity = if vel_cur.y < 0.0 {
                    ecs.get::<Velocity>(motion.entity).copied()
                } else {
                    None
                };
                if let Some(carrier_velocity) = carrier_velocity {
                    vel_cur.x += carrier_velocity.x;
                    vel_cur.y += carrier_velocity.y;
                }
            }

            let gravity_scale = ecs.get::<GravityScale>(entity).copied().unwrap_or_default().0;
            vel_cur.y += gravity * gravity_scale * dt;

            let delta = Vec2::new(vel_cur.x * dt, vel_cur.y * dt);

            if let Some(motion) = support {
                let carry = carry_delta(motion, delta.y.max(0.0));
                let (carried_pos, carried_sub_pixel) = quantize_motion(pos_cur, sub_pixel, carry);
                pos_cur = carried_pos;
                sub_pixel = carried_sub_pixel;
            }

            let true_pos = true_position(pos_cur, sub_pixel);

            let sweep = dynamic_collision_world.sweep_move(
                entity,
                true_pos,
                delta,
                collider,
                pivot,
            );

            // Snap to integer positions, storing the fractional part for next frame
            let (new_int_pos, new_sub_pixel) =
                quantize_motion(pos_cur, sub_pixel, sweep.allowed_delta);
            sub_pixel = new_sub_pixel;

            let was_falling = vel_cur.y >= 0.0;
            let new_true_pos = true_position(new_int_pos, sub_pixel);
            let blocked_y = sweep.blocked_y
                || (was_falling
                    && was_grounded
                    && is_supported_within_snap_distance(
                        &dynamic_collision_world,
                        entity,
                        new_true_pos,
                        collider,
                        pivot,
                    ));

            if sweep.blocked_x {
                vel_cur.x = 0.0;
                sub_pixel.x = 0.0;
            }
            if blocked_y {
                vel_cur.y = 0.0;
            }

            update_entity_position(ecs, entity, new_int_pos);
            if let Some(velocity) = ecs.get_mut::<Velocity>(entity) {
                *velocity = vel_cur;
            }

            if let Some(sp) = ecs.get_mut::<SubPixel>(entity) {
                *sp = sub_pixel;
            }
            if let Some(grounded) = ecs.get_mut::<Grounded>(entity) {
                grounded.0 = blocked_y && was_falling;
            }
        }

        resolve_kinematic_contacts(ecs, room_id, &moved_kinematics, &dynamic_collision_world, events);
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

