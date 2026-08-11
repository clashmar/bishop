use crate::app::SubEditor;
use crate::commands::room::UpdateInteriorZonesCmd;
use crate::editor_global::push_command;
use crate::room::room_editor::{RoomEditor, RoomEditorMode, RoomSceneSubMode};
use crate::world::coord;
use bishop::prelude::*;
use engine_core::theme::with_theme;
use engine_core::worlds::{InteriorZone, InteriorZoneBounds, InteriorZoneId, Room, WorldId};
use widgets::constants::layout;
use widgets::MouseButton;

const HANDLE_SIZE_FACTOR: f32 = 0.5;
const FILL_ALPHA: f32 = 0.18;
const SELECTED_FILL_ALPHA: f32 = 0.28;
const OUTLINE_ALPHA: f32 = 0.85;
const ZONE_LABEL_BASE_FONT_SIZE: f32 = layout::FIELD_TEXT_SIZE_16;
const ZONE_LABEL_MAX_FONT_SIZE: f32 = 128.0;
const ZONE_LABEL_MIN_FONT_SIZE: f32 = 12.0;
const ZONE_LABEL_TARGET_HEIGHT_RATIO: f32 = 0.5;
const ZONE_LABEL_TARGET_WIDTH_RATIO: f32 = 0.5;
const ZONE_LABEL_TOP_PADDING: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ZoneHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

#[derive(Clone, Debug)]
enum ZoneInteraction {
    Creating {
        zone_id: InteriorZoneId,
        anchor: Vec2,
        old_zones: Vec<InteriorZone>,
    },
    Moving {
        zone_id: InteriorZoneId,
        drag_start: Vec2,
        original_bounds: Rect,
        old_zones: Vec<InteriorZone>,
    },
    Resizing {
        zone_id: InteriorZoneId,
        handle: ZoneHandle,
        original_bounds: Rect,
        old_zones: Vec<InteriorZone>,
    },
}

#[derive(Default)]
pub(crate) struct InteriorZoneEditorState {
    pub(crate) selected_zone_id: Option<InteriorZoneId>,
    interaction: Option<ZoneInteraction>,
}

impl InteriorZoneEditorState {
    pub(crate) fn clear(&mut self) {
        self.selected_zone_id = None;
        self.interaction = None;
    }

    fn sync_selection(&mut self, zones: &[InteriorZone]) {
        if self
            .selected_zone_id
            .is_some_and(|id| zones.iter().all(|zone| zone.id != id))
        {
            self.selected_zone_id = None;
        }
    }

    fn is_active(&self) -> bool {
        self.interaction.is_some()
    }
}

impl RoomEditor {
    pub(crate) fn handle_interior_zones(
        &mut self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        world_id: WorldId,
        room: &mut Room,
        grid_size: f32,
    ) -> bool {
        if self.mode != RoomEditorMode::Scene
            || self.scene_sub_mode != RoomSceneSubMode::Zones
        {
            return false;
        }

        let room_id = room.id;
        let room_rect = room.world_rect(grid_size);
        let Some(back) = room.current_variant_mut().layers.back.as_mut() else {
            self.interior_zone_editor.clear();
            return false;
        };

        self.interior_zone_editor.sync_selection(&back.interior_zones);
        let mouse_world = coord::mouse_world_pos(ctx, camera);

        if let Some((old_zones, changed)) = step_zone_interaction(
            &mut self.interior_zone_editor,
            &mut back.interior_zones,
            room_rect,
            grid_size,
            mouse_world,
            ctx.is_mouse_button_down(MouseButton::Left),
            ctx.is_mouse_button_released(MouseButton::Left),
        ) {
            if changed {
                push_command(Box::new(UpdateInteriorZonesCmd::new(
                    world_id,
                    room_id,
                    old_zones,
                    back.interior_zones.clone(),
                )));
            }
            return true;
        }

        if self.should_block_canvas(ctx) {
            return self.interior_zone_editor.is_active();
        }

        if ctx.is_mouse_button_pressed(MouseButton::Left) {
            begin_zone_interaction(
                &mut self.interior_zone_editor,
                &mut back.interior_zones,
                room_rect,
                grid_size,
                mouse_world,
            );
            return true;
        }

        false
    }

    pub(crate) fn delete_selected_interior_zone(
        &mut self,
        room: &mut Room,
        world_id: WorldId,
    ) -> bool {
        let Some(selected_zone_id) = self.interior_zone_editor.selected_zone_id else {
            return false;
        };
        let room_id = room.id;
        let Some(back) = room.current_variant_mut().layers.back.as_mut() else {
            self.interior_zone_editor.clear();
            return false;
        };
        let Some(index) = zone_index(back.interior_zones.as_slice(), selected_zone_id) else {
            self.interior_zone_editor.clear();
            return false;
        };

        let old_zones = back.interior_zones.clone();
        back.interior_zones.remove(index);
        self.interior_zone_editor.selected_zone_id = None;
        push_command(Box::new(UpdateInteriorZonesCmd::new(
            world_id,
            room_id,
            old_zones,
            back.interior_zones.clone(),
        )));
        true
    }

    pub(crate) fn draw_interior_zones_overlay(
        &self,
        ctx: &mut WgpuContext,
        camera: &Camera2D,
        room: &Room,
        grid_size: f32,
    ) {
        let Some(back) = room.current_variant().layers.back.as_ref() else {
            return;
        };

        let zone_mode_active = self.scene_sub_mode == RoomSceneSubMode::Zones;
        let mut labels = Vec::new();
        for zone in &back.interior_zones {
            let bounds = zone.bounds.to_rect();
            let selected = self.interior_zone_editor.selected_zone_id == Some(zone.id);
            let fill = if selected {
                with_theme(|theme| theme.highlight.with_alpha(SELECTED_FILL_ALPHA))
            } else {
                with_theme(|theme| theme.accent.with_alpha(FILL_ALPHA))
            };
            let outline = if selected {
                with_theme(|theme| theme.highlight.with_alpha(OUTLINE_ALPHA))
            } else {
                with_theme(|theme| theme.accent.with_alpha(OUTLINE_ALPHA))
            };

            ctx.draw_rectangle(
                bounds.x,
                bounds.y,
                bounds.w,
                bounds.h,
                fill,
            );
            ctx.draw_rectangle_lines(
                bounds.x,
                bounds.y,
                bounds.w,
                bounds.h,
                outline_thickness(grid_size),
                outline,
            );

            let label = format!("Zone {}", zone.id.0);
            let screen_top_left = coord::world_to_screen(ctx, camera, vec2(bounds.x, bounds.y));
            let screen_top_right = coord::world_to_screen(ctx, camera, vec2(bounds.x + bounds.w, bounds.y));
            let screen_bottom_left = coord::world_to_screen(ctx, camera, vec2(bounds.x, bounds.y + bounds.h));
            let screen_zone_width = (screen_top_right.x - screen_top_left.x).abs();
            let screen_zone_height = (screen_bottom_left.y - screen_top_left.y).abs();
            let base_dims = ctx.measure_text(&label, ZONE_LABEL_BASE_FONT_SIZE);
            if base_dims.width > 0.0 && base_dims.height > 0.0 {
                let target_width = screen_zone_width * ZONE_LABEL_TARGET_WIDTH_RATIO;
                let target_height = screen_zone_height * ZONE_LABEL_TARGET_HEIGHT_RATIO;
                let width_scale = target_width / base_dims.width;
                let height_scale = target_height / base_dims.height;
                let raw_font_size = (ZONE_LABEL_BASE_FONT_SIZE * width_scale.min(height_scale))
                    .clamp(ZONE_LABEL_MIN_FONT_SIZE, ZONE_LABEL_MAX_FONT_SIZE);
                let snapped_font_size = snap_font_size(raw_font_size);
                let snapped_dims = ctx.measure_text(&label, snapped_font_size);
                let font_scale = if snapped_dims.width > 0.0 && snapped_dims.height > 0.0 {
                    (target_width / snapped_dims.width)
                        .min(target_height / snapped_dims.height)
                        .max(0.1)
                } else {
                    1.0
                };
                let scaled_width = snapped_dims.width * font_scale;
                let scaled_height = snapped_dims.height * font_scale;
                let x = screen_top_left.x + (screen_zone_width - scaled_width) * 0.5;
                let y = screen_top_left.y + ZONE_LABEL_TOP_PADDING + scaled_height;
                labels.push((
                    label,
                    x,
                    y,
                    TextParams {
                        font_size: snapped_font_size as u16,
                        font_scale,
                        color: outline,
                        ..Default::default()
                    },
                ));
            }

            if zone_mode_active && selected {
                for (handle_rect, _) in handle_rects(bounds, grid_size) {
                    ctx.draw_rectangle(
                        handle_rect.x,
                        handle_rect.y,
                        handle_rect.w,
                        handle_rect.h,
                        with_theme(|theme| theme.highlight),
                    );
                }
            }
        }

        ctx.set_default_camera();
        for (label, x, y, params) in labels {
            ctx.draw_text_ex(&label, x, y, params);
        }
        ctx.set_camera(camera);
    }
}

fn step_zone_interaction(
    state: &mut InteriorZoneEditorState,
    zones: &mut Vec<InteriorZone>,
    room_rect: Rect,
    grid_size: f32,
    mouse_world: Vec2,
    mouse_down: bool,
    mouse_released: bool,
) -> Option<(Vec<InteriorZone>, bool)> {
    let interaction = state.interaction.clone()?;
    if mouse_down {
        match &interaction {
            ZoneInteraction::Creating {
                zone_id,
                anchor,
                ..
            } => {
                if let Some(zone) = zone_by_id_mut(zones, *zone_id) {
                    zone.bounds = InteriorZoneBounds::from_rect(dragged_creation_rect(
                        *anchor,
                        mouse_world,
                        room_rect,
                        grid_size,
                    ));
                }
            }
            ZoneInteraction::Moving {
                zone_id,
                drag_start,
                original_bounds,
                ..
            } => {
                if let Some(zone) = zone_by_id_mut(zones, *zone_id) {
                    zone.bounds = InteriorZoneBounds::from_rect(move_zone_rect(
                        *original_bounds,
                        mouse_world - *drag_start,
                        room_rect,
                        grid_size,
                    ));
                }
            }
            ZoneInteraction::Resizing {
                zone_id,
                handle,
                original_bounds,
                ..
            } => {
                if let Some(zone) = zone_by_id_mut(zones, *zone_id) {
                    zone.bounds = InteriorZoneBounds::from_rect(resize_zone_rect(
                        *original_bounds,
                        *handle,
                        mouse_world,
                        room_rect,
                        grid_size,
                    ));
                }
            }
        }
    }

    if !mouse_released {
        return None;
    }

    state.interaction = None;
    let old_zones = match interaction {
        ZoneInteraction::Creating { old_zones, .. }
        | ZoneInteraction::Moving { old_zones, .. }
        | ZoneInteraction::Resizing { old_zones, .. } => old_zones,
    };
    let changed = old_zones != *zones;
    Some((old_zones, changed))
}

fn begin_zone_interaction(
    state: &mut InteriorZoneEditorState,
    zones: &mut Vec<InteriorZone>,
    room_rect: Rect,
    grid_size: f32,
    mouse_world: Vec2,
) {
    let mouse_world = clamp_point_to_room(mouse_world, room_rect);

    if let Some(selected_zone_id) = state.selected_zone_id {
        if let Some(selected_zone) = zone_by_id(zones, selected_zone_id) {
            let selected_bounds = selected_zone.bounds.to_rect();
            if let Some(handle) = hit_test_handle(mouse_world, selected_bounds, grid_size) {
                state.interaction = Some(ZoneInteraction::Resizing {
                    zone_id: selected_zone_id,
                    handle,
                    original_bounds: selected_bounds,
                    old_zones: zones.clone(),
                });
                return;
            }
        }
    }

    if let Some(zone) = zone_at_point(zones, mouse_world) {
        state.selected_zone_id = Some(zone.id);
        state.interaction = Some(ZoneInteraction::Moving {
            zone_id: zone.id,
            drag_start: mouse_world,
            original_bounds: zone.bounds.to_rect(),
            old_zones: zones.clone(),
        });
        return;
    }

    let zone_id = next_interior_zone_id(zones);
    let anchor = snap_point_to_grid(mouse_world, grid_size);
    zones.push(InteriorZone {
        id: zone_id,
        bounds: InteriorZoneBounds::from_rect(Rect::new(anchor.x, anchor.y, grid_size, grid_size)),
    });
    state.selected_zone_id = Some(zone_id);
    state.interaction = Some(ZoneInteraction::Creating {
        zone_id,
        anchor,
        old_zones: zones[..zones.len() - 1].to_vec(),
    });
}

fn zone_index(zones: &[InteriorZone], id: InteriorZoneId) -> Option<usize> {
    zones.iter().position(|zone| zone.id == id)
}

fn zone_by_id(zones: &[InteriorZone], id: InteriorZoneId) -> Option<InteriorZone> {
    zone_index(zones, id).map(|index| zones[index])
}

fn zone_by_id_mut(zones: &mut [InteriorZone], id: InteriorZoneId) -> Option<&mut InteriorZone> {
    let index = zone_index(zones, id)?;
    Some(&mut zones[index])
}

fn zone_at_point(zones: &[InteriorZone], point: Vec2) -> Option<InteriorZone> {
    zones
        .iter()
        .rev()
        .copied()
        .find(|zone| zone.bounds.contains(point))
}

fn next_interior_zone_id(zones: &[InteriorZone]) -> InteriorZoneId {
    InteriorZoneId(
        zones
            .iter()
            .map(|zone| zone.id.0)
            .max()
            .unwrap_or(0)
            .saturating_add(1),
    )
}

fn snap_point_to_grid(point: Vec2, grid_size: f32) -> Vec2 {
    vec2(
        coord::round_to_grid(point.x, grid_size),
        coord::round_to_grid(point.y, grid_size),
    )
}

fn clamp_point_to_room(point: Vec2, room_rect: Rect) -> Vec2 {
    vec2(
        point.x.clamp(room_rect.x, room_rect.right()),
        point.y.clamp(room_rect.y, room_rect.bottom()),
    )
}

fn dragged_creation_rect(anchor: Vec2, mouse_world: Vec2, room_rect: Rect, grid_size: f32) -> Rect {
    let current = snap_point_to_grid(clamp_point_to_room(mouse_world, room_rect), grid_size);
    let mut left = anchor.x.min(current.x);
    let mut right = anchor.x.max(current.x);
    let mut top = anchor.y.min(current.y);
    let mut bottom = anchor.y.max(current.y);

    if (right - left) < grid_size {
        right = (left + grid_size).min(room_rect.right());
        left = (right - grid_size).max(room_rect.x);
    }
    if (bottom - top) < grid_size {
        bottom = (top + grid_size).min(room_rect.bottom());
        top = (bottom - grid_size).max(room_rect.y);
    }

    Rect::new(left, top, right - left, bottom - top)
}

fn move_zone_rect(original: Rect, delta: Vec2, room_rect: Rect, grid_size: f32) -> Rect {
    let snapped_delta = snap_point_to_grid(delta, grid_size);
    let max_x = room_rect.right() - original.w;
    let max_y = room_rect.bottom() - original.h;
    Rect::new(
        (original.x + snapped_delta.x).clamp(room_rect.x, max_x),
        (original.y + snapped_delta.y).clamp(room_rect.y, max_y),
        original.w,
        original.h,
    )
}

fn resize_zone_rect(
    original: Rect,
    handle: ZoneHandle,
    mouse_world: Vec2,
    room_rect: Rect,
    grid_size: f32,
) -> Rect {
    let snapped = snap_point_to_grid(clamp_point_to_room(mouse_world, room_rect), grid_size);
    let mut left = original.left();
    let mut right = original.right();
    let mut top = original.top();
    let mut bottom = original.bottom();

    match handle {
        ZoneHandle::TopLeft => {
            left = snapped.x.clamp(room_rect.x, right - grid_size);
            top = snapped.y.clamp(room_rect.y, bottom - grid_size);
        }
        ZoneHandle::Top => {
            top = snapped.y.clamp(room_rect.y, bottom - grid_size);
        }
        ZoneHandle::TopRight => {
            right = snapped.x.clamp(left + grid_size, room_rect.right());
            top = snapped.y.clamp(room_rect.y, bottom - grid_size);
        }
        ZoneHandle::Right => {
            right = snapped.x.clamp(left + grid_size, room_rect.right());
        }
        ZoneHandle::BottomRight => {
            right = snapped.x.clamp(left + grid_size, room_rect.right());
            bottom = snapped.y.clamp(top + grid_size, room_rect.bottom());
        }
        ZoneHandle::Bottom => {
            bottom = snapped.y.clamp(top + grid_size, room_rect.bottom());
        }
        ZoneHandle::BottomLeft => {
            left = snapped.x.clamp(room_rect.x, right - grid_size);
            bottom = snapped.y.clamp(top + grid_size, room_rect.bottom());
        }
        ZoneHandle::Left => {
            left = snapped.x.clamp(room_rect.x, right - grid_size);
        }
    }

    Rect::new(left, top, right - left, bottom - top)
}

fn handle_rects(bounds: Rect, grid_size: f32) -> [(Rect, ZoneHandle); 8] {
    let handle_size = (grid_size * HANDLE_SIZE_FACTOR).max(6.0);
    let half = handle_size * 0.5;
    let left = bounds.left();
    let right = bounds.right();
    let top = bounds.top();
    let bottom = bounds.bottom();
    let center_x = bounds.x + bounds.w * 0.5;
    let center_y = bounds.y + bounds.h * 0.5;

    [
        (Rect::new(left - half, top - half, handle_size, handle_size), ZoneHandle::TopLeft),
        (Rect::new(center_x - half, top - half, handle_size, handle_size), ZoneHandle::Top),
        (Rect::new(right - half, top - half, handle_size, handle_size), ZoneHandle::TopRight),
        (Rect::new(right - half, center_y - half, handle_size, handle_size), ZoneHandle::Right),
        (
            Rect::new(right - half, bottom - half, handle_size, handle_size),
            ZoneHandle::BottomRight,
        ),
        (
            Rect::new(center_x - half, bottom - half, handle_size, handle_size),
            ZoneHandle::Bottom,
        ),
        (
            Rect::new(left - half, bottom - half, handle_size, handle_size),
            ZoneHandle::BottomLeft,
        ),
        (Rect::new(left - half, center_y - half, handle_size, handle_size), ZoneHandle::Left),
    ]
}

fn hit_test_handle(point: Vec2, bounds: Rect, grid_size: f32) -> Option<ZoneHandle> {
    handle_rects(bounds, grid_size)
        .into_iter()
        .find(|(rect, _)| rect.contains(point))
        .map(|(_, handle)| handle)
}

fn outline_thickness(grid_size: f32) -> f32 {
    (grid_size / 16.0).max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragged_creation_rect_clamps_to_room_and_enforces_min_size() {
        let room = Rect::new(32.0, 48.0, 64.0, 64.0);
        let rect = dragged_creation_rect(vec2(40.0, 56.0), vec2(30.0, 50.0), room, 16.0);

        assert_eq!(rect, Rect::new(32.0, 48.0, 16.0, 16.0));
    }

    #[test]
    fn move_zone_rect_stays_inside_room() {
        let room = Rect::new(32.0, 48.0, 64.0, 64.0);
        let moved = move_zone_rect(Rect::new(32.0, 48.0, 32.0, 32.0), vec2(-40.0, 80.0), room, 16.0);

        assert_eq!(moved, Rect::new(32.0, 80.0, 32.0, 32.0));
    }

    #[test]
    fn resize_zone_rect_respects_min_size_and_room_bounds() {
        let room = Rect::new(32.0, 48.0, 96.0, 96.0);
        let resized = resize_zone_rect(
            Rect::new(64.0, 80.0, 32.0, 32.0),
            ZoneHandle::TopLeft,
            vec2(0.0, 0.0),
            room,
            16.0,
        );

        assert_eq!(resized, Rect::new(32.0, 48.0, 64.0, 64.0));
    }

    #[test]
    fn hit_test_handle_prefers_visible_handles() {
        let bounds = Rect::new(64.0, 80.0, 32.0, 32.0);
        let point = vec2(64.0, 80.0);

        assert_eq!(hit_test_handle(point, bounds, 16.0), Some(ZoneHandle::TopLeft));
    }
}
