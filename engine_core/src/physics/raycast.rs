use crate::ecs::{Collider, ColliderShape, CurrentRoom, Ecs, Entity, Sensor, Transform};
use crate::physics::collider_data_for_entity;
use crate::worlds::{RoomId, RoomLayer};
use bishop::prelude::Vec2;
use std::collections::HashSet;

const EPSILON: f32 = 0.0001;

/// Input for a world-space collider raycast.
pub struct RaycastQuery {
    pub from: Vec2,
    pub to: Vec2,
    pub room_id: Option<RoomId>,
    pub layer: Option<RoomLayer>,
    pub ignore: HashSet<Entity>,
    pub include_sensors: bool,
}

/// One entity hit by a world-space collider raycast.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaycastHit {
    pub entity: Entity,
    pub point: Vec2,
    pub normal: Vec2,
    pub distance: f32,
}

/// Returns the nearest collider hit for the supplied raycast query.
pub fn raycast(ecs: &Ecs, query: &RaycastQuery) -> Option<RaycastHit> {
    let segment = query.to - query.from;
    if segment.length_squared() <= EPSILON {
        return None;
    }

    let mut nearest: Option<RaycastHit> = None;
    for &entity in ecs.get_store::<Collider>().data.keys() {
        if query.ignore.contains(&entity) || !entity_matches_query(ecs, entity, query) {
            continue;
        }
        if !query.include_sensors && ecs.get::<Sensor>(entity).is_some() {
            continue;
        }

        let Some(transform) = ecs.get::<Transform>(entity) else {
            continue;
        };
        let Some(collider) = collider_data_for_entity(ecs, entity) else {
            continue;
        };
        let center = transform.position + collider.offset;
        let Some(hit) = hit_shape(entity, query.from, query.to, center, collider.shape) else {
            continue;
        };

        if nearest
            .as_ref()
            .is_none_or(|nearest| hit.distance < nearest.distance)
        {
            nearest = Some(hit);
        }
    }

    nearest
}

fn entity_matches_query(ecs: &Ecs, entity: Entity, query: &RaycastQuery) -> bool {
    if query.room_id.is_none() && query.layer.is_none() {
        return true;
    }

    let Some(current_room) = ecs.get::<CurrentRoom>(entity) else {
        return false;
    };

    query
        .room_id
        .is_none_or(|room_id| current_room.room_id == room_id)
        && query.layer.is_none_or(|layer| current_room.layer == layer)
}

fn hit_shape(
    entity: Entity,
    from: Vec2,
    to: Vec2,
    center: Vec2,
    shape: ColliderShape,
) -> Option<RaycastHit> {
    match shape {
        ColliderShape::Aabb { width, height } => {
            hit_aabb(entity, from, to, center, Vec2::new(width, height))
        }
        ColliderShape::Circle { radius } => hit_circle(entity, from, to, center, radius),
        ColliderShape::Capsule { radius, height } => hit_aabb(
            entity,
            from,
            to,
            center,
            Vec2::new(radius * 2.0, height + radius * 2.0),
        ),
        ColliderShape::Point => hit_point(entity, from, to, center),
    }
}

fn hit_aabb(entity: Entity, from: Vec2, to: Vec2, center: Vec2, size: Vec2) -> Option<RaycastHit> {
    let half = size / 2.0;
    let min = center - half;
    let max = center + half;
    let direction = to - from;
    let segment_length = direction.length();
    let mut clip = ClipState {
        t_min: 0.0,
        t_max: 1.0,
        normal: Vec2::ZERO,
    };

    if !clip_axis(
        AxisClip {
            origin: from.x,
            direction: direction.x,
            min: min.x,
            max: max.x,
            min_normal: Vec2::new(-1.0, 0.0),
            max_normal: Vec2::new(1.0, 0.0),
        },
        &mut clip,
    ) {
        return None;
    }
    if !clip_axis(
        AxisClip {
            origin: from.y,
            direction: direction.y,
            min: min.y,
            max: max.y,
            min_normal: Vec2::new(0.0, -1.0),
            max_normal: Vec2::new(0.0, 1.0),
        },
        &mut clip,
    ) {
        return None;
    }

    if !(0.0..=1.0).contains(&clip.t_min) {
        return None;
    }

    let point = from + direction * clip.t_min;
    Some(RaycastHit {
        entity,
        point,
        normal: clip.normal,
        distance: segment_length * clip.t_min,
    })
}

struct ClipState {
    t_min: f32,
    t_max: f32,
    normal: Vec2,
}

struct AxisClip {
    origin: f32,
    direction: f32,
    min: f32,
    max: f32,
    min_normal: Vec2,
    max_normal: Vec2,
}

fn clip_axis(axis: AxisClip, state: &mut ClipState) -> bool {
    if axis.direction.abs() <= EPSILON {
        return axis.origin >= axis.min && axis.origin <= axis.max;
    }

    let inverse = 1.0 / axis.direction;
    let mut enter = (axis.min - axis.origin) * inverse;
    let mut exit = (axis.max - axis.origin) * inverse;
    let mut enter_normal = axis.min_normal;

    if enter > exit {
        std::mem::swap(&mut enter, &mut exit);
        enter_normal = axis.max_normal;
    }

    if enter > state.t_min {
        state.t_min = enter;
        state.normal = enter_normal;
    }
    state.t_max = state.t_max.min(exit);

    state.t_min <= state.t_max
}

fn hit_circle(
    entity: Entity,
    from: Vec2,
    to: Vec2,
    center: Vec2,
    radius: f32,
) -> Option<RaycastHit> {
    let direction = to - from;
    let segment_length = direction.length();
    let relative = from - center;
    let a = direction.dot(direction);
    let b = 2.0 * relative.dot(direction);
    let c = relative.dot(relative) - radius * radius;
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }

    let sqrt = discriminant.sqrt();
    let first = (-b - sqrt) / (2.0 * a);
    let second = (-b + sqrt) / (2.0 * a);
    let t = [first, second]
        .into_iter()
        .find(|candidate| (0.0..=1.0).contains(candidate))?;
    let point = from + direction * t;
    let normal = normalized_or_zero(point - center);

    Some(RaycastHit {
        entity,
        point,
        normal,
        distance: segment_length * t,
    })
}

fn hit_point(entity: Entity, from: Vec2, to: Vec2, point: Vec2) -> Option<RaycastHit> {
    let direction = to - from;
    let segment_length_squared = direction.length_squared();
    if segment_length_squared <= EPSILON {
        return None;
    }

    let t = ((point - from).dot(direction) / segment_length_squared).clamp(0.0, 1.0);
    let projected = from + direction * t;
    if projected.distance(point) > EPSILON {
        return None;
    }

    Some(RaycastHit {
        entity,
        point,
        normal: Vec2::ZERO,
        distance: direction.length() * t,
    })
}

fn normalized_or_zero(value: Vec2) -> Vec2 {
    let length = value.length();
    if length <= EPSILON {
        Vec2::ZERO
    } else {
        value / length
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::Collider;

    fn aabb(width: f32, height: f32) -> Collider {
        Collider {
            shape: ColliderShape::Aabb { width, height },
            ..Default::default()
        }
    }

    fn query(from: Vec2, to: Vec2) -> RaycastQuery {
        RaycastQuery {
            from,
            to,
            room_id: Some(RoomId(1)),
            layer: Some(RoomLayer::Front),
            ignore: HashSet::new(),
            include_sensors: false,
        }
    }

    #[test]
    fn raycast_when_segment_crosses_aabb_returns_nearest_hit() {
        let mut ecs = Ecs::default();
        let far = ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(14.0, 0.0),
                ..Default::default()
            })
            .with(aabb(4.0, 4.0))
            .with_current_room(RoomId(1))
            .finish();
        let near = ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(8.0, 0.0),
                ..Default::default()
            })
            .with(aabb(4.0, 4.0))
            .with_current_room(RoomId(1))
            .finish();

        let hit = raycast(
            &ecs,
            &query(Vec2::new(0.0, 0.0), Vec2::new(20.0, 0.0)),
        )
        .unwrap();

        assert_eq!(hit.entity, near);
        assert_ne!(hit.entity, far);
        assert_eq!(hit.point, Vec2::new(6.0, 0.0));
        assert_eq!(hit.normal, Vec2::new(-1.0, 0.0));
        assert_eq!(hit.distance, 6.0);
    }

    #[test]
    fn raycast_when_no_collider_intersects_returns_no_hit() {
        let mut ecs = Ecs::default();
        ecs.create_entity()
            .with(Transform {
                position: Vec2::new(8.0, 8.0),
                ..Default::default()
            })
            .with(aabb(4.0, 4.0))
            .with_current_room(RoomId(1))
            .finish();

        let hit = raycast(
            &ecs,
            &query(Vec2::new(0.0, 0.0), Vec2::new(20.0, 0.0)),
        );

        assert!(hit.is_none());
    }

    #[test]
    fn raycast_when_entity_is_ignored_skips_that_entity() {
        let mut ecs = Ecs::default();
        let ignored = ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(8.0, 0.0),
                ..Default::default()
            })
            .with(aabb(4.0, 4.0))
            .with_current_room(RoomId(1))
            .finish();
        let visible = ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(14.0, 0.0),
                ..Default::default()
            })
            .with(aabb(4.0, 4.0))
            .with_current_room(RoomId(1))
            .finish();
        let mut ray = query(Vec2::new(0.0, 0.0), Vec2::new(20.0, 0.0));
        ray.ignore.insert(ignored);

        let hit = raycast(&ecs, &ray).unwrap();

        assert_eq!(hit.entity, visible);
    }

    #[test]
    fn raycast_when_collider_is_sensor_excludes_sensor_by_default() {
        let mut ecs = Ecs::default();
        let sensor = ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(8.0, 0.0),
                ..Default::default()
            })
            .with(aabb(4.0, 4.0))
            .with(Sensor)
            .with_current_room_layer(RoomId(1), RoomLayer::Front)
            .finish();
        let mut ray = query(Vec2::new(0.0, 0.0), Vec2::new(20.0, 0.0));

        assert!(raycast(&ecs, &ray).is_none());

        ray.include_sensors = true;
        let hit = raycast(&ecs, &ray).unwrap();

        assert_eq!(hit.entity, sensor);
    }
}
