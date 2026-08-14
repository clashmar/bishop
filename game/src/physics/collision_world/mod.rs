mod obstacles;
mod sweep;

use bishop::prelude::*;
use engine_core::ecs::*;
use engine_core::worlds::*;
use std::collections::HashMap;

use crate::physics::shapes;

use sweep::SweepContext;
use obstacles::{
    add_back_layer_bound_obstacles,
    add_border_obstacles,
    clamped_back_interior_zones,
    rect_contains_rect,
};

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
        let sweep_ctx = SweepContext {
            moving_entity,
            moving_layer,
            active_back_zone,
        };

        if let ColliderShape::Circle { radius } = collider.shape {
            let center = Vec2::new(collider_pos.x + radius, collider_pos.y + radius);
            return self
                .sweep_circle(center, radius, desired_delta, sweep_ctx)
                .finish(desired_delta);
        }

        if let ColliderShape::Capsule { radius, height } = collider.shape {
            let center = Vec2::new(
                collider_pos.x + radius,
                collider_pos.y + radius + height * 0.5,
            );
            return self
                .sweep_capsule(center, radius, height, desired_delta, sweep_ctx)
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
                    .sweep_aabb(center, width * 0.5, height * 0.5, desired_delta, sweep_ctx)
                    .finish(desired_delta);
            }
        }

        let (allowed_x, blocked_x) = self.resolve_axis(
            collider.shape,
            collider_pos,
            desired_delta.x,
            0,
            sweep_ctx,
        );

        let pos_after_x = Vec2::new(collider_pos.x + allowed_x, collider_pos.y);
        let (allowed_y, blocked_y) = self.resolve_axis(
            collider.shape,
            pos_after_x,
            desired_delta.y,
            1,
            sweep_ctx,
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
            .find(|zone| rect_contains_rect(zone.bounds.to_rect(), collider_bounds))
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

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
