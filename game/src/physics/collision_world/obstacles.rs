use bishop::prelude::*;
use engine_core::ecs::ColliderShape;
use engine_core::worlds::{
    InteriorZone,
    InteriorZoneBounds,
    Room,
    RoomLayer,
};
use std::collections::HashSet;

use crate::physics::shapes;

use super::SolidObj;

pub(super) fn add_back_layer_bound_obstacles(
    solids: &mut Vec<SolidObj>,
    room_bounds: Rect,
    back_interior_zones: &[InteriorZone],
) {
    for zone in back_interior_zones {
        let forbidden_bounds = subtract_allowed_bounds(room_bounds, &[zone.bounds.to_rect()]);

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

pub(super) fn clamped_back_interior_zones(room: &Room, grid_size: f32) -> Vec<InteriorZone> {
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
                    bounds: InteriorZoneBounds::from_rect(clamp_rect_to_room(
                        zone.bounds.to_rect(),
                        room_bounds,
                    )),
                    ..zone
                })
                .filter(|zone| !rect_is_empty(zone.bounds.to_rect()))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn rect_contains_rect(outer: Rect, inner: Rect) -> bool {
    inner.x >= outer.x - shapes::OVERLAP_EPS
        && inner.y >= outer.y - shapes::OVERLAP_EPS
        && inner.x + inner.w <= outer.x + outer.w + shapes::OVERLAP_EPS
        && inner.y + inner.h <= outer.y + outer.h + shapes::OVERLAP_EPS
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

pub(super) fn add_border_obstacles(solids: &mut Vec<SolidObj>, room: &Room, grid_size: f32) {
    let tilemap = &room.variants[room.current_variant_index()].tilemap;
    let ts = grid_size;
    let w = tilemap.width as i32;
    let h = tilemap.height as i32;

    for layer in [RoomLayer::Front, RoomLayer::Back] {
        let mut outer_exits: HashSet<(i32, i32)> = HashSet::new();
        for exit in room.exits.iter().filter(|exit| exit.layer == layer) {
            outer_exits.insert((exit.position.x as i32, exit.position.y as i32));
        }

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
                    layer: Some(layer),
                    interior_zone: None,
                });
            }
        }

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
                    layer: Some(layer),
                    interior_zone: None,
                });
            }
        }

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
                    layer: Some(layer),
                    interior_zone: None,
                });
            }
        }

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
                    layer: Some(layer),
                    interior_zone: None,
                });
            }
        }

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
                    layer: Some(layer),
                    interior_zone: None,
                });
            }
        }
    }
}
