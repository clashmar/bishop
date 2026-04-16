use super::*;

#[test]
fn prefab_child_keyboard_move_updates_position_and_supports_undo_redo() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_child_keyboard_move");
    let (mut editor, _, prefab_id, _) = make_prefab_session_editor(&test_game);
    let child = add_prefab_child_entity(&mut editor, Vec2::new(12.0, 18.0));
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let stage = editor.prefab_stage.as_mut().unwrap();
        let prefab_editor = editor.prefab_editor.as_mut().unwrap();
        prefab_editor.set_selected_entity(Some(child));
        prefab_editor.move_selected_entities_by(&mut stage.ecs, Vec2::new(1.0, 0.0));
    });
    apply_pending_commands();

    with_editor(|editor| {
        let stage = editor.prefab_stage.as_mut().unwrap();
        assert_eq!(
            stage
                .ecs
                .get::<Transform>(child)
                .map(|transform| transform.position),
            Some(Vec2::new(13.0, 18.0))
        );

        let staged_prefab = editor
            .prefab_editor
            .as_mut()
            .unwrap()
            .staged_prefab_state(&mut stage.ctx_mut());
        let StagedPrefabState::PrefabAsset(prefab) = staged_prefab else {
            panic!("expected prefab asset state");
        };
        let child_node = prefab
            .nodes
            .iter()
            .find(|node| node.node_id != prefab.root_node_id)
            .expect("child node should exist");
        let child_transform = child_node
            .components
            .iter()
            .find(|component| component.type_name == Transform::TYPE_NAME)
            .expect("child transform should be captured");
        assert!(child_transform.ron.contains("position:(13.0,18.0)"));
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        let stage = editor.prefab_stage.as_ref().unwrap();
        assert_eq!(
            stage
                .ecs
                .get::<Transform>(child)
                .map(|transform| transform.position),
            Some(Vec2::new(12.0, 18.0))
        );
    });

    request_redo();
    apply_pending_commands();

    with_editor(|editor| {
        let stage = editor.prefab_stage.as_ref().unwrap();
        assert_eq!(
            stage
                .ecs
                .get::<Transform>(child)
                .map(|transform| transform.position),
            Some(Vec2::new(13.0, 18.0))
        );
        assert_eq!(editor.mode, EditorMode::Prefab(prefab_id));
    });
}

#[test]
fn prefab_root_keyboard_move_is_ignored() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_root_keyboard_move_ignored");
    let (editor, _, _, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    let root_before = with_editor(|editor| {
        let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
        let pos = editor
            .prefab_stage
            .as_ref()
            .unwrap()
            .ecs
            .get::<Transform>(root)
            .expect("prefab root should have transform")
            .position;
        (root, pos)
    });

    with_editor(|editor| {
        let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
        let stage = editor.prefab_stage.as_mut().unwrap();
        let prefab_editor = editor.prefab_editor.as_mut().unwrap();
        prefab_editor.set_selected_entity(Some(root));
        prefab_editor.move_selected_entities_by(&mut stage.ecs, Vec2::new(1.0, 0.0));
    });
    apply_pending_commands();

    with_editor(|editor| {
        let stage = editor.prefab_stage.as_ref().unwrap();
        assert_eq!(
            stage
                .ecs
                .get::<Transform>(root_before.0)
                .map(|transform| transform.position),
            Some(root_before.1)
        );
    });
    with_command_manager(|manager| {
        assert_eq!(manager.undo_stack_len(), 0);
        assert_eq!(manager.redo_stack_len(), 0);
        assert_eq!(manager.pending_len(), 0);
    });
}
