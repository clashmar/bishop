// editor/src/commands/asset/tests/remap_asset_path_cmd_tests.rs
use crate::app::EditorMode;
use crate::commands::asset::RemapAssetPathCmd;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::with_editor;
use crate::storage::editor_storage::create_new_game;
use crate::test_utils::{game_fs_test_lock, EditorServicesGuard, TestGameFolder};
use crate::Editor;
use engine_core::prelude::*;
use std::fs;
use std::path::PathBuf;

fn setup_editor_with_sprite(
    test_prefix: &str,
    sprite_id: SpriteId,
    relative_path: &str,
) -> (TestGameFolder, EditorServicesGuard) {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new(test_prefix);

    let game = create_new_game(test_game.name().to_string());

    let full_path = assets_folder().join(relative_path);
    if let Some(parent) = full_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&full_path, b"test-png-bytes").unwrap();

    let editor = Editor {
        game,
        ..Default::default()
    };
    let guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        editor
            .game
            .asset_registry
            .register_asset_relative_path(sprite_id, relative_path)
            .unwrap();
    });

    (test_game, guard)
}

#[test]
fn remap_updates_registry_path_without_moving_file() {
    let (_test_game, _guard) =
        setup_editor_with_sprite("remap_updates_path", SpriteId(1), "old_name.png");
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    set_game_name(_test_game.name());

    let dir = assets_folder();
    let old_path = dir.join("old_name.png");
    let new_path = dir.join("new_name.png");

    fs::rename(&old_path, &new_path).unwrap();
    assert!(!old_path.exists());
    assert!(new_path.exists());

    let mut cmd = RemapAssetPathCmd::new(AssetKey::Sprite(SpriteId(1)), "new_name.png");
    cmd.execute();

    let record = with_editor(|editor| {
        editor
            .game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(1)))
            .cloned()
    });
    let expected = PathBuf::from(paths::ASSETS_FOLDER).join("new_name.png");
    assert_eq!(
        record.unwrap().path,
        expected,
        "registry should point to new path"
    );
}

#[test]
fn remap_undo_reverts_registry_path() {
    let (_test_game, _guard) = setup_editor_with_sprite("remap_undo", SpriteId(1), "old_name.png");
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    set_game_name(_test_game.name());

    let dir = assets_folder();
    fs::rename(dir.join("old_name.png"), dir.join("new_name.png")).unwrap();

    let mut cmd = RemapAssetPathCmd::new(AssetKey::Sprite(SpriteId(1)), "new_name.png");
    cmd.execute();
    cmd.undo();

    let record = with_editor(|editor| {
        editor
            .game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(1)))
            .cloned()
    });
    let expected = PathBuf::from(paths::ASSETS_FOLDER).join("old_name.png");
    assert_eq!(
        record.unwrap().path,
        expected,
        "registry should be restored to old path"
    );
}

#[test]
fn applies_in_all_modes() {
    let cmd = RemapAssetPathCmd::new(AssetKey::Sprite(SpriteId(1)), "new_name.png");
    assert!(cmd.applies_in_mode(EditorMode::Game));
    assert!(cmd.applies_in_mode(EditorMode::Room(RoomId(1))));
    assert!(cmd.applies_in_mode(EditorMode::Prefab(PrefabId(5))));
    assert!(cmd.applies_in_mode(EditorMode::Menu));
}
