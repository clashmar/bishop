use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::worlds::*;
use std::collections::{HashMap, HashSet};

use crate::physics::shapes;

/// Information returned by a sweep-move query.
pub struct SweepResult {
    pub allowed_delta: Vec2,
    pub blocked_x: bool,
    pub blocked_y: bool,
}

#[derive(Clone, Copy)]
struct SweepData {
    t_x: f32,
    t_y: f32,
    push_x: f32,
    push_y: f32,
    blocked_x: bool,
    blocked_y: bool,
}

impl SweepData {
    fn finish(self, desired_delta: Vec2) -> SweepResult {
        SweepResult {
            allowed_delta: Vec2::new(
                desired_delta.x * self.t_x + self.push_x,
                desired_delta.y * self.t_y + self.push_y,
            ),
            blocked_x: self.blocked_x,
            blocked_y: self.blocked_y,
        }
    }
}

struct SolidObj {
    aabb: (Vec2, Vec2),
    shape: ColliderShape,
    shape_pos: Vec2,
    entity: Option<Entity>,
    layer: Option<RoomLayer>,
    interior_zone: Option<InteriorZoneId>,
}

/// Collision world built once per frame per room. Owns its obstacle data.
pub struct CollisionWorld {
    solids: Vec<SolidObj>,
    entity_layers: HashMap<Entity, RoomLayer>,
    back_interior_zones: Vec<InteriorZone>,
}

impl CollisionWorld {
    /// Collects solid tile placements, room borders, and solid ECS entities into a
    /// collision world for the given room.
    pub fn new(
        ecs: &Ecs,
        room: &Room,
        world: &World,
    ) -> Self {
        let mut solids = Vec::new();
        let back_interior_zones = clamped_back_interior_zones(room, world.grid_size);
        let entity_layers = ecs
            .entities_in_room(room.id)
            .iter()
            .filter_map(|&entity| {
                ecs.get::<CurrentRoom>(entity)
                    .map(|current_room| (entity, current_room.layer))
            })
            .collect::<HashMap<_, _>>();

        for (&entity, &layer) in &entity_layers {
            let Some(tile) = ecs.get::<TilePlacement>(entity) else {
                continue;
            };
            if ecs.get::<Solid>(entity).is_some_and(|solid| solid.0) {
                let tile_pos = room.position
                    + vec2(tile.grid_x as f32 * world.grid_size, tile.grid_y as f32 * world.grid_size);
                let tile_aabb = (
                    tile_pos,
                    tile_pos + vec2(world.grid_size, world.grid_size),
                );
                solids.push(SolidObj {
                    aabb: tile_aabb,
                    shape: ColliderShape::Aabb {
                        width: world.grid_size,
                        height: world.grid_size,
                    },
                    shape_pos: tile_pos,
                    entity: Some(entity),
                    layer: Some(layer),
                    interior_zone: None,
                });
            }
        }

        add_border_obstacles(&mut solids, room, world.grid_size);
        add_back_layer_bound_obstacles(
            &mut solids,
            room.world_rect(world.grid_size),
            &back_interior_zones,
        );

        for (&entity, &layer) in &entity_layers {
            if ecs.get::<TilePlacement>(entity).is_some() {
                continue;
            }
            if !ecs.get::<Solid>(entity).is_some_and(|solid| solid.0) {
                continue;
            }
            let Some(transform) = ecs.get::<Transform>(entity) else {
                continue;
            };
            let collider = ecs.get::<Collider>(entity).copied().unwrap_or_default();
            let entity_aabb =
                shapes::collider_aabb(transform.position, collider, transform.pivot);
            solids.push(SolidObj {
                aabb: entity_aabb,
                shape: collider.shape,
                shape_pos: entity_aabb.0,
                entity: Some(entity),
                layer: Some(layer),
                interior_zone: None,
            });
        }

        CollisionWorld {
            solids,
            entity_layers,
            back_interior_zones,
        }
    }

    /// Sweep the moving entity's collider from `entity_position` by `desired_delta`,
    /// resolving collisions against all solids in this world.
    pub fn sweep_move(
        &self,
        moving_entity: Entity,
        entity_position: Vec2,
        desired_delta: Vec2,
        collider: Collider,
        pivot: Pivot,
    ) -> SweepResult {
        let collider_aabb = shapes::collider_aabb(entity_position, collider, pivot);
        let collider_pos = collider_aabb.0;
        let moving_layer = self
            .entity_layers
            .get(&moving_entity)
            .copied()
            .unwrap_or(RoomLayer::Front);
        let active_back_zone = self.active_back_zone(collider_aabb);

        if let ColliderShape::Circle { radius } = collider.shape {
            let center = Vec2::new(collider_pos.x + radius, collider_pos.y + radius);
            return self
                .sweep_circle(
                    center,
                    radius,
                    desired_delta,
                    moving_entity,
                    moving_layer,
                    active_back_zone,
                )
                .finish(desired_delta);
        }

        if let ColliderShape::Capsule { radius, height } = collider.shape {
            let center = Vec2::new(
                collider_pos.x + radius,
                collider_pos.y + radius + height * 0.5,
            );
            return self
                .sweep_capsule(
                    center,
                    radius,
                    height,
                    desired_delta,
                    moving_entity,
                    moving_layer,
                    active_back_zone,
                )
                .finish(desired_delta);
        }

        if let ColliderShape::Aabb { width, height } = collider.shape {
            if self
                .solids
                .iter()
                .any(|solid| {
                    self.solid_affects_layer(solid, moving_layer, active_back_zone)
                        && matches!(solid.shape, ColliderShape::Circle { .. })
                })
            {
                let center = Vec2::new(
                    collider_pos.x + width * 0.5,
                    collider_pos.y + height * 0.5,
                );
                return self
                    .sweep_aabb_2d(
                        center,
                        width * 0.5,
                        height * 0.5,
                        desired_delta,
                        moving_entity,
                        moving_layer,
                        active_back_zone,
                    )
                    .finish(desired_delta);
            }
        }

        let (allowed_x, blocked_x) = self.resolve_axis(
            collider.shape,
            collider_pos,
            desired_delta.x,
            0,
            moving_entity,
            moving_layer,
            active_back_zone,
        );

        let pos_after_x = Vec2::new(collider_pos.x + allowed_x, collider_pos.y);
        let (allowed_y, blocked_y) = self.resolve_axis(
            collider.shape,
            pos_after_x,
            desired_delta.y,
            1,
            moving_entity,
            moving_layer,
            active_back_zone,
        );

        SweepResult {
            allowed_delta: Vec2::new(allowed_x, allowed_y),
            blocked_x,
            blocked_y,
        }
    }

    /// Returns sensor entities whose AABB overlaps the query collider's AABB.
    pub fn check_overlaps(
        &self,
        _position: Vec2,
        _collider: Collider,
        _pivot: Pivot,
    ) -> Vec<Entity> {
        Vec::new()
    }

    /// 2D sweep for circle colliders.
    fn sweep_circle(
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
    fn sweep_capsule(
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
    fn sweep_aabb_2d(
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

    fn sweep_circle_against_rect(
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
        if !circle_touches_rect(contact_center, radius, obs_min, obs_max) {
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

    fn circle_rect_contact_time(
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
        if circle_touches_rect(contact_center, radius, obs_min, obs_max) {
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

    fn sweep_circle_against_circle(
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

    fn sweep_aabb_against_rect(
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

    fn sweep_aabb_against_circle(
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

    fn resolve_axis(
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

    fn solid_affects_layer(
        &self,
        solid: &SolidObj,
        moving_layer: RoomLayer,
        active_back_zone: Option<InteriorZoneId>,
    ) -> bool {
        if !solid.layer.is_none_or(|layer| layer == moving_layer) {
            return false;
        }

        let Some(zone_id) = solid.interior_zone else {
            return true;
        };

        moving_layer == RoomLayer::Back
            && active_back_zone.is_none_or(|active_zone| active_zone == zone_id)
    }

    fn active_back_zone(&self, collider_aabb: (Vec2, Vec2)) -> Option<InteriorZoneId> {
        if self.back_interior_zones.is_empty() {
            return None;
        }

        let collider_bounds = Rect::new(
            collider_aabb.0.x,
            collider_aabb.0.y,
            collider_aabb.1.x - collider_aabb.0.x,
            collider_aabb.1.y - collider_aabb.0.y,
        );
        if let Some(zone) = self
            .back_interior_zones
            .iter()
            .find(|zone| rect_contains_rect(zone.bounds, collider_bounds))
        {
            return Some(zone.id);
        }

        let center = collider_bounds.center();
        self.back_interior_zones
            .iter()
            .find(|zone| zone.bounds.contains(center))
            .map(|zone| zone.id)
    }
}

fn ray_axis(start: f32, dir: f32, lo: f32, hi: f32) -> (f32, f32) {
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

fn circle_touches_rect(c: Vec2, r: f32, obs_min: Vec2, obs_max: Vec2) -> bool {
    let closest = Vec2::new(
        c.x.clamp(obs_min.x, obs_max.x),
        c.y.clamp(obs_min.y, obs_max.y),
    );
    let dist_sq = (c.x - closest.x).powi(2) + (c.y - closest.y).powi(2);
    dist_sq <= r.powi(2) + shapes::OVERLAP_EPS
}

/// Solves for the earliest t in [0, 1] where a sweeping circle touches a point.
fn circle_cast_corner(center: Vec2, delta: Vec2, radius: f32, corner: Vec2) -> Option<f32> {
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

fn axis_blocked_by_normal(desired_delta: Vec2, normal: Vec2, axis: usize) -> bool {
    let (axis_delta, axis_normal, other_normal) = if axis == 0 {
        (desired_delta.x, normal.x, normal.y)
    } else {
        (desired_delta.y, normal.y, normal.x)
    };

    axis_delta * axis_normal < -shapes::OVERLAP_EPS
        && axis_normal.abs() + shapes::OVERLAP_EPS >= other_normal.abs()
}

fn aabb_overlaps(c: Vec2, hw: f32, hh: f32, obs_min: Vec2, obs_max: Vec2) -> bool {
    c.x + hw >= obs_min.x - shapes::OVERLAP_EPS
        && c.x - hw <= obs_max.x + shapes::OVERLAP_EPS
        && c.y + hh >= obs_min.y - shapes::OVERLAP_EPS
        && c.y - hh <= obs_max.y + shapes::OVERLAP_EPS
}

fn select_stronger_push(pushes: [f32; 3]) -> f32 {
    let mut selected = 0.0f32;
    for push in pushes {
        if push.abs() > selected.abs() {
            selected = push;
        }
    }
    selected
}


fn add_back_layer_bound_obstacles(
    solids: &mut Vec<SolidObj>,
    room_bounds: Rect,
    back_interior_zones: &[InteriorZone],
) {
    for zone in back_interior_zones {
        let forbidden_bounds = subtract_allowed_bounds(room_bounds, &[zone.bounds]);

        for bounds in forbidden_bounds {
            let min = vec2(bounds.x, bounds.y);
            solids.push(SolidObj {
                aabb: (min, min + vec2(bounds.w, bounds.h)),
                shape: ColliderShape::Aabb {
                    width: bounds.w,
                    height: bounds.h,
                },
                shape_pos: min,
                entity: None,
                layer: Some(RoomLayer::Back),
                interior_zone: Some(zone.id),
            });
        }
    }
}

fn clamped_back_interior_zones(room: &Room, grid_size: f32) -> Vec<InteriorZone> {
    let room_bounds = room.world_rect(grid_size);
    room.current_variant()
        .layers
        .back
        .as_ref()
        .map(|back| {
            back.interior_zones
                .iter()
                .copied()
                .map(|zone| InteriorZone {
                    bounds: clamp_rect_to_room(zone.bounds, room_bounds),
                    ..zone
                })
                .filter(|zone| !rect_is_empty(zone.bounds))
                .collect()
        })
        .unwrap_or_default()
}

fn subtract_allowed_bounds(room_bounds: Rect, allowed_bounds: &[Rect]) -> Vec<Rect> {
    let allowed = allowed_bounds
        .iter()
        .map(|rect| clamp_rect_to_room(*rect, room_bounds))
        .filter(|rect| !rect_is_empty(*rect))
        .collect::<Vec<_>>();
    if allowed.is_empty() {
        return vec![room_bounds];
    }

    let mut xs = vec![room_bounds.x, room_bounds.x + room_bounds.w];
    let mut ys = vec![room_bounds.y, room_bounds.y + room_bounds.h];
    for rect in &allowed {
        xs.push(rect.x);
        xs.push(rect.x + rect.w);
        ys.push(rect.y);
        ys.push(rect.y + rect.h);
    }
    xs.sort_by(f32::total_cmp);
    ys.sort_by(f32::total_cmp);
    xs.dedup_by(|a, b| (*a - *b).abs() <= shapes::OVERLAP_EPS);
    ys.dedup_by(|a, b| (*a - *b).abs() <= shapes::OVERLAP_EPS);

    let mut forbidden = Vec::new();
    for x_pair in xs.windows(2) {
        for y_pair in ys.windows(2) {
            let x0 = x_pair[0];
            let x1 = x_pair[1];
            let y0 = y_pair[0];
            let y1 = y_pair[1];
            let cell = Rect::new(x0, y0, x1 - x0, y1 - y0);
            if rect_is_empty(cell) {
                continue;
            }

            let center = vec2(cell.x + cell.w * 0.5, cell.y + cell.h * 0.5);
            if !room_bounds.contains(center) {
                continue;
            }
            if allowed.iter().any(|rect| rect.contains(center)) {
                continue;
            }

            forbidden.push(cell);
        }
    }

    forbidden
}

fn rect_contains_rect(outer: Rect, inner: Rect) -> bool {
    inner.x >= outer.x - shapes::OVERLAP_EPS
        && inner.y >= outer.y - shapes::OVERLAP_EPS
        && inner.x + inner.w <= outer.x + outer.w + shapes::OVERLAP_EPS
        && inner.y + inner.h <= outer.y + outer.h + shapes::OVERLAP_EPS
}

fn clamp_rect_to_room(rect: Rect, room_bounds: Rect) -> Rect {
    let x0 = rect.x.max(room_bounds.x);
    let y0 = rect.y.max(room_bounds.y);
    let x1 = (rect.x + rect.w).min(room_bounds.x + room_bounds.w);
    let y1 = (rect.y + rect.h).min(room_bounds.y + room_bounds.h);
    Rect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
}

fn rect_is_empty(rect: Rect) -> bool {
    rect.w <= shapes::OVERLAP_EPS || rect.h <= shapes::OVERLAP_EPS
}

fn add_border_obstacles(solids: &mut Vec<SolidObj>, room: &Room, grid_size: f32) {
    let tilemap = &room.variants[room.current_variant_index()].tilemap;
    let ts = grid_size;
    let w = tilemap.width as i32;
    let h = tilemap.height as i32;

    let mut outer_exits: HashSet<(i32, i32)> =
        HashSet::with_capacity(room.exits.len());
    for e in &room.exits {
        outer_exits.insert((e.position.x as i32, e.position.y as i32));
    }

    // Top border (y = -1)
    for gx in 0..w {
        if !outer_exits.contains(&(gx, -1)) {
            let min = room.position + vec2(gx as f32 * ts, -ts);
            solids.push(SolidObj {
                aabb: (min, min + vec2(ts, ts)),
                shape: ColliderShape::Aabb {
                    width: ts,
                    height: ts,
                },
                shape_pos: min,
                entity: None,
                layer: None,
                interior_zone: None,
            });
        }
    }

    // Bottom border (y = h)
    for gx in 0..w {
        if !outer_exits.contains(&(gx, h)) {
            let min = room.position + vec2(gx as f32 * ts, h as f32 * ts);
            solids.push(SolidObj {
                aabb: (min, min + vec2(ts, ts)),
                shape: ColliderShape::Aabb {
                    width: ts,
                    height: ts,
                },
                shape_pos: min,
                entity: None,
                layer: None,
                interior_zone: None,
            });
        }
    }

    // Left border (x = -1)
    for gy in 0..h {
        if !outer_exits.contains(&(-1, gy)) {
            let min = room.position + vec2(-ts, gy as f32 * ts);
            solids.push(SolidObj {
                aabb: (min, min + vec2(ts, ts)),
                shape: ColliderShape::Aabb {
                    width: ts,
                    height: ts,
                },
                shape_pos: min,
                entity: None,
                layer: None,
                interior_zone: None,
            });
        }
    }

    // Right border (x = w)
    for gy in 0..h {
        if !outer_exits.contains(&(w, gy)) {
            let min = room.position + vec2(w as f32 * ts, gy as f32 * ts);
            solids.push(SolidObj {
                aabb: (min, min + vec2(ts, ts)),
                shape: ColliderShape::Aabb {
                    width: ts,
                    height: ts,
                },
                shape_pos: min,
                entity: None,
                layer: None,
                interior_zone: None,
            });
        }
    }

    // Corners
    for (gx, gy) in [(-1, -1), (w, -1), (-1, h), (w, h)] {
        if !outer_exits.contains(&(gx, gy)) {
            let min = room.position + vec2(gx as f32 * ts, gy as f32 * ts);
            solids.push(SolidObj {
                aabb: (min, min + vec2(ts, ts)),
                shape: ColliderShape::Aabb {
                    width: ts,
                    height: ts,
                },
                shape_pos: min,
                entity: None,
                layer: None,
                interior_zone: None,
            });
        }
    }
}

#[cfg(test)]
#[path = "tests/collision_world_tests.rs"]
mod tests;
