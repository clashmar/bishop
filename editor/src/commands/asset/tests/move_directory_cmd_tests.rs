use crate::app::EditorMode;
use crate::commands::asset::MoveDirectoryCmd;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::with_editor;
use crate::test_utils::{setup_editor, TestEditorContext};
use engine_core::prelude::*;
use std::fs;
use std::path::PathBuf;

const TEST_DIR_OLD: &str = "chars";
const TEST_DIR_NEW: &str = "heroes/chars";
const TEST_FILE_HERO: &str = "hero.png";
const TEST_FILE_ENEMY: &str = "enemy.png";

fn setup_editor_with_nested_assets(test_prefix: &str) -> TestEditorContext {
    let ctx = setup_editor(test_prefix);

    let sprite_path = assets_folder().join(TEST_DIR_OLD).join(TEST_FILE_HERO);
    fs::create_dir_all(sprite_path.parent().unwrap()).unwrap();
    fs::write(&sprite_path, b"test-png-bytes").unwrap();

    with_editor(|editor| {
        editor
            .game
            .asset_registry
            .register_asset_relative_path(SpriteId(1), format!("{TEST_DIR_OLD}/{TEST_FILE_HERO}"))
            .unwrap();
    });

    ctx
}

fn setup_editor_with_two_nested_assets(test_prefix: &str) -> TestEditorContext {
    let ctx = setup_editor(test_prefix);

    let hero_path = assets_folder().join(TEST_DIR_OLD).join(TEST_FILE_HERO);
    let enemy_path = assets_folder().join(TEST_DIR_OLD).join(TEST_FILE_ENEMY);
    fs::create_dir_all(hero_path.parent().unwrap()).unwrap();
    fs::write(&hero_path, b"hero-bytes").unwrap();
    fs::write(&enemy_path, b"enemy-bytes").unwrap();

    with_editor(|editor| {
        editor
            .game
            .asset_registry
            .register_asset_relative_path(SpriteId(1), format!("{TEST_DIR_OLD}/{TEST_FILE_HERO}"))
            .unwrap();
        editor
            .game
            .asset_registry
            .register_asset_relative_path(SpriteId(2), format!("{TEST_DIR_OLD}/{TEST_FILE_ENEMY}"))
            .unwrap();
    });

    ctx
}

#[test]
fn move_directory_cmd_moves_tree_and_remaps_registry() {
    let _ctx = setup_editor_with_nested_assets("move_dir_cmd_execute");

    let old_dir = assets_folder().join(TEST_DIR_OLD);
    let new_dir = assets_folder().join(TEST_DIR_NEW);

    fs::create_dir_all(new_dir.parent().unwrap()).unwrap();

    let mut cmd = MoveDirectoryCmd::new(UserPath::from(old_dir.clone()), new_dir.clone());
    cmd.execute();

    assert!(
        !old_dir.exists(),
        "old directory should be removed after move"
    );
    assert!(new_dir.exists(), "new directory should exist after move");
    assert!(
        new_dir.join(TEST_FILE_HERO).exists(),
        "nested file should move with the directory"
    );

    let record = with_editor(|editor| {
        editor
            .game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(1)))
            .cloned()
    });
    let expected = PathBuf::from(paths::ASSETS_FOLDER)
        .join(TEST_DIR_NEW)
        .join(TEST_FILE_HERO);
    assert_eq!(
        record.unwrap().path,
        expected,
        "registry should point to new nested path"
    );
}

#[test]
fn move_directory_cmd_remaps_multiple_assets() {
    let _ctx = setup_editor_with_two_nested_assets("move_dir_cmd_multi");

    let old_dir = assets_folder().join(TEST_DIR_OLD);
    let new_dir = assets_folder().join(TEST_DIR_NEW);

    fs::create_dir_all(new_dir.parent().unwrap()).unwrap();

    let mut cmd = MoveDirectoryCmd::new(UserPath::from(old_dir.clone()), new_dir.clone());
    cmd.execute();

    let hero_record = with_editor(|editor| {
        editor
            .game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(1)))
            .cloned()
    });
    let enemy_record = with_editor(|editor| {
        editor
            .game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(2)))
            .cloned()
    });

    let expected_hero = PathBuf::from(paths::ASSETS_FOLDER)
        .join(TEST_DIR_NEW)
        .join(TEST_FILE_HERO);
    let expected_enemy = PathBuf::from(paths::ASSETS_FOLDER)
        .join(TEST_DIR_NEW)
        .join(TEST_FILE_ENEMY);

    assert_eq!(
        hero_record.unwrap().path,
        expected_hero,
        "hero should be remapped"
    );
    assert_eq!(
        enemy_record.unwrap().path,
        expected_enemy,
        "enemy should be remapped"
    );
}

#[test]
fn move_directory_cmd_aborts_when_destination_exists() {
    let _ctx = setup_editor_with_nested_assets("move_dir_cmd_abort_dest");

    let old_dir = assets_folder().join(TEST_DIR_OLD);
    let new_dir = assets_folder().join(TEST_DIR_NEW);

    fs::create_dir_all(&new_dir).unwrap();
    fs::write(new_dir.join(TEST_FILE_HERO), b"existing").unwrap();

    assert!(old_dir.exists());
    assert!(new_dir.exists());

    let mut cmd = MoveDirectoryCmd::new(UserPath::from(old_dir.clone()), new_dir.clone());
    cmd.execute();

    assert!(old_dir.exists(), "old directory should still exist");
    assert!(new_dir.exists(), "new directory should still exist");
    assert_eq!(
        fs::read(new_dir.join(TEST_FILE_HERO)).unwrap(),
        b"existing",
        "destination should be unchanged"
    );

    let record = with_editor(|editor| {
        editor
            .game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(1)))
            .cloned()
    });
    let expected = PathBuf::from(paths::ASSETS_FOLDER)
        .join(TEST_DIR_OLD)
        .join(TEST_FILE_HERO);
    assert_eq!(
        record.unwrap().path,
        expected,
        "registry should be unchanged"
    );
}

#[test]
fn move_directory_cmd_aborts_when_moving_into_self() {
    let _ctx = setup_editor_with_nested_assets("move_dir_cmd_abort_self");

    let old_dir = assets_folder().join(TEST_DIR_OLD);
    let new_dir = assets_folder()
        .join(TEST_DIR_OLD)
        .join("nested")
        .join(TEST_DIR_OLD);

    assert!(old_dir.exists());

    let mut cmd = MoveDirectoryCmd::new(UserPath::from(old_dir.clone()), new_dir.clone());
    cmd.execute();

    assert!(old_dir.exists(), "old directory should still exist");
    assert!(
        !new_dir.exists(),
        "nested destination should not have been created"
    );

    let record = with_editor(|editor| {
        editor
            .game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(1)))
            .cloned()
    });
    let expected = PathBuf::from(paths::ASSETS_FOLDER)
        .join(TEST_DIR_OLD)
        .join(TEST_FILE_HERO);
    assert_eq!(
        record.unwrap().path,
        expected,
        "registry should be unchanged"
    );
}

#[test]
fn move_directory_cmd_undo_restores_tree_and_registry() {
    let _ctx = setup_editor_with_nested_assets("move_dir_cmd_undo");

    let old_dir = assets_folder().join(TEST_DIR_OLD);
    let new_dir = assets_folder().join(TEST_DIR_NEW);

    fs::create_dir_all(new_dir.parent().unwrap()).unwrap();

    let mut cmd = MoveDirectoryCmd::new(UserPath::from(old_dir.clone()), new_dir.clone());
    cmd.execute();
    cmd.undo();

    assert!(old_dir.exists(), "old directory should be restored on undo");
    assert!(!new_dir.exists(), "new directory should be removed on undo");
    assert!(
        old_dir.join(TEST_FILE_HERO).exists(),
        "nested file should be restored on undo"
    );

    let record = with_editor(|editor| {
        editor
            .game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(1)))
            .cloned()
    });
    let expected = PathBuf::from(paths::ASSETS_FOLDER)
        .join(TEST_DIR_OLD)
        .join(TEST_FILE_HERO);
    assert_eq!(
        record.unwrap().path,
        expected,
        "registry should be restored on undo"
    );
}

#[test]
fn applies_in_all_modes() {
    let cmd = MoveDirectoryCmd::new(
        UserPath::from(PathBuf::from("/tmp/old")),
        PathBuf::from("/tmp/new"),
    );
    assert!(cmd.applies_in_mode(EditorMode::Game));
    assert!(cmd.applies_in_mode(EditorMode::Room(RoomId(1))));
    assert!(cmd.applies_in_mode(EditorMode::Prefab(PrefabId(5))));
    assert!(cmd.applies_in_mode(EditorMode::Menu));
}
