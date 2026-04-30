// editor/src/commands/asset/tests/create_directory_cmd_tests.rs
use crate::app::EditorMode;
use crate::commands::asset::CreateDirectoryCmd;
use crate::commands::editor_command_manager::EditorCommand;
use crate::test_utils::{setup_editor, TestEditorContext};
use engine_core::prelude::*;
use std::fs;
use std::path::PathBuf;

const SPRITES_DIR: &str = "sprites";
const PROPS_DIR: &str = "props";
const CHILD_FILE_NAME: &str = "child.txt";
const CHILD_FILE_BYTES: &[u8] = b"child-bytes";

#[test]
fn create_directory_cmd_creates_folder_and_undo_removes_it() {
    let _ctx = setup_editor("create_directory_cmd");
    let target = resources_folder_current().join(SPRITES_DIR).join(PROPS_DIR);

    let mut cmd = CreateDirectoryCmd::new(target.clone());
    cmd.execute();
    assert!(
        target.is_dir(),
        "target directory should exist after create"
    );

    cmd.undo();
    assert!(
        !target.exists(),
        "target directory should be removed on undo"
    );
}

#[test]
fn create_directory_cmd_undo_removes_created_directory_even_when_non_empty() {
    let _ctx = setup_editor("create_directory_cmd_non_empty_undo");
    let target = resources_folder_current().join(SPRITES_DIR).join(PROPS_DIR);

    let mut cmd = CreateDirectoryCmd::new(target.clone());
    cmd.execute();
    fs::write(target.join(CHILD_FILE_NAME), CHILD_FILE_BYTES).unwrap();

    cmd.undo();

    assert!(
        !target.exists(),
        "undo should remove the created directory tree"
    );
}

#[test]
fn applies_in_all_modes() {
    let cmd = CreateDirectoryCmd::new(PathBuf::from(SPRITES_DIR));
    assert!(cmd.applies_in_mode(EditorMode::Game));
    assert!(cmd.applies_in_mode(EditorMode::Room(RoomId(1))));
    assert!(cmd.applies_in_mode(EditorMode::Prefab(PrefabId(5))));
    assert!(cmd.applies_in_mode(EditorMode::Menu));
}
