use super::*;
use crate::gui::inspector::collider_module::edit::{
    clear_collider_edit,
    collider_edit_entity,
    toggle_collider_edit,
};

#[test]
fn creating_entity_replaces_stale_root_with_new_root() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_stale_root");
    set_game_name(test_game.name());
    let mut editor = PrefabEditor::new(
        PrefabId(1),
        "Prefab".to_string(),
        StagedPrefabState::Empty,
        PrefabRoomSyncState {
            staged_prefab: StagedPrefabState::Empty,
            linked_instance_snapshots: Vec::new(),
        },
    );
    let mut stage = PrefabStage::new(test_game.name());

    let stale_root = stage
        .ecs
        .create_entity()
        .with(Transform::default())
        .with(Name("Old Root".to_string()))
        .finish();
    editor.root_entity = Some(stale_root);
    editor.set_selected_entity(Some(stale_root));

    {
        stage.with_game_ctx_mut(|ctx| {
            Ecs::remove_entity(ctx, stale_root);
        });
    }

    let new_entity = editor.create_prefab_entity(&mut stage.ecs, None);

    assert_eq!(editor.root_entity, Some(new_entity));
    assert_eq!(get_parent(&stage.ecs, new_entity), None);
}

#[test]
fn selected_create_parent_prefers_inspector_target_during_transient_deselect() {
    let mut editor = PrefabEditor::new(
        PrefabId(1),
        "Prefab".to_string(),
        StagedPrefabState::Empty,
        PrefabRoomSyncState {
            staged_prefab: StagedPrefabState::Empty,
            linked_instance_snapshots: Vec::new(),
        },
    );
    let entity = Entity(22);

    editor.set_selected_entity(Some(entity));
    editor.selected_entities.clear();

    assert_eq!(editor.selected_create_parent(), Some(entity));
}

#[test]
fn clearing_prefab_selection_disables_collider_edit_mode() {
    let mut editor = PrefabEditor::new(
        PrefabId(1),
        "Prefab".to_string(),
        StagedPrefabState::Empty,
        PrefabRoomSyncState {
            staged_prefab: StagedPrefabState::Empty,
            linked_instance_snapshots: Vec::new(),
        },
    );
    let entity = Entity(7);
    clear_collider_edit(entity);
    editor.set_selected_entity(Some(entity));
    assert!(toggle_collider_edit(entity));

    editor.clear_selection();

    assert_eq!(collider_edit_entity(), None);
}

#[test]
fn toggling_last_prefab_selection_off_disables_collider_edit_mode() {
    let mut editor = PrefabEditor::new(
        PrefabId(1),
        "Prefab".to_string(),
        StagedPrefabState::Empty,
        PrefabRoomSyncState {
            staged_prefab: StagedPrefabState::Empty,
            linked_instance_snapshots: Vec::new(),
        },
    );
    let entity = Entity(7);
    clear_collider_edit(entity);
    editor.set_selected_entity(Some(entity));
    assert!(toggle_collider_edit(entity));

    editor.toggle_entity_selection(entity);

    assert_eq!(collider_edit_entity(), None);
}

#[test]
fn selecting_different_prefab_entity_disables_previous_collider_edit_mode() {
    let mut editor = PrefabEditor::new(
        PrefabId(1),
        "Prefab".to_string(),
        StagedPrefabState::Empty,
        PrefabRoomSyncState {
            staged_prefab: StagedPrefabState::Empty,
            linked_instance_snapshots: Vec::new(),
        },
    );
    let first = Entity(7);
    let second = Entity(8);
    clear_collider_edit(first);
    clear_collider_edit(second);
    editor.set_selected_entity(Some(first));
    assert!(toggle_collider_edit(first));

    editor.set_selected_entity(Some(second));

    assert_eq!(collider_edit_entity(), None);
}
