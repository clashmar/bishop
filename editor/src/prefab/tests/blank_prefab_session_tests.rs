use super::*;

fn blank_prefab_session_editor(test_game: &TestGameFolder) -> Editor {
    set_game_name(test_game.name());
    let game = create_new_game(test_game.name().to_string());
    let prefab_stage = PrefabStage::from_editor_services(&game);

    Editor {
        game,
        mode: EditorMode::Prefab(BLANK_PREFAB_ID),
        return_mode: Some(EditorMode::Room(RoomId(1))),
        prefab_editor: Some(PrefabEditor::new(
            BLANK_PREFAB_ID,
            "Prefab".to_string(),
            StagedPrefabState::Empty,
            PrefabRoomSyncState {
                staged_prefab: StagedPrefabState::Empty,
                linked_instance_snapshots: Vec::new(),
            },
        )),
        prefab_stage: Some(prefab_stage),
        ..Default::default()
    }
}

#[test]
fn blank_prefab_session_has_no_persisted_prefab_id() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("blank_prefab_session_ids");
    let editor = blank_prefab_session_editor(&test_game);

    assert!(editor.is_blank_prefab_mode());
    assert_eq!(editor.active_persisted_prefab_id(), None);
}

#[test]
fn blank_prefab_session_save_is_a_no_op() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("blank_prefab_session_save");
    let mut editor = blank_prefab_session_editor(&test_game);

    editor.save_active_prefab();

    assert!(editor.is_blank_prefab_mode());
    assert_eq!(editor.active_persisted_prefab_id(), None);
    assert!(editor.toast.is_some());
    assert!(editor.game.prefab_library.prefabs.is_empty());
}

#[test]
fn blank_prefab_session_opens_real_prefab_without_prompt() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("blank_prefab_session_transition");
    let mut editor = blank_prefab_session_editor(&test_game);
    let prefab = create_prefab(PrefabId(2), "Barrel".to_string());
    editor
        .game
        .prefab_library
        .prefabs
        .insert(prefab.id, prefab.clone());

    assert_eq!(
        editor.request_prefab_transition(PendingPrefabTransition::OpenExisting(prefab.id)),
        PrefabTransitionPrompt::None
    );
    assert_eq!(editor.mode, EditorMode::Prefab(prefab.id));
    assert_eq!(editor.return_mode, Some(EditorMode::Room(RoomId(1))));
    assert_eq!(editor.prefab_state.pending_transition(), None);
    assert_eq!(editor.active_persisted_prefab_id(), Some(prefab.id));
}

#[test]
fn dirty_blank_prefab_session_still_opens_real_prefab_without_prompt() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("dirty_blank_prefab_session_transition");
    let mut editor = blank_prefab_session_editor(&test_game);
    let prefab = create_prefab(PrefabId(2), "Barrel".to_string());
    editor
        .game
        .prefab_library
        .prefabs
        .insert(prefab.id, prefab.clone());

    let root = editor
        .prefab_editor
        .as_mut()
        .expect("prefab editor should exist")
        .create_prefab_entity(
            &mut editor
                .prefab_stage
                .as_mut()
                .expect("prefab stage should exist")
                .ecs,
            None,
        );
    editor
        .prefab_editor
        .as_mut()
        .expect("prefab editor should exist")
        .set_selected_entity(Some(root));

    assert_eq!(
        editor.request_prefab_transition(PendingPrefabTransition::OpenExisting(prefab.id)),
        PrefabTransitionPrompt::None
    );
    assert_eq!(editor.mode, EditorMode::Prefab(prefab.id));
    assert_eq!(editor.return_mode, Some(EditorMode::Room(RoomId(1))));
    assert_eq!(editor.prefab_state.pending_transition(), None);
}
