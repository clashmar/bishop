use super::alt_copy::enter_alt_copy_mode;
use super::drag_motion::begin_entity_drag;
use crate::gui::inspector::collider_module::edit::is_collider_edit_active_for;
use crate::gui::inspector::interactable_module::edit::is_interactable_edit_active_for;
use crate::room::collider_drag::{try_intercept_collider_handle, try_start_collider_handle_on_click};
use crate::room::interactable_drag::{
    try_intercept_interactable_handle,
    try_start_interactable_handle_on_click,
};
use crate::room::room_editor::RoomEditor;
use crate::room::selection::{
    entity_hitbox,
    topmost_entity_from_click_candidates,
};
use crate::world::coord;
use bishop::prelude::{Camera2D, KeyCode, MouseButton, Vec2, WgpuContext};
use bishop::Input;
use engine_core::assets::SpriteManager;
use engine_core::ecs::{Ecs, Entity, Layer, RoomCamera, Transform};
use engine_core::rendering::resolve_visual_entity;
use engine_core::worlds::RoomId;

/// Applies click selection rules for the room editor.
pub(crate) fn apply_click_selection(
    editor: &mut RoomEditor,
    entity: Entity,
    shift_held: bool,
) -> bool {
    if shift_held {
        editor.toggle_entity_selection(entity);
        return false;
    }

    if !editor.selected_entities.contains(&entity) {
        editor.set_selected_entity(Some(entity));
    }
    true
}

/// Starts a room pointer interaction when the left mouse button is pressed.
pub(crate) fn try_begin_pointer_interaction(
    editor: &mut RoomEditor,
    ctx: &mut WgpuContext,
    room_id: RoomId,
    camera: &Camera2D,
    ecs: &mut Ecs,
    sprite_manager: &mut SpriteManager,
    grid_size: f32,
) -> bool {
    if editor.ui_was_clicked(ctx)
        || !ctx.is_mouse_button_pressed(MouseButton::Left)
        || editor.drag_state.entity_drag.dragging
        || editor.drag_state.box_select_active
    {
        return false;
    }

    let mouse_screen: Vec2 = ctx.mouse_position().into();
    let mouse_world = coord::mouse_world_pos(ctx, camera);
    let shift_held = ctx.is_key_down(KeyCode::LeftShift) || ctx.is_key_down(KeyCode::RightShift);

    if let Some((entity, action, interactable)) = try_intercept_interactable_handle(
        editor.single_selected_entity(),
        ecs,
        mouse_world,
        grid_size,
    ) {
        let transform_entity = editor.single_selected_entity().unwrap_or(entity);
        editor.drag_state.interactable_drag.begin(
            entity,
            transform_entity,
            action,
            interactable,
            mouse_world,
        );
        return true;
    }

    if let Some((visual_entity, action, collider)) = try_intercept_collider_handle(
        editor.single_selected_entity(),
        ecs,
        mouse_world,
        grid_size,
    ) {
        let transform_entity = editor.single_selected_entity().unwrap_or(visual_entity);
        editor.drag_state.collider_drag.begin(
            visual_entity,
            transform_entity,
            action,
            collider,
            mouse_world,
        );
        return true;
    }

    let active_layer = editor.active_layer_state.active_layer;
    let layer_store = ecs.get_store::<Layer>();
    let camera_store = ecs.get_store::<RoomCamera>();
    let mut candidates = Vec::new();
    for &entity in ecs.entities_in_room_layer(room_id, active_layer) {
        let Some(transform) = ecs.get::<Transform>(entity) else {
            continue;
        };
        let hitbox = entity_hitbox(
            ctx,
            entity,
            transform.position,
            camera,
            ecs,
            sprite_manager,
            grid_size,
        );
        if hitbox.contains(mouse_screen) {
            let z = layer_store.get(entity).map_or(0, |layer| layer.z);
            let is_camera = camera_store.get(entity).is_some();
            candidates.push((entity, z, is_camera));
        }
    }

    if let Some(entity) = topmost_entity_from_click_candidates(&candidates) {
        let should_start_drag = apply_click_selection(editor, entity, shift_held);
        if !should_start_drag {
            return true;
        }

        if editor.single_selected_entity().is_some_and(is_interactable_edit_active_for) {
            if let Some((interactable_entity, action, interactable)) = try_start_interactable_handle_on_click(
                entity,
                ecs,
                mouse_world,
                grid_size,
            ) {
                editor.drag_state.interactable_drag.begin(
                    interactable_entity,
                    entity,
                    action,
                    interactable,
                    mouse_world,
                );
                return true;
            }
        } else if editor
            .single_selected_entity()
            .is_some_and(|selected| is_collider_edit_active_for(resolve_visual_entity(ecs, selected)))
        {
            if let Some((visual_entity, action, collider)) = try_start_collider_handle_on_click(
                entity,
                ecs,
                mouse_world,
                grid_size,
            ) {
                editor.drag_state.collider_drag.begin(
                    visual_entity,
                    entity,
                    action,
                    collider,
                    mouse_world,
                );
                return true;
            }
        }

        begin_entity_drag(
            &mut editor.drag_state.entity_drag,
            &editor.selected_entities,
            entity,
            ecs,
            mouse_world,
        );
        if ctx.is_key_down(KeyCode::LeftAlt) || ctx.is_key_down(KeyCode::RightAlt) {
            enter_alt_copy_mode(editor, ecs, room_id);
        }
        return true;
    }

    if !shift_held {
        editor.clear_selection();
    }
    editor.drag_state.box_select_start = Some(mouse_world);
    editor.drag_state.box_select_active = true;
    true
}
