use super::super::context_menu::{open_resource, ResourceOpenResult};
use crate::app::EditorMode;
use crate::storage::editor_storage::create_new_game;
use crate::test_utils::{game_fs_test_lock, make_prefab_session_editor, TestGameFolder};
use engine_core::assets::AssetKey;
use engine_core::constants::{extensions, paths};
use engine_core::engine_global::set_game_name;
use engine_core::storage::path_utils::resources_folder_current;

#[test]
fn open_resource_registered_prefab_returns_transition() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("open_resource_registered_prefab");
    let (mut editor, room_id, prefab_id, _root) = make_prefab_session_editor(&test_game);

    editor.prefab_editor = None;
    editor.mode = EditorMode::Room(room_id);

    let prefab_path = editor
        .game
        .asset_registry
        .record(AssetKey::Prefab(prefab_id))
        .map(|r| resources_folder_current().join(&r.path))
        .expect("prefab should be registered");

    let result = open_resource(&prefab_path, &mut editor);

    assert_eq!(result, ResourceOpenResult::PrefabTransition(prefab_id));
}

#[test]
fn open_resource_already_open_prefab_returns_transition() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("open_resource_already_open");
    let (mut editor, _room_id, prefab_id, _root) = make_prefab_session_editor(&test_game);

    let prefab_path = editor
        .game
        .asset_registry
        .record(AssetKey::Prefab(prefab_id))
        .map(|r| resources_folder_current().join(&r.path))
        .expect("prefab should be registered");

    editor.toast = None;

    let result = open_resource(&prefab_path, &mut editor);

    assert_eq!(result, ResourceOpenResult::PrefabTransition(prefab_id));
    assert!(editor.toast.is_none());
}

#[test]
fn open_resource_unregistered_prefab_shows_toast() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("open_resource_unregistered_prefab");
    set_game_name(test_game.name());
    let mut editor = crate::app::Editor {
        game: create_new_game(test_game.name().to_string()),
        ..Default::default()
    };

    let unregistered_path = resources_folder_current()
        .join(paths::PREFABS_FOLDER)
        .join(format!("ghost.{}", extensions::PREFAB));
    std::fs::create_dir_all(unregistered_path.parent().unwrap()).unwrap();
    std::fs::write(&unregistered_path, "").unwrap();

    let result = open_resource(&unregistered_path, &mut editor);

    assert_eq!(result, ResourceOpenResult::Handled);
    assert!(
        editor
            .toast
            .as_ref()
            .is_some_and(|t| t.msg == "Unregistered prefab file"),
        "expected toast for unregistered prefab, got: {}",
        editor
            .toast
            .as_ref()
            .map_or("None".to_string(), |t| t.msg.clone())
    );
}
