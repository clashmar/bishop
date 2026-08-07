use bishop::prelude::*;

use crate::physics::shapes;

use super::sweep_math::{axis_blocked_by_normal, circle_cast_corner, ray_axis};
use super::super::{CollisionWorld, SweepData};

impl CollisionWorld {
    pub(super) fn sweep_circle_against_rect(
        center: Vec2,
        radius: f32,
        desired_delta: Vec2,
        obs_min: Vec2,
        obs_max: Vec2,
    ) -> SweepData {
        let mut t_x = 1.0f32;
        let mut t_y = 1.0f32;
        let mut blocked_x = false;
        let mut blocked_y = false;
        let mut push_x = 0.0f32;
        let mut push_y = 0.0f32;

        let expanded_min = obs_min - Vec2::splat(radius);
        let expanded_max = obs_max + Vec2::splat(radius);

        let (tx_min, tx_max) = ray_axis(
            center.x, desired_delta.x, expanded_min.x, expanded_max.x,
        );
        let (ty_min, ty_max) = ray_axis(
            center.y, desired_delta.y, expanded_min.y, expanded_max.y,
        );

        let t_enter = tx_min.max(ty_min);
        let t_exit = tx_max.min(ty_max);

        if t_enter > t_exit || t_exit < 0.0 || t_enter > 1.0 {
            return SweepData {
                t_x,
                t_y,
                push_x,
                push_y,
                blocked_x,
                blocked_y,
            };
        }

        if t_enter <= 0.0 {
            let closest = Vec2::new(
                center.x.clamp(obs_min.x, obs_max.x),
                center.y.clamp(obs_min.y, obs_max.y),
            );
            let diff = center - closest;
            let dist_sq = diff.length_squared();

            if dist_sq > radius * radius + shapes::OVERLAP_EPS {
                return SweepData {
                    t_x,
                    t_y,
                    push_x,
                    push_y,
                    blocked_x,
                    blocked_y,
                };
            }

            if dist_sq < shapes::OVERLAP_EPS {
                let dx_min = center.x - obs_min.x;
                let dx_max = obs_max.x - center.x;
                let dy_min = center.y - obs_min.y;
                let dy_max = obs_max.y - center.y;
                let min_dx = dx_min.min(dx_max);
                let min_dy = dy_min.min(dy_max);

                if min_dx < min_dy {
                    let depenetration_x = if dx_min < dx_max {
                        -(dx_min + radius)
                    } else {
                        dx_max + radius
                    };
                    push_x += depenetration_x;
                    if desired_delta.x * depenetration_x < -shapes::OVERLAP_EPS {
                        blocked_x = true;
                        t_x = 0.0;
                    }
                } else {
                    let depenetration_y = if dy_min < dy_max {
                        -(dy_min + radius)
                    } else {
                        dy_max + radius
                    };
                    push_y += depenetration_y;
                    if desired_delta.y * depenetration_y < -shapes::OVERLAP_EPS {
                        blocked_y = true;
                        t_y = 0.0;
                    }
                }
            } else {
                let dist = dist_sq.sqrt();
                let normal = diff / dist;

                if dist < radius - shapes::OVERLAP_EPS {
                    let penetration = radius - dist;
                    if normal.x.abs() > normal.y.abs() {
                        push_x += normal.x * penetration;
                    } else {
                        push_y += normal.y * penetration;
                    }
                }

                if desired_delta.x * normal.x < -shapes::OVERLAP_EPS {
                    blocked_x = true;
                    t_x = 0.0;
                }
                if desired_delta.y * normal.y < -shapes::OVERLAP_EPS {
                    blocked_y = true;
                    t_y = 0.0;
                }

                if !blocked_x && desired_delta.x != 0.0 {
                    let t_exit_x = if desired_delta.x > 0.0 {
                        (expanded_max.x - center.x) / desired_delta.x
                    } else {
                        (expanded_min.x - center.x) / desired_delta.x
                    };
                    if t_exit_x > 0.0 && t_exit_x < t_x {
                        t_x = t_exit_x;
                    }
                }
                if !blocked_y && desired_delta.y != 0.0 {
                    let t_exit_y = if desired_delta.y > 0.0 {
                        (expanded_max.y - center.y) / desired_delta.y
                    } else {
                        (expanded_min.y - center.y) / desired_delta.y
                    };
                    if t_exit_y > 0.0 && t_exit_y < t_y {
                        t_y = t_exit_y;
                    }
                }
            }

            return SweepData {
                t_x,
                t_y,
                push_x,
                push_y,
                blocked_x,
                blocked_y,
            };
        }

        let contact_center = center + desired_delta * t_enter;
        if !shapes::circle_touches_rect(contact_center, radius, obs_min, obs_max) {
            let corners = [
                obs_min,
                Vec2::new(obs_max.x, obs_min.y),
                Vec2::new(obs_min.x, obs_max.y),
                obs_max,
            ];
            for &corner in &corners {
                if let Some(t) = circle_cast_corner(center, desired_delta, radius, corner) {
                    let contact_center = center + desired_delta * t;
                    let normal = (contact_center - corner).normalize();
                    if t < t_x && desired_delta.x * normal.x < -shapes::OVERLAP_EPS {
                        t_x = t;
                        blocked_x = true;
                    }
                    if t < t_y && desired_delta.y * normal.y < -shapes::OVERLAP_EPS {
                        t_y = t;
                        blocked_y = true;
                    }
                }
            }
        } else {
            if t_enter < t_x && tx_min >= ty_min {
                t_x = t_enter;
                blocked_x = true;
            }
            if t_enter < t_y && ty_min >= tx_min {
                t_y = t_enter;
                blocked_y = true;
            }
        }

        SweepData {
            t_x,
            t_y,
            push_x,
            push_y,
            blocked_x,
            blocked_y,
        }
    }

    pub(super) fn circle_rect_contact_time(
        center: Vec2,
        radius: f32,
        desired_delta: Vec2,
        obs_min: Vec2,
        obs_max: Vec2,
    ) -> Option<f32> {
        let expanded_min = obs_min - Vec2::splat(radius);
        let expanded_max = obs_max + Vec2::splat(radius);

        let (tx_min, tx_max) = ray_axis(
            center.x, desired_delta.x, expanded_min.x, expanded_max.x,
        );
        let (ty_min, ty_max) = ray_axis(
            center.y, desired_delta.y, expanded_min.y, expanded_max.y,
        );

        let t_enter = tx_min.max(ty_min);
        let t_exit = tx_max.min(ty_max);
        if t_enter > t_exit || t_exit < 0.0 || t_enter > 1.0 {
            return None;
        }

        if t_enter <= 0.0 {
            let closest = Vec2::new(
                center.x.clamp(obs_min.x, obs_max.x),
                center.y.clamp(obs_min.y, obs_max.y),
            );
            let diff = center - closest;
            let dist_sq = diff.length_squared();
            return (dist_sq <= radius * radius + shapes::OVERLAP_EPS).then_some(0.0);
        }

        let contact_center = center + desired_delta * t_enter;
        if shapes::circle_touches_rect(contact_center, radius, obs_min, obs_max) {
            return Some(t_enter);
        }

        let corners = [
            obs_min,
            Vec2::new(obs_max.x, obs_min.y),
            Vec2::new(obs_min.x, obs_max.y),
            obs_max,
        ];
        let mut best_t = None;
        for &corner in &corners {
            if let Some(t) = circle_cast_corner(center, desired_delta, radius, corner) {
                if best_t.is_none_or(|best| t < best) {
                    best_t = Some(t);
                }
            }
        }
        best_t
    }

    pub(super) fn sweep_circle_against_circle(
        center: Vec2,
        radius: f32,
        desired_delta: Vec2,
        obs_center: Vec2,
        obs_radius: f32,
    ) -> SweepData {
        let mut t_x = 1.0f32;
        let mut t_y = 1.0f32;
        let mut blocked_x = false;
        let mut blocked_y = false;
        let mut push_x = 0.0f32;
        let mut push_y = 0.0f32;
        let combined_radius = radius + obs_radius;
        let diff = center - obs_center;
        let dist_sq = diff.length_squared();

        if dist_sq <= combined_radius * combined_radius + shapes::OVERLAP_EPS {
            if dist_sq < shapes::OVERLAP_EPS {
                if desired_delta.x.abs() > desired_delta.y.abs() {
                    push_x = if desired_delta.x >= 0.0 {
                        -combined_radius
                    } else {
                        combined_radius
                    };
                    blocked_x = true;
                    t_x = 0.0;
                } else {
                    push_y = if desired_delta.y >= 0.0 {
                        -combined_radius
                    } else {
                        combined_radius
                    };
                    blocked_y = true;
                    t_y = 0.0;
                }
            } else {
                let dist = dist_sq.sqrt();
                let normal = diff / dist;
                if dist < combined_radius - shapes::OVERLAP_EPS {
                    let penetration = combined_radius - dist;
                    if normal.x.abs() > normal.y.abs() {
                        push_x = normal.x * penetration;
                    } else {
                        push_y = normal.y * penetration;
                    }
                }
                if axis_blocked_by_normal(desired_delta, normal, 0) {
                    blocked_x = true;
                    t_x = 0.0;
                }
                if axis_blocked_by_normal(desired_delta, normal, 1) {
                    blocked_y = true;
                    t_y = 0.0;
                }
            }

            return SweepData {
                t_x,
                t_y,
                push_x,
                push_y,
                blocked_x,
                blocked_y,
            };
        }

        if let Some(t) = circle_cast_corner(center, desired_delta, combined_radius, obs_center) {
            let contact_center = center + desired_delta * t;
            let normal = (contact_center - obs_center).normalize();
            if axis_blocked_by_normal(desired_delta, normal, 0) {
                blocked_x = true;
                t_x = t;
            }
            if axis_blocked_by_normal(desired_delta, normal, 1) {
                blocked_y = true;
                t_y = t;
            }
        }

        SweepData {
            t_x,
            t_y,
            push_x,
            push_y,
            blocked_x,
            blocked_y,
        }
    }
}
