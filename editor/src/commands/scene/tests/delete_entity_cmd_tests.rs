use super::*;

#[test]
fn deleting_prefab_root_clears_root_and_selection_state() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_delete_root");
    set_game_name(test_game.name());
    let mut editor = Editor {
        mode: EditorMode::Prefab(PrefabId(9)),
        prefab_editor: Some(PrefabEditor::new(
            PrefabId(9),
            "Prefab".to_string(),
            StagedPrefabState::Empty,
            PrefabRoomSyncState {
                staged_prefab: StagedPrefabState::Empty,
                linked_instance_snapshots: Vec::new(),
            },
        )),
        prefab_stage: Some(PrefabStage::new(test_game.name())),
        ..Default::default()
    };

    let root = editor
        .prefab_stage
        .as_mut()
        .unwrap()
        .ecs
        .create_entity()
        .with(Transform::default())
        .with(Name("Root".to_string()))
        .finish();
    let child = editor
        .prefab_stage
        .as_mut()
        .unwrap()
        .ecs
        .create_entity()
        .with(Transform::default())
        .with(Name("Child".to_string()))
        .finish();
    set_parent(&mut editor.prefab_stage.as_mut().unwrap().ecs, child, root);

    let prefab_editor = editor.prefab_editor.as_mut().unwrap();
    prefab_editor.root_entity = Some(root);
    prefab_editor.selected_entities.insert(root);
    prefab_editor.selected_entities.insert(child);

    let _guard = EditorServicesGuard::install(editor);

    let mut cmd = DeleteEntityCmd::new(root, EditorMode::Prefab(PrefabId(9)));
    cmd.execute();

    with_editor(|editor| {
        let prefab_editor = editor.prefab_editor.as_ref().unwrap();
        assert_eq!(prefab_editor.root_entity, None);
        assert!(prefab_editor.selected_entities.is_empty());
    });
}
