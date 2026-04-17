use super::*;

#[test]
fn clean_prefab_transition_opens_requested_prefab_without_changing_return_mode() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_clean_switch");
    let (mut editor, room_id, _, _) = make_prefab_session_editor(&test_game);
    let second_prefab = save_test_prefab(&test_game, PrefabId(2), "Barrel");
    editor
        .game
        .prefab_library
        .prefabs
        .insert(second_prefab.id, second_prefab.clone());

    assert_eq!(
        editor.request_prefab_transition(PendingPrefabTransition::OpenExisting(second_prefab.id)),
        PrefabTransitionPrompt::None
    );
    assert_eq!(editor.mode, EditorMode::Prefab(second_prefab.id));
    assert_eq!(editor.return_mode, Some(EditorMode::Room(room_id)));
    assert_eq!(
        editor
            .prefab_editor
            .as_ref()
            .map(|prefab| prefab.prefab_name.as_str()),
        Some("Barrel")
    );
}

#[test]
fn requesting_prefab_transition_from_asset_loads_prefab_into_library() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_transition_asset_load");
    let (mut editor, room_id, _, _) = make_prefab_session_editor(&test_game);
    let second_prefab = save_test_prefab(&test_game, PrefabId(2), "Barrel");

    assert!(!editor
        .game
        .prefab_library
        .prefabs
        .contains_key(&second_prefab.id));
    assert_eq!(
        editor.request_prefab_transition_to_asset(second_prefab.clone()),
        PrefabTransitionPrompt::None
    );
    assert_eq!(editor.mode, EditorMode::Prefab(second_prefab.id));
    assert_eq!(editor.return_mode, Some(EditorMode::Room(room_id)));
    assert_eq!(
        editor.game.prefab_library.prefabs.get(&second_prefab.id),
        Some(&second_prefab)
    );
}

#[test]
fn requesting_prefab_transition_to_asset_reconciles_stale_palette_state() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_transition_asset_reconciles_palette");
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let second_prefab = save_test_prefab(&test_game, PrefabId(2), "Barrel");

    editor.room_editor.active_prefab_id = Some(PrefabId(999));
    editor.room_editor.recent_prefab_ids = vec![PrefabId(999), second_prefab.id, prefab_id];

    assert_eq!(
        editor.request_prefab_transition_to_asset(second_prefab.clone()),
        PrefabTransitionPrompt::None
    );
    assert_eq!(editor.mode, EditorMode::Prefab(second_prefab.id));
    assert_eq!(editor.return_mode, Some(EditorMode::Room(room_id)));
    assert_eq!(
        editor.game.prefab_library.prefabs.get(&second_prefab.id),
        Some(&second_prefab)
    );
    assert_eq!(editor.room_editor.active_prefab_id, Some(second_prefab.id));
    assert_eq!(
        editor.room_editor.recent_prefab_ids,
        vec![second_prefab.id, prefab_id]
    );
}

#[test]
fn requesting_prefab_transition_from_file_path_marks_dirty_session_pending() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_transition_file_dirty");
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let second_prefab = save_test_prefab(&test_game, PrefabId(2), "Barrel");

    let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
    editor
        .prefab_editor
        .as_mut()
        .unwrap()
        .create_prefab_entity(&mut editor.prefab_stage.as_mut().unwrap().ecs, Some(root));

    let result = editor.request_prefab_transition_to_path(&saved_prefab_path(&second_prefab));

    assert_eq!(result.unwrap(), PrefabTransitionPrompt::Dirty);
    assert_eq!(editor.mode, EditorMode::Prefab(prefab_id));
    assert_eq!(editor.return_mode, Some(EditorMode::Room(room_id)));
    assert_eq!(
        editor.pending_prefab_transition,
        Some(PendingPrefabTransition::OpenExisting(second_prefab.id))
    );
    assert_eq!(
        editor.game.prefab_library.prefabs.get(&second_prefab.id),
        Some(&second_prefab)
    );
}

#[test]
fn requesting_prefab_transition_from_invalid_file_path_returns_error() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_transition_file_invalid");
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let invalid_path = write_invalid_prefab(&test_game, "broken.ron");

    let error = editor
        .request_prefab_transition_to_path(&invalid_path)
        .unwrap_err();

    assert_eq!(editor.mode, EditorMode::Prefab(prefab_id));
    assert_eq!(editor.return_mode, Some(EditorMode::Room(room_id)));
    assert_eq!(editor.pending_prefab_transition, None);
    assert!(error.to_string().contains("Could not parse prefab"));
}

#[test]
fn dirty_prefab_transition_save_switches_and_persists_changes() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_dirty_switch_save");
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let second_prefab = save_test_prefab(&test_game, PrefabId(2), "Barrel");
    editor
        .game
        .prefab_library
        .prefabs
        .insert(second_prefab.id, second_prefab.clone());
    editor.room_editor.active_prefab_id = Some(second_prefab.id);
    editor.room_editor.recent_prefab_ids = vec![second_prefab.id, prefab_id];

    let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
    editor
        .prefab_editor
        .as_mut()
        .unwrap()
        .create_prefab_entity(&mut editor.prefab_stage.as_mut().unwrap().ecs, Some(root));

    assert_eq!(
        editor.request_prefab_transition(PendingPrefabTransition::OpenExisting(second_prefab.id)),
        PrefabTransitionPrompt::Dirty
    );
    assert_eq!(
        editor.pending_prefab_transition,
        Some(PendingPrefabTransition::OpenExisting(second_prefab.id))
    );

    editor.confirm_dirty_prefab_transition(DirtyPrefabExitPromptResult::SaveAndSync);

    assert_eq!(editor.mode, EditorMode::Prefab(second_prefab.id));
    assert_eq!(editor.return_mode, Some(EditorMode::Room(room_id)));
    assert_eq!(editor.room_editor.active_prefab_id, Some(prefab_id));
    assert_eq!(
        editor.room_editor.recent_prefab_ids,
        vec![prefab_id, second_prefab.id]
    );
    assert_eq!(
        editor
            .game
            .prefab_library
            .prefabs
            .get(&prefab_id)
            .map(|prefab| prefab.nodes.len()),
        Some(2)
    );
    assert_eq!(editor.pending_prefab_transition, None);

    let saved_state =
        load_prefab_palette_state(test_game.name()).expect("prefab palette state should save");
    assert_eq!(saved_state.active_prefab_id, Some(prefab_id));
    assert_eq!(
        saved_state.recent_prefab_ids,
        vec![prefab_id, second_prefab.id]
    );
}

#[test]
fn dirty_prefab_transition_save_does_not_switch_when_palette_persist_fails() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_dirty_switch_palette_save_failure");
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let second_prefab = save_test_prefab(&test_game, PrefabId(2), "Barrel");
    editor
        .game
        .prefab_library
        .prefabs
        .insert(second_prefab.id, second_prefab.clone());
    editor.room_editor.active_prefab_id = Some(PrefabId(999));
    editor.room_editor.recent_prefab_ids = vec![PrefabId(999), prefab_id, second_prefab.id];

    let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
    editor
        .prefab_editor
        .as_mut()
        .unwrap()
        .create_prefab_entity(&mut editor.prefab_stage.as_mut().unwrap().ecs, Some(root));

    let palette_path = game_folder(test_game.name()).join("prefab_palette.ron");
    std::fs::remove_file(&palette_path).expect("palette state file should exist");
    std::fs::create_dir(&palette_path).expect("palette path should become a directory");

    assert_eq!(
        editor.request_prefab_transition(PendingPrefabTransition::OpenExisting(second_prefab.id)),
        PrefabTransitionPrompt::Dirty
    );
    assert_eq!(
        editor.pending_prefab_transition,
        Some(PendingPrefabTransition::OpenExisting(second_prefab.id))
    );

    editor.confirm_dirty_prefab_transition(DirtyPrefabExitPromptResult::SaveAndSync);

    assert_eq!(editor.mode, EditorMode::Prefab(prefab_id));
    assert_eq!(editor.return_mode, Some(EditorMode::Room(room_id)));
    assert_eq!(
        editor.pending_prefab_transition,
        Some(PendingPrefabTransition::OpenExisting(second_prefab.id))
    );
    assert!(editor.active_prefab_is_clean());
    assert_eq!(editor.room_editor.active_prefab_id, Some(PrefabId(999)));
    assert_eq!(
        editor.room_editor.recent_prefab_ids,
        vec![PrefabId(999), prefab_id, second_prefab.id]
    );
}

#[test]
fn dirty_prefab_transition_cancel_keeps_current_prefab_open() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_dirty_switch_cancel");
    let (mut editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let second_prefab = save_test_prefab(&test_game, PrefabId(2), "Barrel");
    editor
        .game
        .prefab_library
        .prefabs
        .insert(second_prefab.id, second_prefab);

    let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
    editor
        .prefab_editor
        .as_mut()
        .unwrap()
        .create_prefab_entity(&mut editor.prefab_stage.as_mut().unwrap().ecs, Some(root));

    assert_eq!(
        editor.request_prefab_transition(PendingPrefabTransition::OpenExisting(PrefabId(2))),
        PrefabTransitionPrompt::Dirty
    );

    editor.confirm_dirty_prefab_transition(DirtyPrefabExitPromptResult::Cancel);

    assert_eq!(editor.mode, EditorMode::Prefab(prefab_id));
    assert_eq!(editor.return_mode, Some(EditorMode::Room(room_id)));
    assert_eq!(editor.pending_prefab_transition, None);
    assert!(!editor.active_prefab_is_clean());
}

#[test]
fn empty_prefab_transition_delete_switches_to_requested_prefab() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_empty_switch_delete");
    let (editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let second_prefab = save_test_prefab(&test_game, PrefabId(2), "Barrel");
        editor
            .game
            .prefab_library
            .prefabs
            .insert(second_prefab.id, second_prefab);

        let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
        push_command(Box::new(DeleteEntityCmd::new(
            root,
            EditorMode::Prefab(prefab_id),
        )));
    });
    apply_pending_commands();

    with_editor(|editor| {
        editor.reconcile_active_prefab_room_preview();
        assert_eq!(
            editor.request_prefab_transition(PendingPrefabTransition::OpenExisting(PrefabId(2))),
            PrefabTransitionPrompt::Empty
        );
        editor.confirm_empty_prefab_transition(EmptyPrefabExitPromptResult::DeletePrefab);

        assert_eq!(editor.mode, EditorMode::Prefab(PrefabId(2)));
        assert_eq!(editor.return_mode, Some(EditorMode::Room(room_id)));
        assert!(!editor.game.prefab_library.prefabs.contains_key(&prefab_id));
        assert_eq!(editor.pending_prefab_transition, None);
    });
}

#[test]
fn deleting_active_prefab_promotes_next_recent_and_persists_palette_state() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_delete_active_promotes_recent");
    let (editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let second_prefab = save_test_prefab(&test_game, PrefabId(2), "Barrel");
        editor
            .game
            .prefab_library
            .prefabs
            .insert(second_prefab.id, second_prefab);
        editor.room_editor.active_prefab_id = Some(prefab_id);
        editor.room_editor.recent_prefab_ids = vec![prefab_id, PrefabId(2)];
        assert!(editor.save_prefab_palette_state());

        let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
        push_command(Box::new(DeleteEntityCmd::new(
            root,
            EditorMode::Prefab(prefab_id),
        )));
    });
    apply_pending_commands();

    with_editor(|editor| {
        editor.reconcile_active_prefab_room_preview();
        assert_eq!(
            editor.request_prefab_transition(PendingPrefabTransition::Exit),
            PrefabTransitionPrompt::Empty
        );

        editor.confirm_empty_prefab_transition(EmptyPrefabExitPromptResult::DeletePrefab);

        assert_eq!(editor.mode, EditorMode::Room(room_id));
        assert_eq!(editor.room_editor.active_prefab_id, Some(PrefabId(2)));
        assert_eq!(editor.room_editor.recent_prefab_ids, vec![PrefabId(2)]);
    });

    let saved_state =
        load_prefab_palette_state(test_game.name()).expect("prefab palette state should save");
    assert_eq!(saved_state.active_prefab_id, Some(PrefabId(2)));
    assert_eq!(saved_state.recent_prefab_ids, vec![PrefabId(2)]);
}

#[test]
fn deleting_non_active_recent_prefab_compacts_palette_without_changing_active_prefab() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_delete_recent_compacts_palette");
    let (editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let second_prefab = save_test_prefab(&test_game, PrefabId(2), "Barrel");
        editor
            .game
            .prefab_library
            .prefabs
            .insert(second_prefab.id, second_prefab);
        editor.room_editor.active_prefab_id = Some(PrefabId(2));
        editor.room_editor.recent_prefab_ids = vec![PrefabId(2), prefab_id];
        assert!(editor.save_prefab_palette_state());

        let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
        push_command(Box::new(DeleteEntityCmd::new(
            root,
            EditorMode::Prefab(prefab_id),
        )));
    });
    apply_pending_commands();

    with_editor(|editor| {
        editor.reconcile_active_prefab_room_preview();
        assert_eq!(
            editor.request_prefab_transition(PendingPrefabTransition::Exit),
            PrefabTransitionPrompt::Empty
        );

        editor.confirm_empty_prefab_transition(EmptyPrefabExitPromptResult::DeletePrefab);

        assert_eq!(editor.mode, EditorMode::Room(room_id));
        assert_eq!(editor.room_editor.active_prefab_id, Some(PrefabId(2)));
        assert_eq!(editor.room_editor.recent_prefab_ids, vec![PrefabId(2)]);
    });

    let saved_state =
        load_prefab_palette_state(test_game.name()).expect("prefab palette state should save");
    assert_eq!(saved_state.active_prefab_id, Some(PrefabId(2)));
    assert_eq!(saved_state.recent_prefab_ids, vec![PrefabId(2)]);
}

#[test]
fn deleting_active_prefab_keeps_reconciled_palette_state_when_palette_persist_fails() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_delete_palette_save_failure_reconciles");
    let (editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let second_prefab = save_test_prefab(&test_game, PrefabId(2), "Barrel");
        editor
            .game
            .prefab_library
            .prefabs
            .insert(second_prefab.id, second_prefab);
        editor.room_editor.active_prefab_id = Some(prefab_id);
        editor.room_editor.recent_prefab_ids = vec![prefab_id, PrefabId(999), PrefabId(2)];
        assert!(editor.save_prefab_palette_state());

        let palette_path = game_folder(test_game.name()).join("prefab_palette.ron");
        std::fs::remove_file(&palette_path).expect("palette state file should exist");
        std::fs::create_dir(&palette_path).expect("palette path should become a directory");

        let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
        push_command(Box::new(DeleteEntityCmd::new(
            root,
            EditorMode::Prefab(prefab_id),
        )));
    });
    apply_pending_commands();

    with_editor(|editor| {
        editor.reconcile_active_prefab_room_preview();
        assert_eq!(
            editor.request_prefab_transition(PendingPrefabTransition::Exit),
            PrefabTransitionPrompt::Empty
        );

        editor.confirm_empty_prefab_transition(EmptyPrefabExitPromptResult::DeletePrefab);

        assert_eq!(editor.mode, EditorMode::Room(room_id));
        assert_eq!(editor.room_editor.active_prefab_id, Some(PrefabId(2)));
        assert_eq!(editor.room_editor.recent_prefab_ids, vec![PrefabId(2)]);
        assert!(!editor.game.prefab_library.prefabs.contains_key(&prefab_id));
    });
}

#[test]
fn blank_prefab_transition_does_not_create_asset_until_confirmed() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_new_switch_cancel");
    let (mut editor, _, _, _) = make_prefab_session_editor(&test_game);

    let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
    editor
        .prefab_editor
        .as_mut()
        .unwrap()
        .create_prefab_entity(&mut editor.prefab_stage.as_mut().unwrap().ecs, Some(root));

    assert_eq!(
        editor
            .request_prefab_transition(PendingPrefabTransition::CreateBlank("Fresh".to_string(),)),
        PrefabTransitionPrompt::Dirty
    );
    assert!(editor
        .game
        .prefab_library
        .prefabs
        .values()
        .all(|prefab| prefab.name != "Fresh"));

    editor.confirm_dirty_prefab_transition(DirtyPrefabExitPromptResult::Cancel);

    assert!(editor
        .game
        .prefab_library
        .prefabs
        .values()
        .all(|prefab| prefab.name != "Fresh"));
}

#[test]
fn request_blank_prefab_transition_returns_dirty_prompt_without_opening_a_modal() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_new_prompt_dirty");
    let (mut editor, _, _, _) = make_prefab_session_editor(&test_game);

    let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
    editor
        .prefab_editor
        .as_mut()
        .unwrap()
        .create_prefab_entity(&mut editor.prefab_stage.as_mut().unwrap().ecs, Some(root));

    assert!(!editor.modal.is_open());
    assert_eq!(
        editor.request_blank_prefab_transition("Fresh".to_string()),
        PrefabTransitionPrompt::Dirty
    );
    assert!(editor.pending_prefab_transition.is_some());
    assert!(!editor.modal.is_open());
}
