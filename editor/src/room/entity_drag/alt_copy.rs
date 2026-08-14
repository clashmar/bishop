use super::state::PreCopyDragState;
use crate::room::room_editor::RoomEditor;
use bishop::prelude::Vec2;
use engine_core::ecs::{Ecs, Transform, update_entity_position};
use engine_core::worlds::RoomId;

/// Enters alt-copy mode for the active entity drag.
pub(crate) fn enter_alt_copy_mode(
    editor: &mut RoomEditor,
    ecs: &mut Ecs,
    room_id: RoomId,
) -> bool {
    let pre_copy_drag_state = PreCopyDragState {
        anchor_entity: editor.drag_state.entity_drag.anchor_entity,
        selected_entities: editor.selected_entities.clone(),
    };
    let current_positions: Vec<_> = editor
        .drag_state
        .entity_drag
        .drag_start_positions
        .iter()
        .filter_map(|(entity, _)| {
            ecs.get::<Transform>(*entity)
                .map(|transform| (*entity, transform.position))
        })
        .collect();
    let duplicates = editor.duplicate_entities_for_drag(ecs, room_id);
    if duplicates.is_empty() {
        return false;
    }

    for (original, duplicate) in &duplicates {
        if let Some((_, position)) = current_positions.iter().find(|(entity, _)| entity == original)
        {
            update_entity_position(ecs, *duplicate, *position);
        }
    }
    for (entity, initial_position) in &editor.drag_state.entity_drag.drag_initial_start_positions {
        update_entity_position(ecs, *entity, *initial_position);
    }

    let new_anchor = editor
        .drag_state
        .entity_drag
        .anchor_entity
        .and_then(|anchor| duplicates.iter().find(|(original, _)| *original == anchor))
        .map(|(_, duplicate)| *duplicate)
        .unwrap_or(duplicates[0].1);
    let drag_start_positions = duplicates
        .iter()
        .filter_map(|(original, duplicate)| {
            current_positions
                .iter()
                .find(|(entity, _)| entity == original)
                .map(|(_, position)| (*duplicate, *position))
        })
        .collect();

    editor.disable_active_edit_modes();
    editor.selected_entities.clear();
    for (_, duplicate) in &duplicates {
        editor.selected_entities.insert(*duplicate);
    }
    editor.drag_state.entity_drag.pre_copy_drag_state = Some(pre_copy_drag_state);
    editor.drag_state.entity_drag.drag_start_positions = drag_start_positions;
    editor.drag_state.entity_drag.anchor_entity = Some(new_anchor);
    editor.drag_state.entity_drag.alt_copied_entities =
        duplicates.iter().map(|(_, duplicate)| *duplicate).collect();
    editor.drag_state.entity_drag.alt_copy_pairs = duplicates;
    editor.drag_state.entity_drag.alt_copy_mode = true;
    editor.sync_inspector_to_selection();
    true
}

/// Exits alt-copy mode and restores the original drag selection.
pub(crate) fn exit_alt_copy_mode(
    editor: &mut RoomEditor,
    ecs: &mut Ecs,
    mouse_world: Vec2,
) -> bool {
    let original_state = match editor.drag_state.entity_drag.pre_copy_drag_state.take() {
        Some(state) => state,
        None => return false,
    };
    let copy_pairs = editor.drag_state.entity_drag.alt_copy_pairs.clone();
    let mut copy_positions = Vec::with_capacity(copy_pairs.len());
    for (_, copy) in &copy_pairs {
        let Some(position) = ecs.get::<Transform>(*copy).map(|transform| transform.position) else {
            for (_, duplicate) in &copy_pairs {
                ecs.remove_entity_components(*duplicate);
            }
            editor.selected_entities = original_state.selected_entities;
            editor.drag_state.entity_drag.clear();
            editor.sync_inspector_to_selection();
            return false;
        };
        copy_positions.push((*copy, position));
    }

    for (_, copy) in &copy_pairs {
        ecs.remove_entity_components(*copy);
    }

    editor.selected_entities = original_state.selected_entities;
    editor.drag_state.entity_drag.anchor_entity = original_state.anchor_entity;
    editor.drag_state.entity_drag.drag_start_positions.clear();
    for (original, copy) in &copy_pairs {
        let Some((_, copy_position)) = copy_positions.iter().find(|(entity, _)| entity == copy)
        else {
            editor.drag_state.entity_drag.clear();
            editor.sync_inspector_to_selection();
            return false;
        };
        update_entity_position(ecs, *original, *copy_position);
        editor
            .drag_state
            .entity_drag
            .drag_start_positions
            .push((*original, *copy_position));
    }
    if let Some(anchor) = editor.drag_state.entity_drag.anchor_entity {
        editor.drag_state.entity_drag.drag_offset = ecs
            .get_store::<Transform>()
            .get(anchor)
            .map(|transform| transform.position - mouse_world)
            .unwrap_or(Vec2::ZERO);
    }
    editor.drag_state.entity_drag.alt_copied_entities.clear();
    editor.drag_state.entity_drag.alt_copy_pairs.clear();
    editor.drag_state.entity_drag.alt_copy_mode = false;
    editor.sync_inspector_to_selection();
    true
}
