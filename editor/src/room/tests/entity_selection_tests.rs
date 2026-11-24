use super::super::*;
use crate::room::selection::topmost_entity_from_click_candidates;

#[test]
fn pruning_selection_in_active_layer_keeps_existing_entity_inspector() {
    let mut editor = RoomEditor::new();
    let mut ecs = Ecs::default();
    let room_id = RoomId(7);
    let entity = ecs
        .create_entity()
        .with(Transform::default())
        .with(Name("Entity".to_string()))
        .with_current_room_layer(room_id, RoomLayer::Front)
        .finish();

    editor.set_selected_entity(Some(entity));
    editor.active_layer_state.active_layer = RoomLayer::Front;
    let first_addr = editor.inspector.entity_inspector_addr();

    editor.prune_selection_to_active_layer(&ecs, room_id);

    assert_eq!(editor.single_selected_entity(), Some(entity));
    assert_eq!(editor.inspector.selected_entity(), Some(entity));
    assert_eq!(editor.inspector.entity_inspector_addr(), first_addr);
}

#[test]
fn pruning_selection_keeps_global_entities_without_current_room() {
    let mut editor = RoomEditor::new();
    let mut ecs = Ecs::default();
    let room_id = RoomId(7);
    let global_entity = ecs
        .create_entity()
        .with(Global::default())
        .with(Name("Global Entity".to_string()))
        .finish();

    editor.set_selected_entity(Some(global_entity));
    editor.active_layer_state.active_layer = RoomLayer::Front;

    editor.prune_selection_to_active_layer(&ecs, room_id);

    assert_eq!(editor.single_selected_entity(), Some(global_entity));
    assert_eq!(editor.inspector.selected_entity(), Some(global_entity));
}

#[test]
fn multi_selection_keeps_primary_entity_in_inspector() {
    let mut editor = RoomEditor::new();
    let primary = Entity(4);
    let other = Entity(9);

    editor.set_selected_entity(Some(primary));
    editor.add_to_selection(other);

    assert_eq!(editor.selected_entities.len(), 2);
    assert_eq!(editor.inspector.selected_entity(), Some(other));
    assert_eq!(editor.single_selected_entity(), None);
}

#[test]
fn overlapping_entity_selection_same_z_prefers_higher_entity_id() {
    let ecs = Ecs::default();
    let lower = Entity(4);
    let higher = Entity(9);

    let selected = topmost_entity_from_click_candidates(&ecs, &[
        (lower, 3, false),
        (higher, 3, false),
    ]);

    assert_eq!(selected, Some(higher));
}

#[test]
fn overlapping_entity_selection_camera_wins_over_non_camera() {
    let ecs = Ecs::default();
    let camera = Entity(4);
    let other = Entity(99);

    let selected = topmost_entity_from_click_candidates(&ecs, &[
        (other, 50, false),
        (camera, -10, true),
    ]);

    assert_eq!(selected, Some(camera));
}
