// editor/src/commands/asset/tests/delete_directory_cmd_tests.rs
use crate::app::EditorMode;
use crate::commands::asset::DeleteDirectoryCmd;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::with_editor;
use crate::test_utils::{setup_editor, TestEditorContext};
use engine_core::prelude::*;
use std::fs;
use std::path::PathBuf;

const PROPS_DIR: &str = "props";
const CRATE_FILENAME: &str = "crate.png";
const BARREL_FILENAME: &str = "barrel.png";
const CRATE_BYTES: &[u8] = b"test-png-bytes";
const BARREL_BYTES: &[u8] = b"barrel-png-bytes";
const EXTRA_DIR: &str = "extra";
const EXTRA_FILENAME: &str = "chest.png";
const EXTRA_BYTES: &[u8] = b"chest-png-bytes";

fn setup_editor_with_registered_tree(test_prefix: &str) -> TestEditorContext {
    let ctx = setup_editor(test_prefix);

    let first = assets_folder().join(PROPS_DIR).join(CRATE_FILENAME);
    let second = assets_folder().join(PROPS_DIR).join(BARREL_FILENAME);
    fs::create_dir_all(first.parent().unwrap()).unwrap();
    fs::write(&first, CRATE_BYTES).unwrap();
    fs::write(&second, BARREL_BYTES).unwrap();

    with_editor(|editor| {
        editor
            .game
            .asset_registry
            .register_asset_relative_path(SpriteId(1), format!("{PROPS_DIR}/{CRATE_FILENAME}"))
            .unwrap();
        editor
            .game
            .asset_registry
            .register_asset_relative_path(SpriteId(2), format!("{PROPS_DIR}/{BARREL_FILENAME}"))
            .unwrap();
    });

    ctx
}

#[test]
fn delete_directory_cmd_removes_tree_and_registry_entries() {
    let _ctx = setup_editor_with_registered_tree("delete_dir_cmd_execute");

    let target = assets_folder().join(PROPS_DIR);
    let mut cmd = DeleteDirectoryCmd::new(target.clone());
    cmd.execute();

    assert!(!target.exists(), "directory should be removed after delete");
    assert!(
        with_editor(|e| e
            .game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(1)))
            .is_none()),
        "registry entries under deleted directory should be removed"
    );
    assert!(
        with_editor(|e| e
            .game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(2)))
            .is_none()),
        "registry entries under deleted directory should be removed"
    );
}

#[test]
fn delete_directory_cmd_undo_restores_tree_bytes_and_registry_entries() {
    let _ctx = setup_editor_with_registered_tree("delete_dir_cmd_undo");

    let target = assets_folder().join(PROPS_DIR);
    let mut cmd = DeleteDirectoryCmd::new(target.clone());
    cmd.execute();
    cmd.undo();

    assert!(target.exists(), "directory should be restored on undo");
    assert!(
        with_editor(|e| e
            .game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(1)))
            .is_some()),
        "registry entries should be restored on undo"
    );
    assert_eq!(
        fs::read(target.join(CRATE_FILENAME)).unwrap(),
        CRATE_BYTES,
        "file bytes should be restored on undo"
    );
    assert_eq!(
        fs::read(target.join(BARREL_FILENAME)).unwrap(),
        BARREL_BYTES,
        "file bytes should be restored on undo"
    );
    let restored_path = with_editor(|e| {
        e.game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(1)))
            .unwrap()
            .path
            .clone()
    });
    assert_eq!(
        restored_path,
        PathBuf::from(paths::ASSETS_FOLDER)
            .join(PROPS_DIR)
            .join(CRATE_FILENAME),
        "registry paths should be restored to their original values on undo"
    );
}

#[test]
fn delete_directory_cmd_undo_restores_nested_files() {
    let _ctx = setup_editor_with_registered_tree("delete_dir_cmd_nested_undo");
    let target = assets_folder().join(PROPS_DIR);
    let nested_dir = target.join(EXTRA_DIR);
    fs::create_dir_all(&nested_dir).unwrap();
    fs::write(nested_dir.join(EXTRA_FILENAME), EXTRA_BYTES).unwrap();

    let mut cmd = DeleteDirectoryCmd::new(target.clone());
    cmd.execute();
    cmd.undo();

    assert!(target.exists(), "directory should be restored on undo");
    assert_eq!(
        fs::read(target.join(EXTRA_DIR).join(EXTRA_FILENAME)).unwrap(),
        EXTRA_BYTES,
        "nested file bytes should be restored on undo"
    );
}

#[test]
fn applies_in_all_modes() {
    use std::path::Path;

    let cmd = DeleteDirectoryCmd::new(Path::new("/tmp/does_not_exist"));
    assert!(cmd.applies_in_mode(EditorMode::Game));
    assert!(cmd.applies_in_mode(EditorMode::Room(RoomId(1))));
    assert!(cmd.applies_in_mode(EditorMode::Prefab(PrefabId(5))));
    assert!(cmd.applies_in_mode(EditorMode::Menu));
}
