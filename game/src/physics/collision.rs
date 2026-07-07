use bishop::prelude::*;
use engine_core::assets::*;
use engine_core::ecs::*;
use engine_core::tiles::{TileComponent, TileMap};
use engine_core::worlds::*;
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
    sprite_manager: &'a SpriteManager,
    /// ECS world queried for solid entities and their transforms.
    ecs: &'a Ecs,
    /// Room membership used to scope solid-entity queries.
    room_id: RoomId,
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
        sprite_manager: &'a SpriteManager,
        ecs: &'a Ecs,
        room_id: RoomId,
        tilemap: &'a TileMap,
        room_origin: Vec2,
        exits: &'a [Exit],
        grid_size: f32,
    ) -> SweepContext<'a> {
        SweepContext {
            sprite_manager,
            ecs,
            room_id,
            tilemap,
            room_origin,
            exits,
            grid_size,
        }
    }

    pub(crate) fn sweep_move(
        &self,
        moving_entity: Entity,
        entity_position: Vec2,
        desired_delta: Vec2,
        collider: Collider,
        pivot: Pivot,
    ) -> SweepResult {
        let obstacles = self.collect_obstacles(moving_entity);

        let (sw, sh) = collider.shape.size();
        let size = Vec2::new(sw, sh);
        let collider_pos = pivot_offset(entity_position + collider.offset, size, pivot);

        let (allowed_x, blocked_x) =
            resolve_axis(collider_pos, desired_delta.x, 0, size, &obstacles);

        let pos_after_x = collider_pos + Vec2::new(allowed_x, 0.0);
        let (allowed_y, blocked_y) =
            resolve_axis(pos_after_x, desired_delta.y, 1, size, &obstacles);

        SweepResult {
            allowed_delta: Vec2::new(allowed_x, allowed_y),
            blocked_x,
            blocked_y,
        }
    }

    fn collect_obstacles(&self, moving_entity: Entity) -> Vec<(Vec2, Vec2)> {
        let mut obstacles = Vec::new();

        for ((x, y), tile_def_id) in self.tilemap.tiles.iter() {
            let Some(tile_def) = self.sprite_manager.tile_defs.get(tile_def_id) else {
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

        for &other_entity in self.ecs.entities_in_room(self.room_id) {
            if other_entity == moving_entity {
                continue;
            }
            if !self.ecs.get::<Solid>(other_entity).is_some_and(|solid| solid.0) {
                continue;
            }
            let Some(other_transform) = self.ecs.get::<Transform>(other_entity) else {
                continue;
            };
            let Some(other_coll) = self.ecs.get::<Collider>(other_entity).copied() else {
                continue;
            };
            let other_aabb = aabb(other_transform.position, other_coll, other_transform.pivot);
            obstacles.push(other_aabb);
        }

        obstacles
    }
}

/// Build an axis‑aligned bounding box (AABB) from a position + collider + pivot.
/// The pivot determines which point on the collider aligns with the position.
#[inline]
fn aabb(position: Vec2, collider: Collider, pivot: Pivot) -> (Vec2, Vec2) {
    let (sw, sh) = collider.shape.size();
    let size = Vec2::new(sw, sh);
    let top_left = pivot_offset(position + collider.offset, size, pivot);
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

    for (gx, gy) in [(-1, -1), (w, -1), (-1, h), (w, h)] {
        if !outer_exits.contains(&(gx, gy)) {
            let min = room_origin + vec2(gx as f32 * ts, gy as f32 * ts);
            obstacles.push((min, min + vec2(ts, ts)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::ecs::Collider;

    #[test]
    fn aabb_includes_collider_offset() {
        let pos = Vec2::new(10.0, 20.0);
        let collider = Collider {
            shape: ColliderShape::Aabb { width: 8.0, height: 8.0 },
            ..Default::default()
        };
        let (min, max) = aabb(pos, collider, Pivot::TopLeft);
        assert_eq!(min, Vec2::new(10.0, 20.0));
        assert_eq!(max, Vec2::new(18.0, 28.0));

        let offset_collider = Collider {
            shape: ColliderShape::Aabb { width: 8.0, height: 8.0 },
            offset: Vec2::new(3.0, -2.0),
        };
        let (min_off, max_off) = aabb(pos, offset_collider, Pivot::TopLeft);
        assert_eq!(min_off, Vec2::new(13.0, 18.0));
        assert_eq!(max_off, Vec2::new(21.0, 26.0));
    }

    #[test]
    fn border_obstacles_include_all_four_corners() {
        let mut obstacles = Vec::new();
        let tilemap = TileMap::new(3, 2);
        let tile_size = 16.0;

        add_border_obstacles(&mut obstacles, Vec2::ZERO, &tilemap, &[], tile_size);

        let width = tilemap.width as f32;
        let height = tilemap.height as f32;
        let corner_mins: Vec<Vec2> = obstacles.iter().map(|(min, _)| *min).collect();
        assert!(corner_mins.contains(&vec2(-tile_size, -tile_size)));
        assert!(corner_mins.contains(&vec2(width * tile_size, -tile_size)));
        assert!(corner_mins.contains(&vec2(-tile_size, height * tile_size)));
        assert!(corner_mins.contains(&vec2(width * tile_size, height * tile_size)));
    }

    #[test]
    fn border_obstacles_keep_declared_exit_cells_open() {
        let mut obstacles = Vec::new();
        let tilemap = TileMap::new(3, 3);
        let tile_size = 16.0;
        let exit_x = 1.0;
        let exits = vec![Exit {
            position: vec2(exit_x, -1.0),
            ..Default::default()
        }];

        add_border_obstacles(&mut obstacles, Vec2::ZERO, &tilemap, &exits, tile_size);

        let border_mins: Vec<Vec2> = obstacles.iter().map(|(min, _)| *min).collect();
        assert!(!border_mins.contains(&vec2(exit_x * tile_size, -tile_size)));
    }

    #[test]
    fn same_room_solid_entity_blocks_sweep() {
        let mut ecs = Ecs::default();
        let room_id = RoomId(1);
        let mover = ecs
            .create_entity()
            .with_current_room(room_id)
            .with(Transform {
                pivot: Pivot::TopLeft,
                ..Default::default()
            })
            .finish();
        ecs.create_entity()
            .with_current_room(room_id)
            .with(Transform {
                position: Vec2::new(12.0, 0.0),
                pivot: Pivot::TopLeft,
                ..Default::default()
            })
            .with(Collider {
                shape: ColliderShape::Aabb {
                    width: 8.0,
                    height: 8.0,
                },
                ..Default::default()
            })
            .with(Solid(true))
            .finish();

        let tilemap = TileMap::new(8, 8);
        let sweep = SweepContext::new(
            &SpriteManager::default(),
            &ecs,
            room_id,
            &tilemap,
            Vec2::ZERO,
            &[],
            16.0,
        )
        .sweep_move(
            mover,
            Vec2::ZERO,
            Vec2::new(16.0, 0.0),
            Collider {
                shape: ColliderShape::Aabb {
                    width: 8.0,
                    height: 8.0,
                },
                ..Default::default()
            },
            Pivot::TopLeft,
        );

        assert!(sweep.blocked_x);
    }

    #[test]
    fn other_room_solid_entity_does_not_block_sweep() {
        let mut ecs = Ecs::default();
        let mover = ecs
            .create_entity()
            .with_current_room(RoomId(1))
            .with(Transform {
                pivot: Pivot::TopLeft,
                ..Default::default()
            })
            .finish();
        ecs.create_entity()
            .with_current_room(RoomId(2))
            .with(Transform {
                position: Vec2::new(12.0, 0.0),
                pivot: Pivot::TopLeft,
                ..Default::default()
            })
            .with(Collider {
                shape: ColliderShape::Aabb {
                    width: 8.0,
                    height: 8.0,
                },
                ..Default::default()
            })
            .with(Solid(true))
            .finish();

        let tilemap = TileMap::new(8, 8);
        let sweep = SweepContext::new(
            &SpriteManager::default(),
            &ecs,
            RoomId(1),
            &tilemap,
            Vec2::ZERO,
            &[],
            16.0,
        )
        .sweep_move(
            mover,
            Vec2::ZERO,
            Vec2::new(16.0, 0.0),
            Collider {
                shape: ColliderShape::Aabb {
                    width: 8.0,
                    height: 8.0,
                },
                ..Default::default()
            },
            Pivot::TopLeft,
        );

        assert!(!sweep.blocked_x);
    }
}
