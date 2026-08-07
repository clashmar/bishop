mod aabb_pair_sweeps;
mod circle_pair_sweeps;
mod sweep_math;

use bishop::prelude::*;
use engine_core::ecs::{ColliderShape, Entity};
use engine_core::worlds::{InteriorZoneId, RoomLayer};

use crate::physics::shapes;

use self::sweep_math::select_stronger_push;
use super::{CollisionWorld, SweepData};

impl CollisionWorld {
    /// 2D sweep for circle colliders.
    pub(super) fn sweep_circle(
        &self,
        center: Vec2,
        radius: f32,
        desired_delta: Vec2,
        moving_entity: Entity,
        moving_layer: RoomLayer,
        active_back_zone: Option<InteriorZoneId>,
    ) -> SweepData {
        let mut t_x = 1.0f32;
        let mut t_y = 1.0f32;
        let mut blocked_x = false;
        let mut blocked_y = false;
        let mut push_x = 0.0f32;
        let mut push_y = 0.0f32;

        for solid in &self.solids {
            if solid.entity == Some(moving_entity)
                || !self.solid_affects_layer(solid, moving_layer, active_back_zone)
            {
                continue;
            }

            let data = match solid.shape {
                ColliderShape::Circle { radius: obs_radius } => {
                    let obs_center = Vec2::new(
                        solid.shape_pos.x + obs_radius,
                        solid.shape_pos.y + obs_radius,
                    );
                    Self::sweep_circle_against_circle(
                        center,
                        radius,
                        desired_delta,
                        obs_center,
                        obs_radius,
                    )
                }
                _ => Self::sweep_circle_against_rect(
                    center,
                    radius,
                    desired_delta,
                    solid.aabb.0,
                    solid.aabb.1,
                ),
            };

            push_x += data.push_x;
            push_y += data.push_y;
            if data.blocked_x && data.t_x < t_x {
                t_x = data.t_x;
                blocked_x = true;
            }
            if data.blocked_y && data.t_y < t_y {
                t_y = data.t_y;
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

    /// 2D sweep for capsule colliders.
    pub(super) fn sweep_capsule(
        &self,
        center: Vec2,
        radius: f32,
        height: f32,
        desired_delta: Vec2,
        moving_entity: Entity,
        moving_layer: RoomLayer,
        active_back_zone: Option<InteriorZoneId>,
    ) -> SweepData {
        let half = height * 0.5;
        let top_center = Vec2::new(center.x, center.y - half);
        let bot_center = Vec2::new(center.x, center.y + half);

        let top = self.sweep_circle(
            top_center,
            radius,
            desired_delta,
            moving_entity,
            moving_layer,
            active_back_zone,
        );
        let bot = self.sweep_circle(
            bot_center,
            radius,
            desired_delta,
            moving_entity,
            moving_layer,
            active_back_zone,
        );
        let body = self.sweep_aabb_2d(
            center,
            radius,
            half,
            desired_delta,
            moving_entity,
            moving_layer,
            active_back_zone,
        );

        SweepData {
            t_x: top.t_x.min(bot.t_x).min(body.t_x),
            t_y: top.t_y.min(bot.t_y).min(body.t_y),
            push_x: select_stronger_push([top.push_x, bot.push_x, body.push_x]),
            push_y: select_stronger_push([top.push_y, bot.push_y, body.push_y]),
            blocked_x: top.blocked_x || bot.blocked_x || body.blocked_x,
            blocked_y: top.blocked_y || bot.blocked_y || body.blocked_y,
        }
    }

    /// 2D sweep for an AABB centred at `center` with half-extents `(hw, hh)`.
    pub(super) fn sweep_aabb_2d(
        &self,
        center: Vec2,
        hw: f32,
        hh: f32,
        desired_delta: Vec2,
        moving_entity: Entity,
        moving_layer: RoomLayer,
        active_back_zone: Option<InteriorZoneId>,
    ) -> SweepData {
        let mut t_x = 1.0f32;
        let mut t_y = 1.0f32;
        let mut blocked_x = false;
        let mut blocked_y = false;
        let mut push_x = 0.0f32;
        let mut push_y = 0.0f32;

        for solid in &self.solids {
            if solid.entity == Some(moving_entity)
                || !self.solid_affects_layer(solid, moving_layer, active_back_zone)
            {
                continue;
            }

            let data = match solid.shape {
                ColliderShape::Circle { radius: obs_radius } => {
                    let obs_center = Vec2::new(
                        solid.shape_pos.x + obs_radius,
                        solid.shape_pos.y + obs_radius,
                    );
                    Self::sweep_aabb_against_circle(
                        center,
                        hw,
                        hh,
                        desired_delta,
                        obs_center,
                        obs_radius,
                    )
                }
                _ => Self::sweep_aabb_against_rect(
                    center,
                    hw,
                    hh,
                    desired_delta,
                    solid.aabb.0,
                    solid.aabb.1,
                ),
            };

            push_x += data.push_x;
            push_y += data.push_y;
            if data.blocked_x && data.t_x < t_x {
                t_x = data.t_x;
                blocked_x = true;
            }
            if data.blocked_y && data.t_y < t_y {
                t_y = data.t_y;
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

    pub(super) fn resolve_axis(
        &self,
        shape: ColliderShape,
        shape_pos: Vec2,
        delta: f32,
        axis: usize,
        moving_entity: Entity,
        moving_layer: RoomLayer,
        active_back_zone: Option<InteriorZoneId>,
    ) -> (f32, bool) {
        if delta == 0.0 {
            return (0.0, false);
        }

        let mut allowed = delta;
        let mut blocked = false;

        for solid in &self.solids {
            if solid.entity == Some(moving_entity)
                || !self.solid_affects_layer(solid, moving_layer, active_back_zone)
            {
                continue;
            }
            if let Some(limit) =
                shapes::sweep_axis(shape, shape_pos, delta, axis, solid.aabb)
            {
                if (delta > 0.0 && limit < allowed) || (delta < 0.0 && limit > allowed) {
                    allowed = limit;
                    blocked = true;
                }
            }
        }

        (allowed, blocked)
    }
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
