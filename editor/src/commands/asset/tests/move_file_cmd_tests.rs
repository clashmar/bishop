use crate::app::EditorMode;
use crate::commands::asset::MoveFileCmd;
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
fn move_file_cmd_moves_unregistered_file() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("move_unregistered");
    set_game_name(test_game.name());

    let editor = Editor {
        game: create_new_game(test_game.name().to_string()),
        ..Default::default()
    };
    let _guard = EditorServicesGuard::install(editor);

    let old_path = resources_folder_current().join("old.txt");
    let new_path = resources_folder_current().join("moved.txt");

    fs::write(&old_path, b"hello").unwrap();
    assert!(old_path.exists());
    assert!(!new_path.exists());

    let mut cmd = MoveFileCmd::new(&old_path, &new_path, None);
    cmd.execute();

    assert!(!old_path.exists(), "old file should be gone");
    assert!(new_path.exists(), "new file should exist");
}

#[test]
fn move_file_cmd_moves_registered_file_and_updates_registry() {
    let (test_game, _guard) =
        setup_editor_with_sprite("move_registered", SpriteId(1), "old_name.png");
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    set_game_name(test_game.name());

    let dir = assets_folder();
    let old_path = dir.join("old_name.png");
    let new_path = dir.join("new_name.png");

    assert!(old_path.exists());
    assert!(!new_path.exists());

    let mut cmd = MoveFileCmd::new(&old_path, &new_path, Some(AssetKey::Sprite(SpriteId(1))));
    cmd.execute();

    assert!(!old_path.exists(), "old file should be gone");
    assert!(new_path.exists(), "new file should exist");

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
fn move_file_cmd_aborts_when_destination_exists() {
    let (test_game, _guard) = setup_editor_with_sprite("move_abort", SpriteId(1), "old_name.png");
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    set_game_name(test_game.name());

    let dir = assets_folder();
    let old_path = dir.join("old_name.png");
    let new_path = dir.join("new_name.png");

    fs::write(&new_path, b"existing").unwrap();
    assert!(old_path.exists());
    assert!(new_path.exists());

    let mut cmd = MoveFileCmd::new(&old_path, &new_path, Some(AssetKey::Sprite(SpriteId(1))));
    cmd.execute();

    assert!(old_path.exists(), "old file should still exist");
    assert!(new_path.exists(), "new file should still exist");
    assert_eq!(
        fs::read(&new_path).unwrap(),
        b"existing",
        "new file should be unchanged"
    );

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
        "registry should be unchanged"
    );
}

#[test]
fn move_file_cmd_undo_restores_file_and_registry() {
    let (test_game, _guard) = setup_editor_with_sprite("move_undo", SpriteId(1), "old_name.png");
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    set_game_name(test_game.name());

    let dir = assets_folder();
    let old_path = dir.join("old_name.png");
    let new_path = dir.join("new_name.png");

    assert!(old_path.exists());
    assert!(!new_path.exists());

    let mut cmd = MoveFileCmd::new(&old_path, &new_path, Some(AssetKey::Sprite(SpriteId(1))));
    cmd.execute();
    cmd.undo();

    assert!(old_path.exists(), "old file should be restored");
    assert!(!new_path.exists(), "new file should be gone");

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
    let cmd = MoveFileCmd::new("a", "b", None);
    assert!(cmd.applies_in_mode(EditorMode::Game));
    assert!(cmd.applies_in_mode(EditorMode::Room(RoomId(1))));
    assert!(cmd.applies_in_mode(EditorMode::Prefab(PrefabId(5))));
    assert!(cmd.applies_in_mode(EditorMode::Menu));
}
