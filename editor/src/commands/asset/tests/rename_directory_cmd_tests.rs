// editor/src/commands/asset/tests/rename_directory_cmd_tests.rs
use crate::app::EditorMode;
use crate::commands::asset::RenameDirectoryCmd;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::with_editor;
use crate::test_utils::{setup_editor, TestEditorContext};
use engine_core::prelude::*;
use std::fs;
use std::path::PathBuf;

const ASSETS_DIR: &str = "enemies";
const RENAMED_DIR: &str = "flying";
const BAT_FILENAME: &str = "bat.png";
const BOSS_FILENAME: &str = "boss.png";
const BAT_BYTES: &[u8] = b"test-png-bytes";
const BOSS_BYTES: &[u8] = b"boss-png-bytes";

fn setup_editor_with_registered_tree(test_prefix: &str) -> TestEditorContext {
    let ctx = setup_editor(test_prefix);

    let first = assets_folder().join(ASSETS_DIR).join(BAT_FILENAME);
    let second = assets_folder().join(ASSETS_DIR).join(BOSS_FILENAME);
    fs::create_dir_all(first.parent().unwrap()).unwrap();
    fs::write(&first, BAT_BYTES).unwrap();
    fs::write(&second, BOSS_BYTES).unwrap();

    with_editor(|editor| {
        editor
            .game
            .asset_registry
            .register_asset_relative_path(SpriteId(1), format!("{ASSETS_DIR}/{BAT_FILENAME}"))
            .unwrap();
        editor
            .game
            .asset_registry
            .register_asset_relative_path(SpriteId(2), format!("{ASSETS_DIR}/{BOSS_FILENAME}"))
            .unwrap();
    });

    ctx
}

fn registered_canonical_path(id: SpriteId) -> PathBuf {
    with_editor(|editor| {
        editor
            .game
            .asset_registry
            .record(AssetKey::Sprite(id))
            .unwrap()
            .path
            .clone()
    })
}

#[test]
fn rename_directory_cmd_moves_tree_and_rewrites_registry_prefixes() {
    let _ctx = setup_editor_with_registered_tree("rename_dir_cmd_execute");

    let old_dir = assets_folder().join(ASSETS_DIR);
    let new_dir = assets_folder().join(RENAMED_DIR);

    let mut cmd = RenameDirectoryCmd::new(old_dir.clone(), new_dir.clone());
    cmd.execute();

    assert!(
        !old_dir.exists(),
        "old directory should be removed after rename"
    );
    assert!(new_dir.exists(), "new directory should exist after rename");
    assert!(
        new_dir.join(BAT_FILENAME).exists(),
        "child files should move with the directory"
    );
    assert_eq!(
        registered_canonical_path(SpriteId(1)),
        PathBuf::from(paths::ASSETS_FOLDER)
            .join(RENAMED_DIR)
            .join(BAT_FILENAME),
        "registry paths should be remapped to the new directory prefix"
    );
    assert_eq!(
        registered_canonical_path(SpriteId(2)),
        PathBuf::from(paths::ASSETS_FOLDER)
            .join(RENAMED_DIR)
            .join(BOSS_FILENAME),
        "registry paths should be remapped to the new directory prefix"
    );
}

#[test]
fn rename_directory_cmd_undo_restores_original_tree_and_registry_paths() {
    let _ctx = setup_editor_with_registered_tree("rename_dir_cmd_undo");

    let old_dir = assets_folder().join(ASSETS_DIR);
    let new_dir = assets_folder().join(RENAMED_DIR);

    let mut cmd = RenameDirectoryCmd::new(old_dir.clone(), new_dir.clone());
    cmd.execute();
    cmd.undo();

    assert!(old_dir.exists(), "old directory should be restored on undo");
    assert!(!new_dir.exists(), "new directory should be removed on undo");
    assert!(
        old_dir.join(BAT_FILENAME).exists(),
        "child files should be restored on undo"
    );
    assert_eq!(
        registered_canonical_path(SpriteId(1)),
        PathBuf::from(paths::ASSETS_FOLDER)
            .join(ASSETS_DIR)
            .join(BAT_FILENAME),
        "registry paths should be remapped back on undo"
    );
    assert_eq!(
        registered_canonical_path(SpriteId(2)),
        PathBuf::from(paths::ASSETS_FOLDER)
            .join(ASSETS_DIR)
            .join(BOSS_FILENAME),
        "registry paths should be remapped back on undo"
    );
}

#[test]
fn applies_in_all_modes() {
    use std::path::Path;

    let cmd = RenameDirectoryCmd::new(Path::new("/tmp/old"), Path::new("/tmp/new"));
    assert!(cmd.applies_in_mode(EditorMode::Game));
    assert!(cmd.applies_in_mode(EditorMode::Room(RoomId(1))));
    assert!(cmd.applies_in_mode(EditorMode::Prefab(PrefabId(5))));
    assert!(cmd.applies_in_mode(EditorMode::Menu));
}
