use bishop::prelude::*;

use crate::physics::shapes;

pub(super) fn ray_axis(start: f32, dir: f32, lo: f32, hi: f32) -> (f32, f32) {
    if dir > 0.0 {
        ((lo - start) / dir, (hi - start) / dir)
    } else if dir < 0.0 {
        ((hi - start) / dir, (lo - start) / dir)
    } else if start >= lo && start <= hi {
        (f32::NEG_INFINITY, f32::INFINITY)
    } else {
        (f32::INFINITY, f32::NEG_INFINITY)
    }
}

/// Solves for the earliest t in [0, 1] where a sweeping circle touches a point.
pub(super) fn circle_cast_corner(center: Vec2, delta: Vec2, radius: f32, corner: Vec2) -> Option<f32> {
    let to_corner = center - corner;
    let a = delta.length_squared();
    if a < shapes::OVERLAP_EPS {
        return None;
    }
    let b = 2.0 * delta.dot(to_corner);
    let c = to_corner.length_squared() - radius * radius;
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }
    let sqrt_d = discriminant.sqrt();
    // t1 is the entry time; t2 = (-b + sqrt_d) / (2a) is the exit
    let t1 = (-b - sqrt_d) / (2.0 * a);
    if (0.0..=1.0).contains(&t1) {
        Some(t1)
    } else {
        None
    }
}

pub(super) fn axis_blocked_by_normal(desired_delta: Vec2, normal: Vec2, axis: usize) -> bool {
    let (axis_delta, axis_normal, other_normal) = if axis == 0 {
        (desired_delta.x, normal.x, normal.y)
    } else {
        (desired_delta.y, normal.y, normal.x)
    };

    axis_delta * axis_normal < -shapes::OVERLAP_EPS
        && axis_normal.abs() + shapes::OVERLAP_EPS >= other_normal.abs()
}

pub(super) fn aabb_overlaps(c: Vec2, hw: f32, hh: f32, obs_min: Vec2, obs_max: Vec2) -> bool {
    c.x + hw >= obs_min.x - shapes::OVERLAP_EPS
        && c.x - hw <= obs_max.x + shapes::OVERLAP_EPS
        && c.y + hh >= obs_min.y - shapes::OVERLAP_EPS
        && c.y - hh <= obs_max.y + shapes::OVERLAP_EPS
}

pub(super) fn select_stronger_push(pushes: [f32; 3]) -> f32 {
    let mut selected = 0.0f32;
    for push in pushes {
        if push.abs() > selected.abs() {
            selected = push;
        }
    }
    selected
}

