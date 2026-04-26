use super::*;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_assets::generate_prefabs_lua;
use engine_core::scripting::lua_constants::{lua_dirs, lua_files};

#[test]
fn delete_prefab_command_removes_prefab_record_from_asset_registry() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_delete_removes_asset_record");
    let (editor, _, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    let expected_relative_path = with_editor(|editor| {
        let prefab = match editor.active_prefab_staged_state() {
            Some(StagedPrefabState::PrefabAsset(prefab)) => prefab,
            _ => unreachable!(),
        };
        let expected_relative_path = PathBuf::from(
            saved_prefab_path(&prefab)
                .file_name()
                .expect("saved prefab path should have file name"),
        );
        assert!(editor.commit_prefab_asset_save(prefab));
        assert_eq!(
            editor.game.asset_registry.relative_path(prefab_id),
            Some(expected_relative_path.clone())
        );
        expected_relative_path
    });

    push_command(Box::new(crate::commands::scene::DeletePrefabCmd::new(
        prefab_id,
    )));
    apply_pending_commands();

    with_editor(|editor| {
        assert_eq!(
            editor
                .game
                .asset_registry
                .record(AssetKey::Prefab(prefab_id)),
            None
        );
        assert_eq!(editor.game.asset_registry.relative_path(prefab_id), None);
        assert_eq!(
            editor
                .game
                .asset_registry
                .key_for_path(PathBuf::from(paths::PREFABS_FOLDER).join(expected_relative_path)),
            None
        );
    });
}

#[test]
fn delete_prefab_command_persists_registry_changes_across_reload_and_undo() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_delete_persists_registry_changes");
    let (editor, _, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    let expected_relative_path = with_editor(|editor| {
        let prefab = match editor.active_prefab_staged_state() {
            Some(StagedPrefabState::PrefabAsset(prefab)) => prefab,
            _ => unreachable!(),
        };
        let expected_relative_path = PathBuf::from(
            saved_prefab_path(&prefab)
                .file_name()
                .expect("saved prefab path should have file name"),
        );
        assert!(editor.commit_prefab_asset_save(prefab));
        expected_relative_path
    });

    push_command(Box::new(crate::commands::scene::DeletePrefabCmd::new(
        prefab_id,
        EditorMode::Prefab(prefab_id),
    )));
    apply_pending_commands();

    let mut reloaded_after_delete =
        load_game_by_name(test_game.name()).expect("game should reload");
    assert_eq!(
        reloaded_after_delete
            .asset_registry
            .relative_path(prefab_id),
        None
    );
    assert_eq!(
        reloaded_after_delete
            .asset_registry
            .key_for_path(PathBuf::from(paths::PREFABS_FOLDER).join(&expected_relative_path)),
        None
    );
    reloaded_after_delete.reload_prefab_manager();
    assert!(!reloaded_after_delete
        .prefab_manager
        .prefabs
        .contains_key(&prefab_id));

    request_undo();
    apply_pending_commands();

    let mut reloaded_after_undo = load_game_by_name(test_game.name()).expect("game should reload");
    assert_eq!(
        reloaded_after_undo.asset_registry.relative_path(prefab_id),
        Some(expected_relative_path.clone())
    );
    assert_eq!(
        reloaded_after_undo
            .asset_registry
            .key_for_path(PathBuf::from(paths::PREFABS_FOLDER).join(expected_relative_path)),
        Some(AssetKey::Prefab(prefab_id))
    );
    reloaded_after_undo.reload_prefab_manager();
    assert!(reloaded_after_undo
        .prefab_manager
        .prefabs
        .contains_key(&prefab_id));
}

#[test]
fn delete_prefab_command_applies_in_deleted_blank_prefab_mode() {
    let prefab_id = PrefabId(7);
    let command = crate::commands::scene::DeletePrefabCmd::new(prefab_id);

    assert!(command.applies_in_mode(EditorMode::Prefab(prefab_id)));
    assert!(command.applies_in_mode(EditorMode::Prefab(BLANK_PREFAB_ID)));
    assert!(!command.applies_in_mode(EditorMode::Room(RoomId(1))));
}

#[test]
fn delete_prefab_command_restores_asset_instances_palette_and_session_on_undo_redo() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_delete_undo_redo");
    let (editor, _, prefab_id, _) = make_prefab_session_editor(&test_game);
    let other_prefab = save_test_prefab(&test_game, PrefabId(2), "Barrel");
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        editor
            .game
            .prefab_manager
            .prefabs
            .insert(other_prefab.id, other_prefab.clone());
        editor.room_editor.active_prefab_id = Some(prefab_id);
        editor.room_editor.recent_prefab_ids = vec![prefab_id, other_prefab.id];

        push_command(Box::new(crate::commands::scene::DeletePrefabCmd::new(
            prefab_id,
            EditorMode::Prefab(prefab_id),
        )));
    });
    apply_pending_commands();

    with_editor(|editor| {
        assert!(editor.is_blank_prefab_mode());
        assert_eq!(editor.mode, EditorMode::Prefab(BLANK_PREFAB_ID));
        assert!(editor.prefab_state.require_picker());
        assert!(!editor.game.prefab_manager.prefabs.contains_key(&prefab_id));
        assert!(linked_root_entities(&editor.game.ecs, prefab_id).is_empty());
        assert_eq!(editor.room_editor.active_prefab_id, Some(other_prefab.id));
        assert_eq!(editor.room_editor.recent_prefab_ids, vec![other_prefab.id]);
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        assert_eq!(editor.mode, EditorMode::Prefab(prefab_id));
        assert!(!editor.prefab_state.require_picker());
        assert!(!editor.modal.is_open());
        assert_eq!(
            editor.prefab_editor.as_ref().map(|prefab| prefab.prefab_id),
            Some(prefab_id)
        );
        assert!(editor.game.prefab_manager.prefabs.contains_key(&prefab_id));
        let prefabs_lua = std::fs::read_to_string(
            scripts_folder()
                .join(lua_dirs::ENGINE)
                .join(lua_files::PREFABS),
        )
        .unwrap();
        let prefab_names = editor
            .game
            .prefab_manager
            .prefabs
            .values()
            .map(|prefab| prefab.name.clone())
            .collect::<Vec<_>>();
        assert_eq!(prefabs_lua, generate_prefabs_lua(&prefab_names));
        assert_eq!(linked_root_entities(&editor.game.ecs, prefab_id).len(), 1);
        assert_eq!(editor.room_editor.active_prefab_id, Some(prefab_id));
        assert_eq!(
            editor.room_editor.recent_prefab_ids,
            vec![prefab_id, other_prefab.id]
        );
    });

    request_redo();
    apply_pending_commands();

    with_editor(|editor| {
        assert!(editor.is_blank_prefab_mode());
        assert_eq!(editor.mode, EditorMode::Prefab(BLANK_PREFAB_ID));
        assert!(editor.prefab_state.require_picker());
        assert!(!editor.game.prefab_manager.prefabs.contains_key(&prefab_id));
        let prefabs_lua = std::fs::read_to_string(
            scripts_folder()
                .join(lua_dirs::ENGINE)
                .join(lua_files::PREFABS),
        )
        .unwrap();
        let prefab_names = editor
            .game
            .prefab_manager
            .prefabs
            .values()
            .map(|prefab| prefab.name.clone())
            .collect::<Vec<_>>();
        assert_eq!(prefabs_lua, generate_prefabs_lua(&prefab_names));
        assert!(linked_root_entities(&editor.game.ecs, prefab_id).is_empty());
        assert_eq!(editor.room_editor.active_prefab_id, Some(other_prefab.id));
        assert_eq!(editor.room_editor.recent_prefab_ids, vec![other_prefab.id]);
    });
}
