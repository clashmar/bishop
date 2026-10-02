mod aabb_pair_sweeps;
mod circle_pair_sweeps;
mod sweep_math;

use bishop::prelude::*;
use engine_core::ecs::{ColliderShape, Entity};
use engine_core::worlds::{InteriorZoneId, RoomLayer};

use crate::physics::shapes;

use self::sweep_math::select_stronger_push;
use super::{CollisionWorld, SweepData};

/// Context shared by sweep helpers for one moving body.
#[derive(Clone, Copy)]
pub(super) struct SweepContext {
    moving_entity: Entity,
    moving_layer: RoomLayer,
    active_back_zone: Option<InteriorZoneId>,
}

impl SweepContext {
    /// Builds sweep context for one moving body.
    pub(super) fn new(
        moving_entity: Entity,
        moving_layer: RoomLayer,
        active_back_zone: Option<InteriorZoneId>,
    ) -> Self {
        Self {
            moving_entity,
            moving_layer,
            active_back_zone,
        }
    }
}

impl CollisionWorld {
    /// 2D sweep for circle colliders.
    pub(super) fn sweep_circle(
        &self,
        center: Vec2,
        radius: f32,
        desired_delta: Vec2,
        ctx: SweepContext,
    ) -> SweepData {
        let mut t_x = 1.0f32;
        let mut t_y = 1.0f32;
        let mut blocked_x = false;
        let mut blocked_y = false;
        let mut push_x = 0.0f32;
        let mut push_y = 0.0f32;

        for solid in &self.solids {
            if solid.entity == Some(ctx.moving_entity)
                || !self.solid_affects_layer(solid, ctx.moving_layer, ctx.active_back_zone)
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
        ctx: SweepContext,
    ) -> SweepData {
        let half = height * 0.5;
        let top_center = Vec2::new(center.x, center.y - half);
        let bot_center = Vec2::new(center.x, center.y + half);

        let top = self.sweep_circle(top_center, radius, desired_delta, ctx);
        let bot = self.sweep_circle(bot_center, radius, desired_delta, ctx);
        let body = self.sweep_aabb(center, radius, half, desired_delta, ctx);

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
    pub(super) fn sweep_aabb(
        &self,
        center: Vec2,
        hw: f32,
        hh: f32,
        desired_delta: Vec2,
        ctx: SweepContext,
    ) -> SweepData {
        let mut t_x = 1.0f32;
        let mut t_y = 1.0f32;
        let mut blocked_x = false;
        let mut blocked_y = false;
        let mut push_x = 0.0f32;
        let mut push_y = 0.0f32;

        for solid in &self.solids {
            if solid.entity == Some(ctx.moving_entity)
                || !self.solid_affects_layer(solid, ctx.moving_layer, ctx.active_back_zone)
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

    /// Resolves one-axis movement for non-shape-aware sweep fallback.
    pub(super) fn resolve_axis(
        &self,
        shape: ColliderShape,
        shape_pos: Vec2,
        delta: f32,
        axis: usize,
        ctx: SweepContext,
    ) -> (f32, bool) {
        if delta == 0.0 {
            return (0.0, false);
        }

        let mut allowed = delta;
        let mut blocked = false;

        for solid in &self.solids {
            if solid.entity == Some(ctx.moving_entity)
                || !self.solid_affects_layer(solid, ctx.moving_layer, ctx.active_back_zone)
            {
                continue;
            }
            if let Some(limit) = shapes::sweep_axis(shape, shape_pos, delta, axis, solid.aabb) {
                if (delta > 0.0 && limit < allowed) || (delta < 0.0 && limit > allowed) {
                    allowed = limit;
                    blocked = true;
                }
            }
        }

        (allowed, blocked)
    }
}

/// Returns whether two shapes overlap at their current positions.
pub(super) fn shapes_overlap(
    moving_shape: ColliderShape,
    moving_shape_pos: Vec2,
    obstacle_shape: ColliderShape,
    obstacle_shape_pos: Vec2,
    obstacle_aabb: (Vec2, Vec2),
) -> bool {
    has_overlap(pair_sweep_data(
        moving_shape,
        moving_shape_pos,
        Vec2::ZERO,
        obstacle_shape,
        obstacle_shape_pos,
        obstacle_aabb,
    ))
}

fn has_overlap(data: SweepData) -> bool {
    data.push_x.abs() > shapes::OVERLAP_EPS
        || data.push_y.abs() > shapes::OVERLAP_EPS
        || data.blocked_x
        || data.blocked_y
}

fn pair_sweep_data(
    moving_shape: ColliderShape,
    moving_shape_pos: Vec2,
    desired_delta: Vec2,
    obstacle_shape: ColliderShape,
    obstacle_shape_pos: Vec2,
    obstacle_aabb: (Vec2, Vec2),
) -> SweepData {
    match moving_shape {
        ColliderShape::Circle { radius } => {
            let center = Vec2::new(moving_shape_pos.x + radius, moving_shape_pos.y + radius);
            match obstacle_shape {
                ColliderShape::Circle { radius: obs_radius } => {
                    let obs_center = Vec2::new(
                        obstacle_shape_pos.x + obs_radius,
                        obstacle_shape_pos.y + obs_radius,
                    );
                    CollisionWorld::sweep_circle_against_circle(
                        center,
                        radius,
                        desired_delta,
                        obs_center,
                        obs_radius,
                    )
                }
                _ => CollisionWorld::sweep_circle_against_rect(
                    center,
                    radius,
                    desired_delta,
                    obstacle_aabb.0,
                    obstacle_aabb.1,
                ),
            }
        }
        ColliderShape::Capsule { radius, height } => {
            let half = height * 0.5;
            let center = Vec2::new(
                moving_shape_pos.x + radius,
                moving_shape_pos.y + radius + half,
            );
            let top = pair_sweep_data(
                ColliderShape::Circle { radius },
                Vec2::new(center.x - radius, center.y - half - radius),
                desired_delta,
                obstacle_shape,
                obstacle_shape_pos,
                obstacle_aabb,
            );
            let bottom = pair_sweep_data(
                ColliderShape::Circle { radius },
                Vec2::new(center.x - radius, center.y + half - radius),
                desired_delta,
                obstacle_shape,
                obstacle_shape_pos,
                obstacle_aabb,
            );
            let body = pair_sweep_data(
                ColliderShape::Aabb {
                    width: radius * 2.0,
                    height,
                },
                Vec2::new(center.x - radius, center.y - half),
                desired_delta,
                obstacle_shape,
                obstacle_shape_pos,
                obstacle_aabb,
            );

            SweepData {
                t_x: top.t_x.min(bottom.t_x).min(body.t_x),
                t_y: top.t_y.min(bottom.t_y).min(body.t_y),
                push_x: select_stronger_push([top.push_x, bottom.push_x, body.push_x]),
                push_y: select_stronger_push([top.push_y, bottom.push_y, body.push_y]),
                blocked_x: top.blocked_x || bottom.blocked_x || body.blocked_x,
                blocked_y: top.blocked_y || bottom.blocked_y || body.blocked_y,
            }
        }
        ColliderShape::Aabb { width, height } => {
            let center = Vec2::new(
                moving_shape_pos.x + width * 0.5,
                moving_shape_pos.y + height * 0.5,
            );
            match obstacle_shape {
                ColliderShape::Circle { radius: obs_radius } => {
                    let obs_center = Vec2::new(
                        obstacle_shape_pos.x + obs_radius,
                        obstacle_shape_pos.y + obs_radius,
                    );
                    CollisionWorld::sweep_aabb_against_circle(
                        center,
                        width * 0.5,
                        height * 0.5,
                        desired_delta,
                        obs_center,
                        obs_radius,
                    )
                }
                _ => CollisionWorld::sweep_aabb_against_rect(
                    center,
                    width * 0.5,
                    height * 0.5,
                    desired_delta,
                    obstacle_aabb.0,
                    obstacle_aabb.1,
                ),
            }
        }
        ColliderShape::Point => match obstacle_shape {
            ColliderShape::Circle { radius: obs_radius } => {
                let obs_center = Vec2::new(
                    obstacle_shape_pos.x + obs_radius,
                    obstacle_shape_pos.y + obs_radius,
                );
                CollisionWorld::sweep_aabb_against_circle(
                    moving_shape_pos,
                    0.0,
                    0.0,
                    desired_delta,
                    obs_center,
                    obs_radius,
                )
            }
            _ => CollisionWorld::sweep_aabb_against_rect(
                moving_shape_pos,
                0.0,
                0.0,
                desired_delta,
                obstacle_aabb.0,
                obstacle_aabb.1,
            ),
        },
    }
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
