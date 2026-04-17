use super::*;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_assets::generate_prefabs_lua;
use engine_core::scripting::lua_constants::{ENGINE_DIR, PREFABS_FILE};

#[test]
fn delete_prefab_command_applies_in_deleted_blank_prefab_mode() {
    let prefab_id = PrefabId(7);
    let command =
        crate::commands::scene::DeletePrefabCmd::new(prefab_id, EditorMode::Prefab(prefab_id));

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
            .prefab_library
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
        assert!(!editor.game.prefab_library.prefabs.contains_key(&prefab_id));
        assert!(linked_root_entities(&editor.game.ecs, prefab_id).is_empty());
        assert_eq!(editor.room_editor.active_prefab_id, Some(other_prefab.id));
        assert_eq!(editor.room_editor.recent_prefab_ids, vec![other_prefab.id]);
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        assert_eq!(editor.mode, EditorMode::Prefab(prefab_id));
        assert!(!editor.prefab_state.require_picker());
        assert_eq!(
            editor.prefab_editor.as_ref().map(|prefab| prefab.prefab_id),
            Some(prefab_id)
        );
        assert!(editor.game.prefab_library.prefabs.contains_key(&prefab_id));
        let prefabs_lua =
            std::fs::read_to_string(scripts_folder().join(ENGINE_DIR).join(PREFABS_FILE)).unwrap();
        let prefab_names = editor
            .game
            .prefab_library
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
        assert!(!editor.game.prefab_library.prefabs.contains_key(&prefab_id));
        let prefabs_lua =
            std::fs::read_to_string(scripts_folder().join(ENGINE_DIR).join(PREFABS_FILE)).unwrap();
        let prefab_names = editor
            .game
            .prefab_library
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
