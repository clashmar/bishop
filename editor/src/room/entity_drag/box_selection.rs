use crate::room::room_editor::RoomEditor;
use crate::room::selection::entity_world_rect;
use crate::shared::selection::{box_selection_drag_started, rect_from_two_points, rects_intersect};
use crate::world::coord;
use bishop::prelude::{Camera2D, MouseButton, Rect, Vec2, WgpuContext};
use bishop::Input;
use engine_core::assets::SpriteManager;
use engine_core::ecs::{Ecs, Entity, Transform};
use engine_core::worlds::{RoomId, RoomLayer};
use std::collections::HashSet;

/// Finishes an active box selection when the left mouse button is released.
pub(crate) fn finish_box_selection(
    editor: &mut RoomEditor,
    ctx: &WgpuContext,
    camera: &Camera2D,
    room_id: RoomId,
    ecs: &Ecs,
    sprite_manager: &mut SpriteManager,
    grid_size: f32,
) -> bool {
    if !editor.drag_state.box_select_active {
        return false;
    }
    if !ctx.is_mouse_button_released(MouseButton::Left) {
        return true;
    }

    let Some(start) = editor.drag_state.box_select_start.take() else {
        editor.drag_state.box_select_active = false;
        return true;
    };
    editor.drag_state.box_select_active = false;

    let start_screen = coord::world_to_screen(ctx, camera, start);
    let mouse_screen: Vec2 = ctx.mouse_position().into();
    if !box_selection_drag_started(start_screen, mouse_screen) {
        return true;
    }

    let box_rect = rect_from_two_points(start, coord::mouse_world_pos(ctx, camera));
    let selected = collect_box_selected_entities(
        ecs,
        room_id,
        editor.active_layer_state.active_layer,
        box_rect,
        sprite_manager,
        grid_size,
    );
    if !selected.is_empty() {
        editor.disable_active_edit_modes();
        for entity in selected {
            editor.selected_entities.insert(entity);
        }
        editor.sync_inspector_to_selection();
    }
    true
}

/// Collects entities intersecting the finished box selection.
pub(crate) fn collect_box_selected_entities(
    ecs: &Ecs,
    room_id: RoomId,
    layer: RoomLayer,
    box_rect: Rect,
    sprite_manager: &mut SpriteManager,
    grid_size: f32,
) -> HashSet<Entity> {
    let mut selected = HashSet::new();
    for &entity in ecs.entities_in_room_layer(room_id, layer) {
        let Some(transform) = ecs.get::<Transform>(entity) else {
            continue;
        };
        let entity_rect = entity_world_rect(
            entity,
            transform.position,
            ecs,
            sprite_manager,
            grid_size,
        );
        if rects_intersect(box_rect, entity_rect) {
            selected.insert(entity);
        }
    }
    selected
}
