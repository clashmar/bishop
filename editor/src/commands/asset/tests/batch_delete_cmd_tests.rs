// editor/src/commands/asset/tests/batch_delete_cmd_tests.rs
use crate::app::EditorMode;
use crate::commands::asset::{BatchDeleteCmd, DeleteTarget};
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::with_editor;
use crate::test_utils::{setup_editor, TestEditorContext};
use engine_core::prelude::*;
use std::fs;

const PROPS_DIR: &str = "props";
const CRATE_FILENAME: &str = "crate.png";
const BARREL_FILENAME: &str = "barrel.png";
const CRATE_BYTES: &[u8] = b"test-png-bytes";
const BARREL_BYTES: &[u8] = b"barrel-png-bytes";
const NOTES_FILE_NAME: &str = "notes.txt";
const NOTES_BYTES: &[u8] = b"phase-two-notes";

fn setup_editor_with_mixed_resources(test_prefix: &str) -> TestEditorContext {
    let ctx = setup_editor(test_prefix);

    let props_dir = assets_folder().join(PROPS_DIR);
    fs::create_dir_all(&props_dir).unwrap();
    fs::write(props_dir.join(CRATE_FILENAME), CRATE_BYTES).unwrap();
    fs::write(props_dir.join(BARREL_FILENAME), BARREL_BYTES).unwrap();

    let notes_path = resources_folder_current().join(NOTES_FILE_NAME);
    fs::write(&notes_path, NOTES_BYTES).unwrap();

    with_editor(|editor| {
        editor
            .game
            .asset_registry
            .register_asset_relative_path(SpriteId(1), format!("{PROPS_DIR}/{CRATE_FILENAME}"))
            .unwrap();
    });

    ctx
}

#[test]
fn batch_delete_removes_multiple_files_and_dirs() {
    let _ctx = setup_editor_with_mixed_resources("batch_delete_execute");

    let props_dir = assets_folder().join(PROPS_DIR);
    let notes_path = resources_folder_current().join(NOTES_FILE_NAME);

    let targets = vec![
        DeleteTarget::Directory(UserPath::from(props_dir.clone())),
        DeleteTarget::UnregisteredFile(notes_path.clone()),
    ];

    let mut cmd = BatchDeleteCmd::new(targets);
    cmd.execute();

    assert!(!props_dir.exists(), "directory should be removed");
    assert!(!notes_path.exists(), "unregistered file should be removed");
    assert!(
        with_editor(|e| e
            .game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(1)))
            .is_none()),
        "registry entry should be removed"
    );
}

#[test]
fn batch_delete_undo_restores_all() {
    let _ctx = setup_editor_with_mixed_resources("batch_delete_undo");

    let props_dir = assets_folder().join(PROPS_DIR);
    let notes_path = resources_folder_current().join(NOTES_FILE_NAME);

    let targets = vec![
        DeleteTarget::Directory(UserPath::from(props_dir.clone())),
        DeleteTarget::UnregisteredFile(notes_path.clone()),
    ];

    let mut cmd = BatchDeleteCmd::new(targets);
    cmd.execute();
    cmd.undo();

    assert!(props_dir.exists(), "directory should be restored");
    assert!(notes_path.exists(), "unregistered file should be restored");
    assert_eq!(
        fs::read(props_dir.join(CRATE_FILENAME)).unwrap(),
        CRATE_BYTES,
        "file bytes should be restored"
    );
    assert_eq!(
        fs::read(&notes_path).unwrap(),
        NOTES_BYTES,
        "notes bytes should be restored"
    );
    assert!(
        with_editor(|e| e
            .game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(1)))
            .is_some()),
        "registry entry should be restored"
    );
}

#[test]
fn batch_delete_deduplicates_nested_selections() {
    let _ctx = setup_editor_with_mixed_resources("batch_delete_dedup");

    let props_dir = assets_folder().join(PROPS_DIR);
    let nested_file = props_dir.join(BARREL_FILENAME);

    let targets = vec![
        DeleteTarget::Directory(UserPath::from(props_dir.clone())),
        DeleteTarget::UnregisteredFile(nested_file.clone()),
    ];

    let mut cmd = BatchDeleteCmd::new(targets);
    cmd.execute();

    assert!(!props_dir.exists(), "directory should be removed");

    cmd.undo();

    assert!(props_dir.exists(), "directory should be restored");
    assert_eq!(
        fs::read(&nested_file).unwrap(),
        BARREL_BYTES,
        "nested file bytes should be restored"
    );
}

#[test]
fn batch_delete_partial_failure_continues() {
    let _ctx = setup_editor_with_mixed_resources("batch_delete_partial");

    let bad_path = resources_folder_current().join("does_not_exist.txt");
    let notes_path = resources_folder_current().join(NOTES_FILE_NAME);

    let targets = vec![
        DeleteTarget::UnregisteredFile(bad_path.clone()),
        DeleteTarget::UnregisteredFile(notes_path.clone()),
    ];

    let mut cmd = BatchDeleteCmd::new(targets);
    cmd.execute();

    assert!(!notes_path.exists(), "valid file should be deleted");

    cmd.undo();

    assert!(notes_path.exists(), "valid file should be restored on undo");
    assert_eq!(
        fs::read(&notes_path).unwrap(),
        NOTES_BYTES,
        "notes bytes should be restored"
    );
}

#[test]
fn batch_delete_redo_deletes_again() {
    let _ctx = setup_editor_with_mixed_resources("batch_delete_redo");

    let props_dir = assets_folder().join(PROPS_DIR);
    let notes_path = resources_folder_current().join(NOTES_FILE_NAME);

    let targets = vec![
        DeleteTarget::Directory(UserPath::from(props_dir.clone())),
        DeleteTarget::UnregisteredFile(notes_path.clone()),
    ];

    let mut cmd = BatchDeleteCmd::new(targets);
    cmd.execute();

    assert!(
        !props_dir.exists(),
        "directory should be removed after first execute"
    );
    assert!(
        !notes_path.exists(),
        "unregistered file should be removed after first execute"
    );

    cmd.undo();

    assert!(
        props_dir.exists(),
        "directory should be restored after undo"
    );
    assert!(
        notes_path.exists(),
        "unregistered file should be restored after undo"
    );

    // Simulate redo: execute() is called again by the command manager
    cmd.execute();

    assert!(
        !props_dir.exists(),
        "directory should be removed after redo"
    );
    assert!(
        !notes_path.exists(),
        "unregistered file should be removed after redo"
    );
    assert!(
        with_editor(|e| e
            .game
            .asset_registry
            .record(AssetKey::Sprite(SpriteId(1)))
            .is_none()),
        "registry entry should be removed after redo"
    );
}

#[test]
fn applies_in_all_modes() {
    let cmd = BatchDeleteCmd::new(Vec::new());
    assert!(cmd.applies_in_mode(EditorMode::Game));
    assert!(cmd.applies_in_mode(EditorMode::Room(RoomId(1))));
    assert!(cmd.applies_in_mode(EditorMode::Prefab(PrefabId(5))));
    assert!(cmd.applies_in_mode(EditorMode::Menu));
}
