// editor/src/commands/asset/tests/delete_unregistered_file_cmd_tests.rs
use crate::app::EditorMode;
use crate::commands::asset::DeleteUnregisteredFileCmd;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::with_editor;
use crate::test_utils::{setup_editor, TestEditorContext};
use engine_core::prelude::*;
use std::fs;
use std::path::PathBuf;

const NOTES_FILE_NAME: &str = "notes.txt";
const ORIGINAL_NOTES_BYTES: &[u8] = b"phase-two-notes";
const UPDATED_NOTES_BYTES: &[u8] = b"updated-notes";
const TARGET_DIR_NAME: &str = "notes_dir";

#[test]
fn delete_unregistered_file_cmd_restores_original_bytes_on_undo() {
    let _ctx = setup_editor("delete_unregistered_file_cmd");
    let target = resources_folder_current().join(NOTES_FILE_NAME);
    fs::write(&target, ORIGINAL_NOTES_BYTES).unwrap();

    let mut cmd = DeleteUnregisteredFileCmd::new(target.clone());
    cmd.execute();
    assert!(!target.exists(), "target file should be deleted");
    assert!(
        with_editor(|editor| editor.game.asset_registry.key_for_path(&target).is_none()),
        "unregistered delete should not add registry state"
    );

    cmd.undo();
    assert_eq!(
        fs::read(&target).unwrap(),
        ORIGINAL_NOTES_BYTES,
        "undo should restore original bytes"
    );
    assert!(
        with_editor(|editor| editor.game.asset_registry.key_for_path(&target).is_none()),
        "unregistered delete should not touch registry on undo"
    );
}

#[test]
fn delete_unregistered_file_cmd_failed_execute_leaves_no_undo_payload() {
    let _ctx = setup_editor("delete_unregistered_file_cmd_failed_execute");
    let target = resources_folder_current().join(TARGET_DIR_NAME);
    fs::create_dir_all(&target).unwrap();

    let mut cmd = DeleteUnregisteredFileCmd::new(target.clone());
    cmd.execute();

    assert!(
        target.is_dir(),
        "failed execute should leave the directory in place"
    );

    fs::remove_dir(&target).unwrap();
    fs::write(&target, UPDATED_NOTES_BYTES).unwrap();

    cmd.undo();

    assert_eq!(
        fs::read(&target).unwrap(),
        UPDATED_NOTES_BYTES,
        "undo should not restore stale bytes after a failed delete"
    );
}

#[test]
fn applies_in_all_modes() {
    let cmd = DeleteUnregisteredFileCmd::new(PathBuf::from(NOTES_FILE_NAME));
    assert!(cmd.applies_in_mode(EditorMode::Game));
    assert!(cmd.applies_in_mode(EditorMode::Room(RoomId(1))));
    assert!(cmd.applies_in_mode(EditorMode::Prefab(PrefabId(5))));
    assert!(cmd.applies_in_mode(EditorMode::Menu));
}
