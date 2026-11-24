use super::super::*;
use crate::gui::inspector::animation_module::frame_edit;
use crate::gui::inspector::collider_module::edit::{clear_collider_edit, collider_edit_entity, toggle_collider_edit};
use crate::gui::inspector::interactable_module::edit::{clear_interactable_edit, interactable_edit_entity, toggle_interactable_edit};
use engine_core::animation::{ClipDef, ClipId};
use std::collections::HashMap;

fn frame_edit_animation() -> Animation {
    Animation {
        current: Some(ClipId::Run),
        clips: HashMap::from([(ClipId::Run, ClipDef::default())]),
        ..Default::default()
    }
}

#[test]
fn clearing_room_selection_disables_collider_edit_mode() {
    let mut editor = RoomEditor::new();
    let entity = Entity(7);
    clear_collider_edit(entity);
    editor.set_selected_entity(Some(entity));
    assert!(toggle_collider_edit(entity));

    editor.clear_selection();

    assert_eq!(collider_edit_entity(), None);
}

#[test]
fn toggling_last_room_selection_off_disables_collider_edit_mode() {
    let mut editor = RoomEditor::new();
    let entity = Entity(7);
    clear_collider_edit(entity);
    editor.set_selected_entity(Some(entity));
    assert!(toggle_collider_edit(entity));

    editor.toggle_entity_selection(entity);

    assert_eq!(collider_edit_entity(), None);
}

#[test]
fn selecting_different_room_entity_disables_previous_collider_edit_mode() {
    let mut editor = RoomEditor::new();
    let first = Entity(7);
    let second = Entity(8);
    clear_collider_edit(first);
    clear_collider_edit(second);
    editor.set_selected_entity(Some(first));
    assert!(toggle_collider_edit(first));

    editor.set_selected_entity(Some(second));

    assert_eq!(collider_edit_entity(), None);
}

#[test]
fn clearing_room_selection_exits_animation_frame_edit_mode() {
    let mut editor = RoomEditor::new();
    let entity = Entity(7);
    let animation = frame_edit_animation();
    editor.set_selected_entity(Some(entity));
    frame_edit::enter(entity, &animation);

    editor.clear_selection();

    assert!(frame_edit::active_target(entity).is_none());
}

#[test]
fn selecting_different_room_entity_exits_animation_frame_edit_mode() {
    let mut editor = RoomEditor::new();
    let first = Entity(7);
    let second = Entity(8);
    let animation = frame_edit_animation();
    editor.set_selected_entity(Some(first));
    frame_edit::enter(first, &animation);

    editor.set_selected_entity(Some(second));

    assert!(frame_edit::active_target(first).is_none());
}

#[test]
fn switching_room_editor_mode_exits_animation_frame_edit_mode() {
    let mut editor = RoomEditor::new();
    let entity = Entity(7);
    let animation = frame_edit_animation();
    editor.set_selected_entity(Some(entity));
    frame_edit::enter(entity, &animation);

    editor.set_mode(RoomEditorMode::Tilemap);

    assert!(frame_edit::active_target(entity).is_none());
}

#[test]
fn resetting_room_editor_exits_animation_frame_edit_mode() {
    let mut editor = RoomEditor::new();
    let entity = Entity(7);
    let animation = frame_edit_animation();
    editor.set_selected_entity(Some(entity));
    frame_edit::enter(entity, &animation);

    editor.reset();

    assert!(frame_edit::active_target(entity).is_none());
}

#[test]
fn clearing_room_selection_disables_interactable_edit_mode() {
    let mut editor = RoomEditor::new();
    let entity = Entity(7);
    clear_interactable_edit(entity);
    editor.set_selected_entity(Some(entity));
    assert!(toggle_interactable_edit(entity));

    editor.clear_selection();

    assert_eq!(interactable_edit_entity(), None);
}

#[test]
fn selecting_different_room_entity_disables_previous_interactable_edit_mode() {
    let mut editor = RoomEditor::new();
    let first = Entity(7);
    let second = Entity(8);
    clear_interactable_edit(first);
    clear_interactable_edit(second);
    editor.set_selected_entity(Some(first));
    assert!(toggle_interactable_edit(first));

    editor.set_selected_entity(Some(second));

    assert_eq!(interactable_edit_entity(), None);
}
