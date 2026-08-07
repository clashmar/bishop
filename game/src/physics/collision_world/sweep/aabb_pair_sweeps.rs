use bishop::prelude::*;

use crate::physics::shapes;

use super::sweep_math::{aabb_overlaps, axis_blocked_by_normal, ray_axis};
use super::super::{CollisionWorld, SweepData};

impl CollisionWorld {
    pub(super) fn sweep_aabb_against_rect(
        center: Vec2,
        hw: f32,
        hh: f32,
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

        let expanded_min = Vec2::new(obs_min.x - hw, obs_min.y - hh);
        let expanded_max = Vec2::new(obs_max.x + hw, obs_max.y + hh);

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
            if !aabb_overlaps(center, hw, hh, obs_min, obs_max) {
                return SweepData {
                    t_x,
                    t_y,
                    push_x,
                    push_y,
                    blocked_x,
                    blocked_y,
                };
            }

            let overlap_x = (center.x + hw).min(obs_max.x) - (center.x - hw).max(obs_min.x);
            let overlap_y = (center.y + hh).min(obs_max.y) - (center.y - hh).max(obs_min.y);
            let penetrating_x = overlap_x > 0.0;
            let penetrating_y = overlap_y > 0.0;

            if penetrating_x && penetrating_y {
                if overlap_x < overlap_y {
                    let push_left = (center.x + hw) - obs_min.x;
                    let push_right = obs_max.x - (center.x - hw);
                    push_x += if push_left < push_right {
                        -push_left
                    } else {
                        push_right
                    };
                    blocked_x = true;
                    t_x = 0.0;
                } else {
                    let push_up = (center.y + hh) - obs_min.y;
                    let push_down = obs_max.y - (center.y - hh);
                    push_y += if push_up < push_down {
                        -push_up
                    } else {
                        push_down
                    };
                    blocked_y = true;
                    t_y = 0.0;
                }
            }

            if !blocked_x && overlap_y > shapes::OVERLAP_EPS {
                let body_left = center.x - hw;
                let body_right = center.x + hw;
                let moving_into_right_face = desired_delta.x > 0.0
                    && center.x <= obs_min.x + shapes::OVERLAP_EPS
                    && body_right >= obs_min.x - shapes::OVERLAP_EPS
                    && body_right + desired_delta.x > obs_min.x;
                let moving_into_left_face = desired_delta.x < 0.0
                    && center.x >= obs_max.x - shapes::OVERLAP_EPS
                    && body_left <= obs_max.x + shapes::OVERLAP_EPS
                    && body_left + desired_delta.x < obs_max.x;
                if moving_into_right_face || moving_into_left_face {
                    blocked_x = true;
                    t_x = 0.0;
                }
            }
            if !blocked_y && overlap_x > shapes::OVERLAP_EPS {
                let body_top = center.y - hh;
                let body_bottom = center.y + hh;
                let moving_into_bottom_face = desired_delta.y > 0.0
                    && center.y <= obs_min.y + shapes::OVERLAP_EPS
                    && body_bottom >= obs_min.y - shapes::OVERLAP_EPS
                    && body_bottom + desired_delta.y > obs_min.y;
                let moving_into_top_face = desired_delta.y < 0.0
                    && center.y >= obs_max.y - shapes::OVERLAP_EPS
                    && body_top <= obs_max.y + shapes::OVERLAP_EPS
                    && body_top + desired_delta.y < obs_max.y;
                if moving_into_bottom_face || moving_into_top_face {
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

        let contact_center = center + desired_delta * t_enter;
        let overlaps_x = contact_center.x + hw > obs_min.x + shapes::OVERLAP_EPS
            && contact_center.x - hw < obs_max.x - shapes::OVERLAP_EPS;
        let overlaps_y = contact_center.y + hh > obs_min.y + shapes::OVERLAP_EPS
            && contact_center.y - hh < obs_max.y - shapes::OVERLAP_EPS;

        if t_enter < t_x && tx_min >= ty_min && overlaps_y {
            t_x = t_enter;
            blocked_x = true;
        }
        if t_enter < t_y && ty_min >= tx_min && overlaps_x {
            t_y = t_enter;
            blocked_y = true;
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

    pub(super) fn sweep_aabb_against_circle(
        center: Vec2,
        hw: f32,
        hh: f32,
        desired_delta: Vec2,
        obs_center: Vec2,
        obs_radius: f32,
    ) -> SweepData {
        let rect_min = Vec2::new(center.x - hw, center.y - hh);
        let rect_max = Vec2::new(center.x + hw, center.y + hh);
        let reverse = Self::sweep_circle_against_rect(
            obs_center,
            obs_radius,
            -desired_delta,
            rect_min,
            rect_max,
        );
        let Some(t) = Self::circle_rect_contact_time(
            obs_center,
            obs_radius,
            -desired_delta,
            rect_min,
            rect_max,
        ) else {
            return SweepData {
                t_x: 1.0,
                t_y: 1.0,
                push_x: -reverse.push_x,
                push_y: -reverse.push_y,
                blocked_x: false,
                blocked_y: false,
            };
        };

        let moved_center = center + desired_delta * t;
        let moved_min = Vec2::new(moved_center.x - hw, moved_center.y - hh);
        let moved_max = Vec2::new(moved_center.x + hw, moved_center.y + hh);
        let closest = Vec2::new(
            obs_center.x.clamp(moved_min.x, moved_max.x),
            obs_center.y.clamp(moved_min.y, moved_max.y),
        );
        let mut normal = closest - obs_center;
        if normal.length_squared() <= shapes::OVERLAP_EPS {
            let left = obs_center.x - moved_min.x;
            let right = moved_max.x - obs_center.x;
            let top = obs_center.y - moved_min.y;
            let bottom = moved_max.y - obs_center.y;
            let min_x = left.min(right);
            let min_y = top.min(bottom);
            normal = if min_x < min_y {
                if left < right { Vec2::new(-1.0, 0.0) } else { Vec2::new(1.0, 0.0) }
            } else if top < bottom {
                Vec2::new(0.0, -1.0)
            } else {
                Vec2::new(0.0, 1.0)
            };
        } else {
            normal = normal.normalize();
        }

        let blocked_x = if desired_delta.y.abs() <= shapes::OVERLAP_EPS {
            desired_delta.x.abs() > shapes::OVERLAP_EPS
                && (reverse.blocked_x
                    || (t > shapes::OVERLAP_EPS && normal.x.abs() >= normal.y.abs()))
        } else if desired_delta.x.abs() <= shapes::OVERLAP_EPS {
            false
        } else {
            axis_blocked_by_normal(desired_delta, normal, 0)
        };
        let blocked_y = if desired_delta.x.abs() <= shapes::OVERLAP_EPS {
            if desired_delta.y > shapes::OVERLAP_EPS {
                reverse.blocked_y || t > shapes::OVERLAP_EPS
            } else {
                desired_delta.y < -shapes::OVERLAP_EPS
                    && (reverse.blocked_y
                        || (t > shapes::OVERLAP_EPS && normal.y.abs() >= normal.x.abs()))
            }
        } else if desired_delta.y.abs() <= shapes::OVERLAP_EPS {
            false
        } else {
            axis_blocked_by_normal(desired_delta, normal, 1)
        };

        SweepData {
            t_x: if blocked_x { t } else { 1.0 },
            t_y: if blocked_y { t } else { 1.0 },
            push_x: -reverse.push_x,
            push_y: -reverse.push_y,
            blocked_x,
            blocked_y,
        }
    }
}
