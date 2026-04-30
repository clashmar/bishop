// editor/src/commands/asset/tests/rename_asset_cmd_tests.rs
use crate::app::EditorMode;
use crate::commands::asset::RenameAssetCmd;
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
fn rename_updates_registry_path_and_moves_file() {
    let (_test_game, _guard) =
        setup_editor_with_sprite("rename_updates_path", SpriteId(1), "test_sprite.png");
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    set_game_name(_test_game.name());

    let old_path = assets_folder().join("test_sprite.png");
    let new_relative = "test_sprite_renamed.png";
    let new_path = assets_folder().join(new_relative);

    let mut cmd = RenameAssetCmd::new(AssetKey::Sprite(SpriteId(1)), new_relative);
    cmd.execute();

    assert!(!old_path.exists(), "old file should be gone after rename");
    assert!(new_path.exists(), "new file should exist after rename");
    assert_eq!(
        fs::read(&new_path).unwrap(),
        b"test-png-bytes",
        "file contents should be preserved"
    );

    let record = with_editor(|editor| {
        editor
            .game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(1)))
            .cloned()
    });
    let expected_path = PathBuf::from(paths::ASSETS_FOLDER).join(new_relative);
    assert_eq!(
        record.unwrap().path,
        expected_path,
        "registry path should be updated"
    );
}

#[test]
fn rename_undo_restores_original_path_and_file() {
    let (_test_game, _guard) =
        setup_editor_with_sprite("rename_undo", SpriteId(1), "test_sprite.png");
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    set_game_name(_test_game.name());

    let old_path = assets_folder().join("test_sprite.png");
    let new_relative = "test_sprite_renamed.png";
    let new_path = assets_folder().join(new_relative);

    let mut cmd = RenameAssetCmd::new(AssetKey::Sprite(SpriteId(1)), new_relative);
    cmd.execute();
    cmd.undo();

    assert!(old_path.exists(), "old file should be restored after undo");
    assert!(!new_path.exists(), "new file should be gone after undo");
    assert_eq!(
        fs::read(&old_path).unwrap(),
        b"test-png-bytes",
        "file contents should be restored"
    );

    let record = with_editor(|editor| {
        editor
            .game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(1)))
            .cloned()
    });
    let expected_path = PathBuf::from(paths::ASSETS_FOLDER).join("test_sprite.png");
    assert_eq!(
        record.unwrap().path,
        expected_path,
        "registry path should be restored on undo"
    );
}

#[test]
fn rename_rejected_if_new_path_already_registered() {
    let (_test_game, _guard) =
        setup_editor_with_sprite("rename_conflict", SpriteId(1), "test_sprite.png");
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    set_game_name(_test_game.name());

    with_editor(|editor| {
        editor
            .game
            .asset_registry
            .register_asset_relative_path(SpriteId(2), "other_sprite.png")
            .unwrap();
    });

    let cmd = RenameAssetCmd::new(AssetKey::Sprite(SpriteId(1)), "other_sprite.png");
    assert!(
        cmd.is_valid().is_some(),
        "should reject rename to a path already owned by another key"
    );
}

#[test]
fn applies_in_all_modes() {
    let cmd = RenameAssetCmd::new(AssetKey::Sprite(SpriteId(1)), "new_name.png");
    assert!(cmd.applies_in_mode(EditorMode::Game));
    assert!(cmd.applies_in_mode(EditorMode::Room(RoomId(1))));
    assert!(cmd.applies_in_mode(EditorMode::Prefab(PrefabId(5))));
    assert!(cmd.applies_in_mode(EditorMode::Menu));
}
