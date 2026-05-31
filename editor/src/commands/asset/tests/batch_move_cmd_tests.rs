/src/commands/asset/tests/batch_move_cmd_tests.rs
use crate::app::EditorMode;
use crate::commands::asset::{BatchMoveCmd, MoveTarget};
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::{take_pending_toast, with_editor};
use crate::test_utils::{setup_editor, TestEditorContext};
use engine_core::prelude::*;
use std::fs;
use std::path::PathBuf;

const TEST_DIR_OLD: &str = "chars";
const TEST_DIR_NEW: &str = "heroes/chars";
const TEST_FILE_HERO: &str = "hero.png";

fn setup_editor_with_nested_asset(test_prefix: &str) -> TestEditorContext {
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

#[test]
fn batch_move_cmd_deduplicates_directory_and_inner_file() {
    let _ctx = setup_editor_with_nested_asset("batch_move_dedup");

    let old_dir = assets_folder().join(TEST_DIR_OLD);
    let new_dir = assets_folder().join(TEST_DIR_NEW);
    let old_file = old_dir.join(TEST_FILE_HERO);
    let new_file = new_dir.join(TEST_FILE_HERO);

    fs::create_dir_all(new_dir.parent().unwrap()).unwrap();

    let targets = vec![
        MoveTarget::Directory {
            old_path: UserPath::from(old_dir.clone()),
            new_path: new_dir.clone(),
        },
        MoveTarget::File {
            old_path: old_file.clone(),
            new_path: new_file.clone(),
            key: Some(AssetKey::Sprite(SpriteId(1))),
        },
    ];

    let mut cmd = BatchMoveCmd::new(targets);
    cmd.execute();

    assert!(!old_dir.exists(), "old directory should be gone");
    assert!(new_dir.exists(), "new directory should exist");
    assert!(new_file.exists(), "file should exist at new location");

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

    cmd.undo();

    assert!(old_dir.exists(), "old directory should be restored");
    assert!(!new_dir.exists(), "new directory should be gone");
    assert!(old_file.exists(), "file should be restored at old location");

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
        "registry should be restored to old path"
    );
}

#[test]
fn batch_move_cmd_handles_partial_failure() {
    let _ctx = setup_editor_with_nested_asset("batch_move_partial");

    let old_dir = assets_folder().join(TEST_DIR_OLD);
    let new_dir = assets_folder().join(TEST_DIR_NEW);
    let other_old_file = assets_folder().join("other.png");
    let other_new_file = assets_folder().join("moved_other.png");

    fs::write(&other_old_file, b"other-bytes").unwrap();

    // Create collision: destination for the directory already exists
    fs::create_dir_all(&new_dir).unwrap();
    fs::write(new_dir.join(TEST_FILE_HERO), b"collision").unwrap();

    let targets = vec![
        MoveTarget::Directory {
            old_path: UserPath::from(old_dir.clone()),
            new_path: new_dir.clone(),
        },
        MoveTarget::File {
            old_path: other_old_file.clone(),
            new_path: other_new_file.clone(),
            key: None,
        },
    ];

    let mut cmd = BatchMoveCmd::new(targets);
    cmd.execute();

    // Directory move should fail (collision)
    assert!(old_dir.exists(), "old directory should still exist");
    assert!(new_dir.exists(), "new directory should still exist");
    assert_eq!(
        fs::read(new_dir.join(TEST_FILE_HERO)).unwrap(),
        b"collision",
        "destination file should be unchanged"
    );

    // File move should succeed
    assert!(!other_old_file.exists(), "other old file should be gone");
    assert!(other_new_file.exists(), "other new file should exist");

    // Toast should report the failure
    let toast = take_pending_toast();
    assert!(
        toast.is_some(),
        "a toast should have been pushed for the failure"
    );
    let toast = toast.unwrap();
    assert!(
        toast.msg.contains("Could not move 1 item"),
        "toast should mention the failure count, got: {}",
        toast.msg
    );

    // Undo should restore the successful move
    cmd.undo();

    assert!(other_old_file.exists(), "other old file should be restored");
    assert!(!other_new_file.exists(), "other new file should be gone");
}

#[test]
fn batch_move_cmd_redo_moves_again() {
    let _ctx = setup_editor_with_nested_asset("batch_move_redo");

    let old_dir = assets_folder().join(TEST_DIR_OLD);
    let new_dir = assets_folder().join(TEST_DIR_NEW);
    let old_file = old_dir.join(TEST_FILE_HERO);
    let new_file = new_dir.join(TEST_FILE_HERO);

    fs::create_dir_all(new_dir.parent().unwrap()).unwrap();

    let targets = vec![MoveTarget::Directory {
        old_path: UserPath::from(old_dir.clone()),
        new_path: new_dir.clone(),
    }];

    let mut cmd = BatchMoveCmd::new(targets);

    cmd.execute();
    assert!(
        !old_dir.exists(),
        "old directory should be gone after first execute"
    );
    assert!(
        new_dir.exists(),
        "new directory should exist after first execute"
    );
    assert!(
        new_file.exists(),
        "file should exist at new location after first execute"
    );

    cmd.undo();
    assert!(
        old_dir.exists(),
        "old directory should be restored after undo"
    );
    assert!(!new_dir.exists(), "new directory should be gone after undo");
    assert!(
        old_file.exists(),
        "file should be restored at old location after undo"
    );

    cmd.execute();
    assert!(!old_dir.exists(), "old directory should be gone after redo");
    assert!(new_dir.exists(), "new directory should exist after redo");
    assert!(
        new_file.exists(),
        "file should exist at new location after redo"
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
        "registry should point to new nested path after redo"
    );
}

#[test]
fn applies_in_all_modes() {
    let cmd = BatchMoveCmd::new(Vec::new());
    assert!(cmd.applies_in_mode(EditorMode::Game));
    assert!(cmd.applies_in_mode(EditorMode::Room(RoomId(1))));
    assert!(cmd.applies_in_mode(EditorMode::Prefab(PrefabId(5))));
    assert!(cmd.applies_in_mode(EditorMode::Menu));
}
