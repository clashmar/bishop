use super::*;

#[test]
fn staged_prefab_edits_preview_sync_to_linked_room_instances() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_preview_sync");
    let (editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let prefab_editor = editor.prefab_editor.as_mut().unwrap();
        let root = prefab_editor.root_entity.expect("prefab root should exist");
        let child = prefab_editor
            .create_prefab_entity(&mut editor.prefab_stage.as_mut().unwrap().ecs, Some(root));
        editor
            .prefab_stage
            .as_mut()
            .unwrap()
            .ecs
            .add_component_to_entity(child, Name("Preview Child".to_string()));
        editor.reconcile_active_prefab_room_preview();
    });

    with_editor(|editor| {
        let linked_roots = linked_root_entities(&editor.game.ecs, prefab_id);
        assert_eq!(linked_roots.len(), 1);
        assert_eq!(linked_instance_node_count(&editor.game.ecs, prefab_id), 2);
        assert_eq!(
            editor
                .game
                .ecs
                .get::<CurrentRoom>(linked_roots[0])
                .map(|room| room.0),
            Some(room_id)
        );
    });
}

#[test]
fn empty_prefab_preview_delete_and_undo_restore_room_instances() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_empty_preview_undo");
    let (editor, _, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
        push_command(Box::new(DeleteEntityCmd::new(
            root,
            EditorMode::Prefab(prefab_id),
        )));
    });
    apply_pending_commands();

    with_editor(|editor| {
        editor.reconcile_active_prefab_room_preview();
        assert!(linked_root_entities(&editor.game.ecs, prefab_id).is_empty());
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        editor.reconcile_active_prefab_room_preview();
        assert!(editor.prefab_editor.as_ref().unwrap().root_entity.is_some());
        assert_eq!(linked_root_entities(&editor.game.ecs, prefab_id).len(), 1);
    });
}

#[test]
fn discarding_empty_prefab_exit_restores_committed_room_state() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_empty_exit_discard");
    let (editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
        push_command(Box::new(DeleteEntityCmd::new(
            root,
            EditorMode::Prefab(prefab_id),
        )));
    });
    apply_pending_commands();

    with_editor(|editor| {
        editor.reconcile_active_prefab_room_preview();
        assert!(linked_root_entities(&editor.game.ecs, prefab_id).is_empty());
        assert_eq!(
            editor.request_prefab_transition(PendingPrefabTransition::Exit),
            PrefabTransitionPrompt::Empty
        );
        editor.confirm_empty_prefab_transition(EmptyPrefabExitPromptResult::DiscardChanges);
        assert_eq!(editor.mode, EditorMode::Room(room_id));
        assert!(editor.prefab_editor.is_none());
        assert_eq!(linked_root_entities(&editor.game.ecs, prefab_id).len(), 1);
    });
}
