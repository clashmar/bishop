// game/src/physics/collision.rs
use bishop::prelude::*;
use engine_core::prelude::*;
use std::collections::HashSet;

const OVERLAP_EPS: f32 = 0.0001;

/// Information returned by the sweep test.
pub struct SweepResult {
    /// The displacement that can actually be applied without intersecting anything.
    pub allowed_delta: Vec2,
    /// Was the X‑axis blocked?
    pub blocked_x: bool,
    /// Was the Y‑axis blocked?
    pub blocked_y: bool,
}

/// Aggregates all solid geometry visible to a single sweep-move query —
/// tile solids, room border walls, and solid ECS entities.
pub(crate) struct SweepContext<'a> {
    /// Asset lookup used to resolve tile definitions.
    asset_manager: &'a AssetManager,
    /// ECS world queried for solid entities and their transforms.
    ecs: &'a Ecs,
    /// Tilemap for the active room variant.
    tilemap: &'a TileMap,
    /// World-space origin of the room.
    room_origin: Vec2,
    /// Exits that should remain open in the room border.
    exits: &'a [Exit],
    /// Size of a tile in world units.
    grid_size: f32,
}

impl SweepContext<'_> {
    pub(crate) fn new<'a>(
        asset_manager: &'a AssetManager,
        ecs: &'a Ecs,
        tilemap: &'a TileMap,
        room_origin: Vec2,
        exits: &'a [Exit],
        grid_size: f32,
    ) -> SweepContext<'a> {
        SweepContext {
            asset_manager,
            ecs,
            tilemap,
            room_origin,
            exits,
            grid_size,
        }
    }

    pub(crate) fn sweep_move(
        &self,
        entity_position: Vec2,
        desired_delta: Vec2,
        collider: Collider,
        pivot: Pivot,
    ) -> SweepResult {
        let obstacles = self.collect_obstacles(entity_position);

        let collider_size = Vec2::new(collider.width, collider.height);
        let collider_pos = pivot_offset(entity_position, collider_size, pivot);

        let (allowed_x, blocked_x) =
            resolve_axis(collider_pos, desired_delta.x, 0, collider_size, &obstacles);

        let pos_after_x = collider_pos + Vec2::new(allowed_x, 0.0);
        let (allowed_y, blocked_y) =
            resolve_axis(pos_after_x, desired_delta.y, 1, collider_size, &obstacles);

        SweepResult {
            allowed_delta: Vec2::new(allowed_x, allowed_y),
            blocked_x,
            blocked_y,
        }
    }

    fn collect_obstacles(&self, entity_position: Vec2) -> Vec<(Vec2, Vec2)> {
        let mut obstacles = Vec::new();

        for ((x, y), tile_def_id) in self.tilemap.tiles.iter() {
            let Some(tile_def) = self.asset_manager.tile_defs.get(tile_def_id) else {
                continue;
            };

            if tile_def.components.contains(&TileComponent::Solid(true)) {
                let tile_pos = self.room_origin
                    + vec2(*x as f32 * self.grid_size, *y as f32 * self.grid_size);
                let tile_aabb = (tile_pos, tile_pos + vec2(self.grid_size, self.grid_size));
                obstacles.push(tile_aabb);
            }
        }

        add_border_obstacles(
            &mut obstacles,
            self.room_origin,
            self.tilemap,
            self.exits,
            self.grid_size,
        );

        for (other_entity, other_coll) in self.ecs.get_store::<Collider>().data.iter() {
            if let Some(other_pos) = self.ecs.get::<Transform>(*other_entity) {
                if (other_pos.position - entity_position).length() < 0.001 {
                    continue;
                }
            }

            if self.ecs.get::<Solid>(*other_entity).is_some_and(|solid| solid.0) {
                if let Some(other_transform) = self.ecs.get::<Transform>(*other_entity) {
                    let other_aabb = aabb(other_transform.position, *other_coll, other_transform.pivot);
                    obstacles.push(other_aabb);
                }
            }
        }

        obstacles
    }
}

/// Build an axis‑aligned bounding box (AABB) from a position + collider + pivot.
/// The pivot determines which point on the collider aligns with the position.
#[inline]
fn aabb(position: Vec2, collider: Collider, pivot: Pivot) -> (Vec2, Vec2) {
    let size = Vec2::new(collider.width, collider.height);
    let top_left = pivot_offset(position, size, pivot);
    (top_left, top_left + size)
}

/// Resolve a single axis (X or Y).
fn resolve_axis(
    position: Vec2,
    delta: f32,
    axis: usize,
    this_size: Vec2,
    obstacles: &[(Vec2, Vec2)],
) -> (f32, bool) {
    if delta == 0.0 {
        return (0.0, false);
    }

    let mut allowed = delta;
    let mut blocked = false;

    let (my_min, my_max) = if axis == 0 {
        (position.x, position.x + this_size.x)
    } else {
        (position.y, position.y + this_size.y)
    };

    for (obs_min, obs_max) in obstacles.iter() {
        let (obs_min_axis, obs_max_axis) = if axis == 0 {
            (obs_min.x, obs_max.x)
        } else {
            (obs_min.y, obs_max.y)
        };

        // Overlap on the other axis
        let overlap_other = if axis == 0 {
            !(position.y + this_size.y <= obs_min.y + OVERLAP_EPS
                || position.y >= obs_max.y - OVERLAP_EPS)
        } else {
            !(position.x + this_size.x <= obs_min.x + OVERLAP_EPS
                || position.x >= obs_max.x - OVERLAP_EPS)
        };

        if !overlap_other {
            continue;
        }

        // Apply directional epsilon for movement axis
        if delta > 0.0 {
            // Moving positive (right or down)
            if my_max <= obs_min_axis + OVERLAP_EPS && my_max + delta > obs_min_axis {
                let dist = obs_min_axis - my_max;
                if dist < allowed {
                    allowed = dist;
                    blocked = true;
                }
            }
        } else {
            // Moving negative (left or up)
            if my_min >= obs_max_axis - OVERLAP_EPS && my_min + delta < obs_max_axis {
                let dist = obs_max_axis - my_min;
                if dist > allowed {
                    allowed = dist;
                    blocked = true;
                }
            }
        }
    }

    (allowed, blocked)
}

/// Creates solid tiles around the edge of the tilemap to constrain movement.
fn add_border_obstacles(
    obstacles: &mut Vec<(Vec2, Vec2)>,
    room_origin: Vec2,
    tilemap: &TileMap,
    exits: &[Exit],
    grid_size: f32,
) {
    let ts = grid_size;
    let w = tilemap.width as i32;
    let h = tilemap.height as i32;

    let mut outer_exits: HashSet<(i32, i32)> = HashSet::with_capacity(exits.len());
    for e in exits {
        outer_exits.insert((e.position.x as i32, e.position.y as i32));
    }

    for gx in 0..w {
        if !outer_exits.contains(&(gx, -1)) {
            let min = room_origin + vec2(gx as f32 * ts, -ts);
            obstacles.push((min, min + vec2(ts, ts)));
        }
    }

    for gx in 0..w {
        if !outer_exits.contains(&(gx, h)) {
            let min = room_origin + vec2(gx as f32 * ts, h as f32 * ts);
            obstacles.push((min, min + vec2(ts, ts)));
        }
    }

    for gy in 0..h {
        if !outer_exits.contains(&(-1, gy)) {
            let min = room_origin + vec2(-ts, gy as f32 * ts);
            obstacles.push((min, min + vec2(ts, ts)));
        }
    }

    for gy in 0..h {
        if !outer_exits.contains(&(w, gy)) {
            let min = room_origin + vec2(w as f32 * ts, gy as f32 * ts);
            obstacles.push((min, min + vec2(ts, ts)));
        }
    }
}
