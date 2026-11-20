use crate::commands::asset::RegisterAssetCmd;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::with_editor;
use crate::storage::game_io::create_new_game;
use crate::test_utils::{game_fs_test_lock, EditorServicesGuard, TestGameFolder};
use crate::Editor;
use engine_core::assets::{AssetKey, AssetKind};
use engine_core::ecs::{ScriptId, SpriteId};
use engine_core::engine_global::set_game_name;
use engine_core::storage::path_utils::{assets_folder, resources_folder_current, scripts_folder};
use std::fs;
use std::io;
use std::path::Path;

#[test]
fn register_records_existing_sprite_without_copying_it() -> Result<(), Box<dyn std::error::Error>> {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("register_sprite");
    set_game_name(test_game.name());
    let existing = assets_folder().join("already_here.png");
    write_existing_file(&existing, b"existing-data")?;
    let original_contents = fs::read(&existing)?;

    let editor = Editor {
        game: create_new_game(test_game.name().to_string()),
        ..Default::default()
    };
    let _guard = EditorServicesGuard::install(editor);
    RegisterAssetCmd::new(AssetKind::Sprite, &existing).execute();
    RegisterAssetCmd::new(AssetKind::Sprite, &existing).execute();

    assert_eq!(fs::read(&existing)?, original_contents);
    assert!(with_editor(|editor| editor
        .game
        .asset_registry
        .record(AssetKey::Sprite(SpriteId(1)))
        .is_some()));
    Ok(())
}

#[test]
fn undo_register_removes_identity_but_keeps_existing_file() -> Result<(), Box<dyn std::error::Error>> {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("undo_register");
    set_game_name(test_game.name());
    let existing = assets_folder().join("keep.png");
    write_existing_file(&existing, b"existing-data")?;

    let editor = Editor {
        game: create_new_game(test_game.name().to_string()),
        ..Default::default()
    };
    let _guard = EditorServicesGuard::install(editor);
    let mut command = RegisterAssetCmd::new(AssetKind::Sprite, &existing);
    command.execute();
    command.undo();

    assert_eq!(fs::read(&existing)?, b"existing-data");
    assert!(with_editor(|editor| editor
        .game
        .asset_registry
        .record(AssetKey::Sprite(SpriteId(1)))
        .is_none()));
    Ok(())
}

#[test]
fn register_rejects_file_outside_canonical_resource_folder_without_copying() -> Result<(), Box<dyn std::error::Error>> {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("register_wrong_folder");
    set_game_name(test_game.name());
    let existing = resources_folder_current().join("other/player.png");
    write_existing_file(&existing, b"existing-data")?;

    let editor = Editor {
        game: create_new_game(test_game.name().to_string()),
        ..Default::default()
    };
    let _guard = EditorServicesGuard::install(editor);
    RegisterAssetCmd::new(AssetKind::Sprite, &existing).execute();

    assert_eq!(fs::read(&existing)?, b"existing-data");
    assert!(with_editor(|editor| editor.game.asset_registry.records().is_empty()));
    Ok(())
}

#[test]
fn register_script_reuses_script_manager_identity() -> Result<(), Box<dyn std::error::Error>> {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("register_script");
    set_game_name(test_game.name());
    let existing = scripts_folder().join("player.lua");
    write_existing_file(&existing, b"return {}\n")?;

    let editor = Editor {
        game: create_new_game(test_game.name().to_string()),
        ..Default::default()
    };
    let _guard = EditorServicesGuard::install(editor);
    RegisterAssetCmd::new(AssetKind::Script, &existing).execute();

    assert!(with_editor(|editor| editor
        .game
        .asset_registry
        .record(AssetKey::Script(ScriptId(1)))
        .is_some()));
    Ok(())
}

fn write_existing_file(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "resource has no parent"))?;
    fs::create_dir_all(parent)?;
    fs::write(path, contents)
}
