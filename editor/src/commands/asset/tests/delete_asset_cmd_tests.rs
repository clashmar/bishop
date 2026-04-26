// editor/src/commands/asset/tests/delete_asset_cmd_tests.rs
use crate::app::EditorMode;
use crate::commands::asset::DeleteAssetCmd;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::with_editor;
use crate::storage::editor_storage::create_new_game;
use crate::test_utils::{game_fs_test_lock, EditorServicesGuard, TestGameFolder};
use crate::Editor;
use engine_core::prelude::*;
use std::fs;

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
fn delete_removes_file_and_registry_record() {
    let (_test_game, _guard) =
        setup_editor_with_sprite("delete_removes_file", SpriteId(1), "test_sprite.png");
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    set_game_name(_test_game.name());

    let full_path = assets_folder().join("test_sprite.png");
    assert!(full_path.exists(), "file should exist before delete");

    let mut cmd = DeleteAssetCmd::new(AssetKey::Sprite(SpriteId(1)));
    cmd.execute();

    assert!(!full_path.exists(), "file should be gone after delete");
    assert!(
        with_editor(|e| e
            .game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(1)))
            .is_none()),
        "registry record should be gone after delete"
    );
}

#[test]
fn delete_undo_restores_file_and_registry_record() {
    let (_test_game, _guard) =
        setup_editor_with_sprite("delete_undo", SpriteId(1), "test_sprite.png");
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    set_game_name(_test_game.name());

    let full_path = assets_folder().join("test_sprite.png");

    let mut cmd = DeleteAssetCmd::new(AssetKey::Sprite(SpriteId(1)));
    cmd.execute();
    cmd.undo();

    assert!(full_path.exists(), "file should be restored after undo");
    assert_eq!(
        fs::read(&full_path).unwrap(),
        b"test-png-bytes",
        "file contents should match"
    );
    assert!(
        with_editor(|e| e
            .game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(1)))
            .is_some()),
        "registry record should be restored after undo"
    );
}

#[test]
fn applies_in_all_modes() {
    let cmd = DeleteAssetCmd::new(AssetKey::Sprite(SpriteId(1)));
    assert!(cmd.applies_in_mode(EditorMode::Game));
    assert!(cmd.applies_in_mode(EditorMode::Room(RoomId(1))));
    assert!(cmd.applies_in_mode(EditorMode::Prefab(PrefabId(5))));
    assert!(cmd.applies_in_mode(EditorMode::Menu));
}
