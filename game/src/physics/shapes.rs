use bishop::prelude::*;
use engine_core::ecs::*;

/// Floating-point tolerance for overlap checks.
pub const OVERLAP_EPS: f32 = 0.0001;

/// Returns the world-space axis-aligned bounding box (min, max) for a collider
/// at a position with a pivot.
pub fn collider_aabb(position: Vec2, collider: Collider, pivot: Pivot) -> (Vec2, Vec2) {
    let (sw, sh) = collider.shape.size();
    let size = Vec2::new(sw, sh);
    let top_left = pivot_offset(position + collider.offset, size, pivot);
    (top_left, top_left + size)
}

/// Sweep a shape against a single rect obstacle along one axis.
/// Returns None if no collision, Some(max_allowed_delta) if blocked.
pub fn sweep_axis(
    shape: ColliderShape,
    shape_pos: Vec2,
    delta: f32,
    axis: usize,
    obstacle: (Vec2, Vec2),
) -> Option<f32> {
    if delta == 0.0 {
        return None;
    }

    let (sw, sh) = shape.size();
    let shape_size = Vec2::new(sw, sh);

    let (obs_min, obs_max) = obstacle;

    let (my_min, my_max) = if axis == 0 {
        (shape_pos.x, shape_pos.x + shape_size.x)
    } else {
        (shape_pos.y, shape_pos.y + shape_size.y)
    };

    let (obs_min_axis, obs_max_axis) = if axis == 0 {
        (obs_min.x, obs_max.x)
    } else {
        (obs_min.y, obs_max.y)
    };

    let overlap_other = if axis == 0 {
        !(shape_pos.y + shape_size.y <= obs_min.y + OVERLAP_EPS
            || shape_pos.y >= obs_max.y - OVERLAP_EPS)
    } else {
        !(shape_pos.x + shape_size.x <= obs_min.x + OVERLAP_EPS
            || shape_pos.x >= obs_max.x - OVERLAP_EPS)
    };

    if !overlap_other {
        return None;
    }

    match shape {
        ColliderShape::Aabb { .. } => {
            sweep_aabb_axis(my_min, my_max, delta, obs_min_axis, obs_max_axis)
        }
        ColliderShape::Circle { radius } => {
            let center = Vec2::new(shape_pos.x + radius, shape_pos.y + radius);
            sweep_circle_axis(center, radius, delta, axis, obs_min, obs_max)
        }
        ColliderShape::Capsule { .. } => {
            // Capsule projects to its AABB for axis-separated sweep
            sweep_aabb_axis(my_min, my_max, delta, obs_min_axis, obs_max_axis)
        }
        ColliderShape::Point => {
            let my_pos = if axis == 0 { shape_pos.x } else { shape_pos.y };
            sweep_point_axis(my_pos, delta, obs_min_axis, obs_max_axis)
        }
    }
}

fn sweep_circle_axis(
    center: Vec2,
    radius: f32,
    delta: f32,
    axis: usize,
    obs_min: Vec2,
    obs_max: Vec2,
) -> Option<f32> {
    let center_axis = if axis == 0 { center.x } else { center.y };
    let obs_min_axis = if axis == 0 { obs_min.x } else { obs_min.y };
    let obs_max_axis = if axis == 0 { obs_max.x } else { obs_max.y };

    if delta > 0.0 {
        let edge = center_axis + radius;
        if edge > obs_min_axis + OVERLAP_EPS || edge + delta <= obs_min_axis {
            return None;
        }
        let contact_center = if axis == 0 {
            Vec2::new(obs_min.x - radius, center.y)
        } else {
            Vec2::new(center.x, obs_min.y - radius)
        };
        if !circle_touches_rect(contact_center, radius, obs_min, obs_max) {
            return None;
        }
        Some(obs_min_axis - edge)
    } else {
        let edge = center_axis - radius;
        if edge < obs_max_axis - OVERLAP_EPS || edge + delta >= obs_max_axis {
            return None;
        }
        let contact_center = if axis == 0 {
            Vec2::new(obs_max.x + radius, center.y)
        } else {
            Vec2::new(center.x, obs_max.y + radius)
        };
        if !circle_touches_rect(contact_center, radius, obs_min, obs_max) {
            return None;
        }
        Some(obs_max_axis - edge)
    }
}

fn circle_touches_rect(c: Vec2, r: f32, obs_min: Vec2, obs_max: Vec2) -> bool {
    let closest = Vec2::new(
        c.x.clamp(obs_min.x, obs_max.x),
        c.y.clamp(obs_min.y, obs_max.y),
    );
    let dist_sq = (c.x - closest.x).powi(2) + (c.y - closest.y).powi(2);
    dist_sq <= r.powi(2) + OVERLAP_EPS
}

fn sweep_point_axis(
    pos: f32,
    delta: f32,
    obs_min: f32,
    obs_max: f32,
) -> Option<f32> {
    if delta > 0.0 {
        (pos <= obs_min + OVERLAP_EPS && pos + delta > obs_min)
            .then_some(obs_min - pos)
    } else {
        (pos >= obs_max - OVERLAP_EPS && pos + delta < obs_max)
            .then_some(obs_max - pos)
    }
}

fn sweep_aabb_axis(
    my_min: f32,
    my_max: f32,
    delta: f32,
    obs_min: f32,
    obs_max: f32,
) -> Option<f32> {
    if delta > 0.0 {
        (my_max <= obs_min + OVERLAP_EPS && my_max + delta > obs_min)
            .then_some(obs_min - my_max)
    } else {
        (my_min >= obs_max - OVERLAP_EPS && my_min + delta < obs_max)
            .then_some(obs_max - my_min)
    }
}