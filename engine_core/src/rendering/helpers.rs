use bishop::prelude::*;
use crate::ecs::*;
use crate::assets::*;
use crate::rendering::renderable::Renderable;
use crate::worlds::{ExitDirection, Room, RoomBounds, RoomId, World};
use std::collections::HashMap;

/// Common display refresh rates to snap frame times to (checked in order).
const SNAP_FREQUENCIES: [f32; 5] = [60.0, 120.0, 144.0, 240.0, 30.0];

/// Scale factor applied to outline_thickness for entity outlines.
pub const ENTITY_OUTLINE_SCALE: f32 = 0.25;

/// Tracks whether the frame-time EMA has been seeded from a reliable sample.
#[derive(Debug, Default)]
pub enum SmoothedDtState {
    /// No frame processed yet.
    #[default]
    AwaitingFirstFrame,
    /// First frame discarded; EMA seeds from the next call.
    AwaitingSeed,
    /// EMA is seeded and actively smoothing.
    Active(f32),
}

/// Resolves the entity to use for visual lookups. A `PlayerProxy` redirects
/// to the actual player entity so the proxy renders with the player's visuals.
pub fn resolve_visual_entity(ecs: &Ecs, entity: Entity) -> Entity {
    if ecs.has::<PlayerProxy>(entity) {
        ecs.get_player_entity().unwrap_or(entity)
    } else {
        entity
    }
}

/// Compares two entities in deterministic draw order.
pub fn compare_entity_draw_order(
    a_entity: Entity,
    a_z: i32,
    b_entity: Entity,
    b_z: i32,
) -> std::cmp::Ordering {
    a_z.cmp(&b_z).then_with(|| a_entity.cmp(&b_entity))
}

/// Returns the pixel dimensions of an entity for rendering.
pub fn entity_dimensions(
    ecs: &Ecs,
    sprite_manager: &SpriteManager,
    entity: Entity,
    grid_size: f32,
) -> Vec2 {
    let visual_entity = resolve_visual_entity(ecs, entity);
    let from_anim = ecs
        .get_store::<CurrentFrame>()
        .get(visual_entity)
        .and_then(|cf| cf.dimensions(sprite_manager));

    let from_sprite = || {
        ecs.get_store::<Sprite>()
            .get(visual_entity)
            .and_then(|s| s.dimensions(sprite_manager))
    };

    from_anim
        .or_else(from_sprite)
        .unwrap_or(Vec2::splat(grid_size))
}

/// Returns the true visual position for rendering, including any sub-pixel remainder.
#[inline]
pub fn visual_position(position: Vec2, sub_pixel: Option<&SubPixel>) -> Vec2 {
    let sub_pixel = sub_pixel.copied().unwrap_or_default();
    position + Vec2::new(sub_pixel.x, sub_pixel.y)
}

/// Linearly interpolates between two positions and rounds to the nearest pixel.
#[inline]
pub fn lerp_rounded(prev_pos: Vec2, current_pos: Vec2, alpha: f32) -> Vec2 {
    lerp_position(prev_pos, current_pos, alpha).round()
}

/// Linearly interpolates between two positions without pixel snapping.
#[inline]
pub fn lerp_position(prev_pos: Vec2, current_pos: Vec2, alpha: f32) -> Vec2 {
    prev_pos * (1.0 - alpha) + current_pos * alpha
}

/// Returns the interpolated render position or the current position.
#[inline]
pub fn interpolate_position(
    entity: Entity,
    current_pos: Vec2,
    alpha: f32,
    prev_positions: Option<&HashMap<Entity, Vec2>>,
) -> Vec2 {
    if let Some(prev_map) = prev_positions
        && let Some(prev_pos) = prev_map.get(&entity)
    {
        return lerp_position(*prev_pos, current_pos, alpha);
    }

    current_pos
}

/// Smooths `raw_dt` using an exponential moving average with the given `alpha`.
#[inline]
pub fn smooth_dt(state: &mut SmoothedDtState, raw_dt: f32, alpha: f32) -> f32 {
    match state {
        SmoothedDtState::AwaitingFirstFrame => {
            *state = SmoothedDtState::AwaitingSeed;
            snap_dt(raw_dt)
        }
        SmoothedDtState::AwaitingSeed => {
            let snapped = snap_dt(raw_dt);
            *state = SmoothedDtState::Active(snapped);
            snapped
        }
        SmoothedDtState::Active(s) => {
            *s = *s * alpha + raw_dt * (1.0 - alpha);
            *s
        }
    }
}

/// Snaps raw_dt to the nearest common display interval if within 10% of it.
/// Eliminates accumulator drift that causes periodic stutter.
#[inline]
pub fn snap_dt(raw_dt: f32) -> f32 {
    for freq in SNAP_FREQUENCIES {
        let target = 1.0 / freq;
        if (raw_dt - target).abs() < target * 0.1 {
            return target;
        }
    }
    raw_dt
}

/// Returns the outline thickness for the given grid size.
pub fn outline_thickness(grid_size: f32) -> f32 {
    (grid_size * 0.2).max(1.0)
}

/// Returns the top-left draw position after applying the entity pivot offset.
#[inline]
pub fn pivot_adjusted_position(entity_pos: Vec2, texture_size: Vec2, pivot: Pivot) -> Vec2 {
    let offset = pivot.as_normalized();
    vec2(
        entity_pos.x - texture_size.x * offset.x,
        entity_pos.y - texture_size.y * offset.y,
    )
}

/// Returns the rendered room plus neighboring rooms relevant to spillover visibility.
pub fn spillover_candidate_room_ids(world: &World, room: &Room) -> Vec<RoomId> {
    let bounds = RoomBounds::from_room(room, world.grid_size);
    let mut ids = vec![room.id];
    ids.extend(world.room_grid.neighboring_rooms(&bounds, room.id));
    ids
}

/// Checks whether an entity should be visible in a room.
pub fn entity_visible_in_room(
    ecs: &Ecs,
    sprite_manager: &SpriteManager,
    world: &World,
    entity: Entity,
    entity_room_id: RoomId,
    visual_pos: Vec2,
    room: &Room,
    grid_size: f32,
) -> bool {
    if entity_room_id == room.id {
        return true;
    }

    let Some(other_room) = world.get_room(entity_room_id) else {
        return false;
    };

    let entity_rect = entity_visual_rect(ecs, sprite_manager, entity, visual_pos, grid_size);
    let Some(overlap_rect) = entity_rect.intersection(&room.world_rect(grid_size)) else {
        return false;
    };

    room
        .exits_facing_room(other_room, grid_size)
        .into_iter()
        .any(|exit| {
            let exit_rect = projected_exit_cell_rect(
                room,
                exit.world_grid_position,
                exit.direction,
                grid_size,
            );
            overlap_rect.overlaps(&exit_rect)
        })
}

/// Checks whether an entity's visual body overlaps a room's world-space rectangle.
pub fn entity_visual_overlaps_room(
    ecs: &Ecs,
    sprite_manager: &SpriteManager,
    entity: Entity,
    visual_pos: Vec2,
    room: &Room,
    grid_size: f32,
) -> bool {
    entity_visual_rect(ecs, sprite_manager, entity, visual_pos, grid_size)
        .overlaps(&room.world_rect(grid_size))
}

/// Returns the world-space rectangle occupied by the entity's rendered visual.
pub fn entity_visual_rect(
    ecs: &Ecs,
    sprite_manager: &SpriteManager,
    entity: Entity,
    visual_pos: Vec2,
    grid_size: f32,
) -> Rect {
    let visual_entity = resolve_visual_entity(ecs, entity);
    let pivot = ecs
        .get_store::<Transform>()
        .get(entity)
        .map(|transform| transform.pivot)
        .unwrap_or(Pivot::BottomCenter);

    if let Some(current_frame) = ecs.get_store::<CurrentFrame>().get(visual_entity) {
        let size = current_frame.frame_size;
        let draw_base = pivot_adjusted_position(visual_pos, size, pivot) + current_frame.offset;
        return Rect::new(draw_base.x, draw_base.y, size.x, size.y);
    }

    let size = entity_dimensions(ecs, sprite_manager, entity, grid_size);
    let draw_base = pivot_adjusted_position(visual_pos, size, pivot);
    Rect::new(draw_base.x, draw_base.y, size.x, size.y)
}

fn projected_exit_cell_rect(
    room: &Room,
    world_pos: Vec2,
    direction: ExitDirection,
    grid_size: f32,
) -> Rect {
    let local_pos = world_pos - room.position / grid_size;
    let width = room.size.x * grid_size;
    let height = room.size.y * grid_size;

    match direction {
        ExitDirection::Up => Rect::new(
            room.position.x + local_pos.x * grid_size,
            room.position.y,
            grid_size,
            grid_size,
        ),
        ExitDirection::Down => Rect::new(
            room.position.x + local_pos.x * grid_size,
            room.position.y + height - grid_size,
            grid_size,
            grid_size,
        ),
        ExitDirection::Left => Rect::new(
            room.position.x,
            room.position.y + local_pos.y * grid_size,
            grid_size,
            grid_size,
        ),
        ExitDirection::Right => Rect::new(
            room.position.x + width - grid_size,
            room.position.y + local_pos.y * grid_size,
            grid_size,
            grid_size,
        ),
    }
}

#[cfg(test)]
#[path = "helpers_tests.rs"]
mod helpers_tests;
