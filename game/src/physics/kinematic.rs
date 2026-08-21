use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::worlds::RoomId;

use crate::physics::collision_world::{shapes_overlap, CollisionWorld};
use crate::physics::events::{KinematicContactEvent, PhysicsEvents};
use crate::physics::physics_system::SUPPORT_SNAP_DISTANCE;
use crate::physics::shapes;

#[derive(Clone, Copy)]
pub(crate) struct KinematicFrameMotion {
    pub entity: Entity,
    pub start_position: Vec2,
    pub delta: Vec2,
    pub collider: Collider,
    pub pivot: Pivot,
    pub contact_behavior: KinematicContactBehavior,
}

#[derive(Clone, Copy)]
struct DynamicSnapshot {
    entity: Entity,
    transform_position: Vec2,
    sub_pixel: SubPixel,
    collider: Collider,
    pivot: Pivot,
    aabb: (Vec2, Vec2),
}

#[derive(Clone, Copy)]
struct KinematicContact {
    aabb_overlap: Vec2,
    dynamic_aabb: (Vec2, Vec2),
    kinematic_aabb: (Vec2, Vec2),
}

#[derive(Clone, Copy)]
struct ShapeState {
    position: Vec2,
    collider: Collider,
    pivot: Pivot,
}

impl DynamicSnapshot {
    fn from_ecs(ecs: &Ecs, entity: Entity) -> Option<Self> {
        let transform = ecs.get::<Transform>(entity).copied()?;
        let sub_pixel = ecs.get::<SubPixel>(entity).copied().unwrap_or_default();
        let collider = ecs.get::<Collider>(entity).copied().unwrap_or_default();
        let true_position = true_position(transform.position, sub_pixel);

        Some(Self {
            entity,
            transform_position: transform.position,
            sub_pixel,
            collider,
            pivot: transform.pivot,
            aabb: shapes::collider_aabb(true_position, collider, transform.pivot),
        })
    }

    fn true_position(&self) -> Vec2 {
        true_position(self.transform_position, self.sub_pixel)
    }

    fn update_quantized_state(&mut self, transform_position: Vec2, sub_pixel: SubPixel) {
        self.transform_position = transform_position;
        self.sub_pixel = sub_pixel;
        self.aabb = shapes::collider_aabb(
            true_position(transform_position, sub_pixel),
            self.collider,
            self.pivot,
        );
    }
}

pub(crate) fn update_kinematic_bodies(
    ecs: &mut Ecs,
    room_id: RoomId,
    collision_world: &CollisionWorld,
    dt: f32,
) -> Vec<KinematicFrameMotion> {
    let entities: Vec<_> = ecs
        .entities_in_room(room_id)
        .iter()
        .copied()
        .filter(|entity| ecs.has::<Kinematic>(*entity))
        .filter(|entity| ecs.get::<Active>(*entity).is_some_and(Active::is_enabled))
        .collect();

    let mut moved = Vec::with_capacity(entities.len());

    for entity in entities {
        let Some(transform) = ecs.get::<Transform>(entity).copied() else {
            continue;
        };
        let Some(mut kinematic) = ecs.get::<Kinematic>(entity).copied() else {
            continue;
        };
        let collider = ecs.get::<Collider>(entity).copied().unwrap_or_default();
        let sub_pixel = ecs.get::<SubPixel>(entity).copied().unwrap_or_default();
        let true_pos = true_position(transform.position, sub_pixel);
        let contact_behavior = authored_contact_behavior(kinematic);
        let delta = desired_kinematic_delta(&mut kinematic, true_pos, dt);
        let sweep = collision_world.sweep_move(
            entity,
            true_pos,
            delta,
            collider,
            transform.pivot,
        );
        apply_quantized_delta(ecs, entity, transform.position, sub_pixel, sweep.allowed_delta);

        if contact_behavior == KinematicContactBehavior::Reverse
            && motion_blocked_on_leading_face(sweep.blocked_x, sweep.blocked_y, delta)
        {
            kinematic.set_runtime_direction(kinematic.runtime_direction().reversed());
        }

        if let Some(velocity) = ecs.get_mut::<Velocity>(entity) {
            *velocity = resolved_velocity(sweep.allowed_delta, sweep.blocked_x, sweep.blocked_y, dt);
        }
        if let Some(existing) = ecs.get_mut::<Kinematic>(entity) {
            *existing = kinematic;
        }

        moved.push(KinematicFrameMotion {
            entity,
            start_position: true_pos,
            delta: sweep.allowed_delta,
            collider,
            pivot: transform.pivot,
            contact_behavior,
        });
    }

    moved
}

/// Returns the support carry applied to a grounded rider this frame.
pub(crate) fn carry_delta(motion: KinematicFrameMotion, downward_delta: f32) -> Vec2 {
    let carry_down = if motion.delta.y > 0.0 && downward_delta >= 0.0 {
        motion.delta.y.min(downward_delta + SUPPORT_SNAP_DISTANCE)
    } else {
        motion.delta.y.min(0.0)
    };

    Vec2::new(motion.delta.x, carry_down)
}

pub(crate) fn supporting_kinematic(
    position: Vec2,
    collider: Collider,
    pivot: Pivot,
    moved_kinematics: &[KinematicFrameMotion],
) -> Option<KinematicFrameMotion> {
    moved_kinematics
        .iter()
        .copied()
        .filter(|motion| motion.contact_behavior.is_solid())
        .find(|motion| supports_body(position, collider, pivot, *motion))
}

pub(crate) fn resolve_kinematic_contacts(
    ecs: &mut Ecs,
    room_id: RoomId,
    moved_kinematics: &[KinematicFrameMotion],
    collision_world: &CollisionWorld,
    events: &mut PhysicsEvents,
) {
    let mut dynamics = dynamic_snapshots(ecs, room_id);

    for motion in moved_kinematics {
        let crush_collision_world = (motion.contact_behavior == KinematicContactBehavior::Crush)
            .then(|| collision_world.excluding_entity(motion.entity));
        for dynamic in &mut dynamics {
            let Some(contact) = kinematic_contact(*dynamic, *motion) else {
                continue;
            };

            match motion.contact_behavior {
                KinematicContactBehavior::Stop => {
                    resolve_stop_contact(ecs, dynamic, *motion, contact)
                }
                KinematicContactBehavior::Crush => {
                    if resolve_crush_contact(
                        ecs,
                        dynamic,
                        *motion,
                        contact,
                        crush_collision_world.as_ref().unwrap(),
                    ) {
                        events.push_kinematic_contact(KinematicContactEvent::Crushed {
                            kinematic: motion.entity,
                            dynamic: dynamic.entity,
                        });
                    }
                }
                KinematicContactBehavior::Eject => eject_dynamic(ecs, dynamic, *motion),
                KinematicContactBehavior::Reverse => {
                    resolve_reverse_contact(ecs, dynamic, *motion, contact)
                }
                KinematicContactBehavior::Trigger => {
                    events.push_kinematic_contact(KinematicContactEvent::Contact {
                        kinematic: motion.entity,
                        dynamic: dynamic.entity,
                    });
                }
            }
        }
    }
}

fn authored_contact_behavior(kinematic: Kinematic) -> KinematicContactBehavior {
    if kinematic.contact_behavior.requires_ping_pong()
        && kinematic.motion.mode != KinematicMotionMode::PingPong
    {
        KinematicContactBehavior::Stop
    } else {
        kinematic.contact_behavior
    }
}

fn desired_kinematic_delta(kinematic: &mut Kinematic, position: Vec2, dt: f32) -> Vec2 {
    let motion = kinematic.motion;
    let speed = motion.speed.max(0.0);

    match motion.mode {
        KinematicMotionMode::None => Vec2::ZERO,
        KinematicMotionMode::Constant => axis_delta(motion.axis, motion.direction.sign() * speed * dt),
        KinematicMotionMode::PingPong => ping_pong_delta(kinematic, position, speed, dt),
    }
}

fn ping_pong_delta(
    kinematic: &mut Kinematic,
    position: Vec2,
    speed: f32,
    dt: f32,
) -> Vec2 {
    let travel_distance = kinematic.motion.travel_distance.max(0.0);
    if speed == 0.0 || travel_distance == 0.0 {
        return Vec2::ZERO;
    }

    if kinematic.runtime_origin().is_none() {
        kinematic.set_runtime_origin(position);
        kinematic.set_runtime_direction(kinematic.motion.direction);
    }
    let origin = kinematic.runtime_origin().unwrap_or(position);

    let axis = kinematic.motion.axis;
    let current = axis_value(axis, position);
    let start = axis_value(axis, origin);
    let (min_bound, max_bound) = match kinematic.motion.direction {
        KinematicDirection::Positive => (start, start + travel_distance),
        KinematicDirection::Negative => (start - travel_distance, start),
    };

    if kinematic.runtime_direction() == KinematicDirection::Positive && current >= max_bound {
        kinematic.set_runtime_direction(KinematicDirection::Negative);
    } else if kinematic.runtime_direction() == KinematicDirection::Negative && current <= min_bound {
        kinematic.set_runtime_direction(KinematicDirection::Positive);
    }

    let target = match kinematic.runtime_direction() {
        KinematicDirection::Positive => max_bound,
        KinematicDirection::Negative => min_bound,
    };
    let remaining = target - current;
    let desired = kinematic.runtime_direction().sign() * speed * dt;
    let clamped = if desired.abs() > remaining.abs() {
        remaining
    } else {
        desired
    };

    if (clamped - remaining).abs() <= f32::EPSILON {
        kinematic.set_runtime_direction(kinematic.runtime_direction().reversed());
    }

    axis_delta(axis, clamped)
}

fn motion_blocked_on_leading_face(blocked_x: bool, blocked_y: bool, delta: Vec2) -> bool {
    (delta.x != 0.0 && blocked_x) || (delta.y != 0.0 && blocked_y)
}

fn resolved_velocity(delta: Vec2, blocked_x: bool, blocked_y: bool, dt: f32) -> Velocity {
    if dt == 0.0 {
        return Velocity::default();
    }

    Velocity {
        x: if blocked_x { 0.0 } else { delta.x / dt },
        y: if blocked_y { 0.0 } else { delta.y / dt },
    }
}

fn axis_delta(axis: KinematicAxis, amount: f32) -> Vec2 {
    match axis {
        KinematicAxis::Horizontal => Vec2::new(amount, 0.0),
        KinematicAxis::Vertical => Vec2::new(0.0, amount),
    }
}

fn axis_value(axis: KinematicAxis, position: Vec2) -> f32 {
    match axis {
        KinematicAxis::Horizontal => position.x,
        KinematicAxis::Vertical => position.y,
    }
}

fn supports_body(
    position: Vec2,
    collider: Collider,
    pivot: Pivot,
    motion: KinematicFrameMotion,
) -> bool {
    let body_aabb = shapes::collider_aabb(position, collider, pivot);
    let platform_aabb = shapes::collider_aabb(motion.start_position, motion.collider, motion.pivot);
    let horizontal_overlap = body_aabb.0.x < platform_aabb.1.x && body_aabb.1.x > platform_aabb.0.x;
    let vertical_gap = platform_aabb.0.y - body_aabb.1.y;

    horizontal_overlap && vertical_gap.abs() <= SUPPORT_SNAP_DISTANCE
}

fn dynamic_snapshots(ecs: &Ecs, room_id: RoomId) -> Vec<DynamicSnapshot> {
    ecs.entities_in_room(room_id)
        .iter()
        .copied()
        .filter(|entity| ecs.has::<PhysicsBody>(*entity))
        .filter(|entity| ecs.get::<Active>(*entity).is_some_and(Active::is_enabled))
        .filter_map(|entity| DynamicSnapshot::from_ecs(ecs, entity))
        .collect()
}

fn kinematic_contact(dynamic: DynamicSnapshot, motion: KinematicFrameMotion) -> Option<KinematicContact> {
    build_contact(dynamic, motion.start_position + motion.delta, motion.collider, motion.pivot)
}

fn current_kinematic_contact(
    ecs: &Ecs,
    dynamic: DynamicSnapshot,
    motion: KinematicFrameMotion,
) -> Option<KinematicContact> {
    let kinematic = current_kinematic_state(ecs, motion)?;
    build_contact(dynamic, kinematic.position, kinematic.collider, kinematic.pivot)
}

fn build_contact(
    dynamic: DynamicSnapshot,
    kinematic_position: Vec2,
    kinematic_collider: Collider,
    kinematic_pivot: Pivot,
) -> Option<KinematicContact> {
    let dynamic_aabb = dynamic.aabb;
    let kinematic_aabb = shapes::collider_aabb(kinematic_position, kinematic_collider, kinematic_pivot);
    let aabb_overlap = shapes::aabb_overlap(dynamic_aabb, kinematic_aabb)?;
    if !shapes_overlap(
        kinematic_position,
        kinematic_collider,
        kinematic_pivot,
        dynamic.true_position(),
        dynamic.collider,
        dynamic.pivot,
    ) {
        return None;
    }

    Some(KinematicContact {
        aabb_overlap,
        dynamic_aabb,
        kinematic_aabb,
    })
}

fn current_kinematic_state(ecs: &Ecs, motion: KinematicFrameMotion) -> Option<ShapeState> {
    let transform = ecs.get::<Transform>(motion.entity).copied()?;
    let sub_pixel = ecs.get::<SubPixel>(motion.entity).copied().unwrap_or_default();
    Some(ShapeState {
        position: true_position(transform.position, sub_pixel),
        collider: motion.collider,
        pivot: transform.pivot,
    })
}

fn contact_is_on_leading_face(contact: KinematicContact, motion: KinematicFrameMotion) -> bool {
    match motion_axis(motion) {
        Some(KinematicAxis::Horizontal) if motion.delta.x > 0.0 => {
            contact.dynamic_aabb.0.x >= contact.kinematic_aabb.0.x
        }
        Some(KinematicAxis::Horizontal) if motion.delta.x < 0.0 => {
            contact.dynamic_aabb.1.x <= contact.kinematic_aabb.1.x
        }
        Some(KinematicAxis::Vertical) if motion.delta.y > 0.0 => {
            contact.dynamic_aabb.0.y >= contact.kinematic_aabb.0.y
        }
        Some(KinematicAxis::Vertical) if motion.delta.y < 0.0 => {
            contact.dynamic_aabb.1.y <= contact.kinematic_aabb.1.y
        }
        _ => false,
    }
}

fn resolve_stop_contact(
    ecs: &mut Ecs,
    dynamic: &mut DynamicSnapshot,
    motion: KinematicFrameMotion,
    contact: KinematicContact,
) {
    if contact_is_on_leading_face(contact, motion) {
        stop_kinematic(ecs, dynamic, motion, contact);
    }

    resolve_remaining_stop_overlap(ecs, dynamic, motion);
}

fn resolve_reverse_contact(
    ecs: &mut Ecs,
    dynamic: &mut DynamicSnapshot,
    motion: KinematicFrameMotion,
    contact: KinematicContact,
) {
    if !contact_is_on_leading_face(contact, motion) {
        return;
    }

    stop_kinematic(ecs, dynamic, motion, contact);
    if let Some(kinematic) = ecs.get_mut::<Kinematic>(motion.entity) {
        kinematic.set_runtime_direction(kinematic.runtime_direction().reversed());
    }
    resolve_remaining_stop_overlap(ecs, dynamic, motion);
}

fn stop_kinematic(
    ecs: &mut Ecs,
    dynamic: &DynamicSnapshot,
    motion: KinematicFrameMotion,
    contact: KinematicContact,
) {
    let Some(axis) = motion_axis(motion) else {
        return;
    };
    let Some(transform) = ecs.get::<Transform>(motion.entity).copied() else {
        return;
    };
    let sub_pixel = ecs.get::<SubPixel>(motion.entity).copied().unwrap_or_default();
    let kinematic_position = true_position(transform.position, sub_pixel);
    let max_distance = axis_overlap(contact.aabb_overlap, axis).min(axis_value(axis, motion.delta).abs());
    let direction = if axis_value(axis, motion.delta) > 0.0 { -1.0 } else { 1.0 };
    let amount = axis_separation_distance(
        ShapeState {
            position: kinematic_position,
            collider: motion.collider,
            pivot: transform.pivot,
        },
        ShapeState {
            position: dynamic.true_position(),
            collider: dynamic.collider,
            pivot: dynamic.pivot,
        },
        axis,
        direction,
        max_distance,
    );
    if amount <= shapes::OVERLAP_EPS {
        return;
    }

    apply_quantized_delta(
        ecs,
        motion.entity,
        transform.position,
        sub_pixel,
        axis_delta(axis, direction * amount),
    );

    if let Some(velocity) = ecs.get_mut::<Velocity>(motion.entity) {
        if motion.delta.x != 0.0 {
            velocity.x = 0.0;
        }
        if motion.delta.y != 0.0 {
            velocity.y = 0.0;
        }
    }
}

fn resolve_remaining_stop_overlap(
    ecs: &mut Ecs,
    dynamic: &mut DynamicSnapshot,
    motion: KinematicFrameMotion,
) {
    let Some(axis) = motion_axis(motion) else {
        return;
    };
    let Some(contact) = current_kinematic_contact(ecs, *dynamic, motion) else {
        return;
    };
    let Some(kinematic) = current_kinematic_state(ecs, motion) else {
        return;
    };
    let direction = separation_direction(contact.dynamic_aabb, contact.kinematic_aabb, axis);
    let amount = axis_separation_distance(
        ShapeState {
            position: dynamic.true_position(),
            collider: dynamic.collider,
            pivot: dynamic.pivot,
        },
        kinematic,
        axis,
        direction,
        axis_overlap(contact.aabb_overlap, axis),
    );
    if amount <= shapes::OVERLAP_EPS {
        return;
    }

    apply_dynamic_correction(
        ecs,
        dynamic,
        axis_delta(axis, direction * amount),
        axis == KinematicAxis::Horizontal,
        axis == KinematicAxis::Vertical,
    );
}

fn resolve_crush_contact(
    ecs: &mut Ecs,
    dynamic: &mut DynamicSnapshot,
    motion: KinematicFrameMotion,
    contact: KinematicContact,
    collision_world: &CollisionWorld,
) -> bool {
    let Some(axis) = motion_axis(motion) else {
        return false;
    };
    if !contact_is_on_leading_face(contact, motion) {
        return false;
    }
    let Some(kinematic) = current_kinematic_state(ecs, motion) else {
        return false;
    };

    let direction = motion_axis_direction(motion, axis);
    let amount = axis_separation_distance(
        ShapeState {
            position: dynamic.true_position(),
            collider: dynamic.collider,
            pivot: dynamic.pivot,
        },
        kinematic,
        axis,
        direction,
        directed_clearance(contact.dynamic_aabb, contact.kinematic_aabb, axis, direction),
    );

    if amount <= shapes::OVERLAP_EPS {
        return false;
    }

    let desired_delta = axis_delta(axis, direction * amount);
    let sweep = collision_world.sweep_move(
        dynamic.entity,
        dynamic.true_position(),
        desired_delta,
        dynamic.collider,
        dynamic.pivot,
    );

    if axis == KinematicAxis::Vertical
        && !ecs.get::<Grounded>(dynamic.entity).is_some_and(|grounded| grounded.0)
    {
        return (sweep.allowed_delta - desired_delta).length_squared() > shapes::OVERLAP_EPS.powi(2);
    }

    if sweep.allowed_delta != Vec2::ZERO {
        apply_dynamic_correction(
            ecs,
            dynamic,
            sweep.allowed_delta,
            axis == KinematicAxis::Horizontal,
            axis == KinematicAxis::Vertical,
        );
    }

    current_kinematic_contact(ecs, *dynamic, motion).is_some()
}

fn eject_dynamic(
    ecs: &mut Ecs,
    dynamic: &mut DynamicSnapshot,
    motion: KinematicFrameMotion,
) {
    let Some(contact) = current_kinematic_contact(ecs, *dynamic, motion) else {
        return;
    };
    let Some(kinematic) = current_kinematic_state(ecs, motion) else {
        return;
    };
    let direction = separation_direction(
        contact.dynamic_aabb,
        contact.kinematic_aabb,
        KinematicAxis::Horizontal,
    );
    let amount = axis_separation_distance(
        ShapeState {
            position: dynamic.true_position(),
            collider: dynamic.collider,
            pivot: dynamic.pivot,
        },
        kinematic,
        KinematicAxis::Horizontal,
        direction,
        contact.aabb_overlap.x,
    );
    if amount <= shapes::OVERLAP_EPS {
        return;
    }

    apply_dynamic_correction(
        ecs,
        dynamic,
        Vec2::new(direction * amount, 0.0),
        true,
        false,
    );
}

fn motion_axis_direction(motion: KinematicFrameMotion, axis: KinematicAxis) -> f32 {
    if axis_value(axis, motion.delta) >= 0.0 {
        1.0
    } else {
        -1.0
    }
}

fn directed_clearance(
    dynamic_aabb: (Vec2, Vec2),
    kinematic_aabb: (Vec2, Vec2),
    axis: KinematicAxis,
    direction: f32,
) -> f32 {
    match axis {
        KinematicAxis::Horizontal if direction > 0.0 => kinematic_aabb.1.x - dynamic_aabb.0.x,
        KinematicAxis::Horizontal => dynamic_aabb.1.x - kinematic_aabb.0.x,
        KinematicAxis::Vertical if direction > 0.0 => kinematic_aabb.1.y - dynamic_aabb.0.y,
        KinematicAxis::Vertical => dynamic_aabb.1.y - kinematic_aabb.0.y,
    }
}

fn axis_overlap(overlap: Vec2, axis: KinematicAxis) -> f32 {
    match axis {
        KinematicAxis::Horizontal => overlap.x,
        KinematicAxis::Vertical => overlap.y,
    }
}

fn separation_direction(
    dynamic_aabb: (Vec2, Vec2),
    kinematic_aabb: (Vec2, Vec2),
    axis: KinematicAxis,
) -> f32 {
    let dynamic_center = axis_value(axis, shapes::aabb_center(dynamic_aabb));
    let kinematic_center = axis_value(axis, shapes::aabb_center(kinematic_aabb));

    if dynamic_center >= kinematic_center {
        1.0
    } else {
        -1.0
    }
}

fn axis_separation_distance(
    moving: ShapeState,
    obstacle: ShapeState,
    axis: KinematicAxis,
    direction: f32,
    max_distance: f32,
) -> f32 {
    if max_distance <= shapes::OVERLAP_EPS {
        return 0.0;
    }
    if !shapes_overlap(
        moving.position,
        moving.collider,
        moving.pivot,
        obstacle.position,
        obstacle.collider,
        obstacle.pivot,
    ) {
        return 0.0;
    }

    let max_delta = axis_delta(axis, direction * max_distance);
    if shapes_overlap(
        moving.position + max_delta,
        moving.collider,
        moving.pivot,
        obstacle.position,
        obstacle.collider,
        obstacle.pivot,
    ) {
        return max_distance;
    }

    let mut low = 0.0;
    let mut high = max_distance;
    for _ in 0..12 {
        let mid = (low + high) * 0.5;
        let delta = axis_delta(axis, direction * mid);
        if shapes_overlap(
            moving.position + delta,
            moving.collider,
            moving.pivot,
            obstacle.position,
            obstacle.collider,
            obstacle.pivot,
        ) {
            low = mid;
        } else {
            high = mid;
        }
    }

    high
}

fn motion_axis(motion: KinematicFrameMotion) -> Option<KinematicAxis> {
    if motion.delta.x != 0.0 {
        Some(KinematicAxis::Horizontal)
    } else if motion.delta.y != 0.0 {
        Some(KinematicAxis::Vertical)
    } else {
        None
    }
}

fn apply_dynamic_correction(
    ecs: &mut Ecs,
    dynamic: &mut DynamicSnapshot,
    correction: Vec2,
    zero_x: bool,
    zero_y: bool,
) {
    let (new_position, new_sub_pixel) =
        quantize_motion(dynamic.transform_position, dynamic.sub_pixel, correction);
    apply_quantized_state(ecs, dynamic.entity, new_position, new_sub_pixel);
    dynamic.update_quantized_state(new_position, new_sub_pixel);

    if let Some(velocity) = ecs.get_mut::<Velocity>(dynamic.entity) {
        if zero_x {
            velocity.x = 0.0;
        }
        if zero_y {
            velocity.y = 0.0;
        }
    }
}


