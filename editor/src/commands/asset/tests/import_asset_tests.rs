use crate::commands::asset::ImportAssetCmd;
use crate::commands::editor_command_manager::EditorCommand;
use crate::editor_global::with_editor;
use crate::storage::game_io::{create_new_game, load_game_by_name};
use crate::test_utils::{game_fs_test_lock, EditorServicesGuard, TestGameFolder};
use crate::Editor;
use bishop::text::FontId;
use engine_core::assets::{AssetKey, AssetKind};
use engine_core::constants::{extensions, paths};
use engine_core::ecs::SpriteId;
use engine_core::engine_global::set_game_name;
use engine_core::storage::path_utils::{assets_folder, fonts_folder, resources_folder_current};
use std::fs;
use std::io;
use std::path::Path;

#[test]
fn import_copies_sprite_to_canonical_folder_and_registers_it() -> Result<(), Box<dyn std::error::Error>> {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("import_sprite");
    set_game_name(test_game.name());
    let file_name = format!("player.{}", extensions::PNG);
    let source = test_game.path().join("chosen").join(&file_name);
    write_source(&source, b"sprite-data")?;

    let editor = Editor {
        game: create_new_game(test_game.name().to_string()),
        ..Default::default()
    };
    let _guard = EditorServicesGuard::install(editor);
    let mut command = ImportAssetCmd::new(AssetKind::Sprite, &source);
    command.execute();
    ImportAssetCmd::new(AssetKind::Sprite, &source).execute();

    let imported = assets_folder().join(&file_name);
    assert_eq!(fs::read(imported)?, b"sprite-data");
    assert!(with_editor(|editor| editor
        .game
        .asset_registry
        .record(AssetKey::Sprite(SpriteId(1)))
        .is_some()));
    assert_eq!(with_editor(|editor| editor.game.asset_registry.records().len()), 1);
    Ok(())
}

#[test]
fn undo_import_removes_the_copy_and_registry_record() -> Result<(), Box<dyn std::error::Error>> {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("undo_import");
    set_game_name(test_game.name());
    let file_name = format!("undo.{}", extensions::PNG);
    let source = test_game.path().join("chosen").join(&file_name);
    write_source(&source, b"sprite-data")?;

    let editor = Editor {
        game: create_new_game(test_game.name().to_string()),
        ..Default::default()
    };
    let _guard = EditorServicesGuard::install(editor);
    let imported = assets_folder().join(&file_name);
    let mut command = ImportAssetCmd::new(AssetKind::Sprite, &source);
    command.execute();
    command.undo();

    assert!(!imported.exists());
    assert!(with_editor(|editor| editor
        .game
        .asset_registry
        .record(AssetKey::Sprite(SpriteId(1)))
        .is_none()));
    Ok(())
}

#[test]
fn import_keeps_file_in_selected_nested_resource_directory() -> Result<(), Box<dyn std::error::Error>> {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("import_nested_resource_directory");
    set_game_name(test_game.name());
    let file_name = format!("explosion.{}", extensions::WAV);
    let source = test_game.path().join("chosen").join(&file_name);
    write_source(&source, b"sound-data")?;

    let editor = Editor {
        game: create_new_game(test_game.name().to_string()),
        ..Default::default()
    };
    let _guard = EditorServicesGuard::install(editor);
    let destination = resources_folder_current()
        .join(paths::AUDIO_FOLDER)
        .join(paths::SFX_FOLDER);
    fs::create_dir_all(&destination)?;

    ImportAssetCmd::new_in_directory(AssetKind::Sound, &source, &destination).execute();

    let imported = destination.join(&file_name);
    assert_eq!(fs::read(&imported)?, b"sound-data");
    assert!(with_editor(|editor| editor
        .game
        .asset_registry
        .key_for_full_path(&imported)
        .is_some()));
    assert!(!resources_folder_current()
        .join(paths::AUDIO_FOLDER)
        .join(&file_name)
        .exists());
    Ok(())
}

#[test]
fn import_rejects_unsupported_extension_without_leaving_a_copy() -> Result<(), Box<dyn std::error::Error>> {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("import_invalid_extension");
    set_game_name(test_game.name());
    let source = test_game.path().join("chosen").join("Display.woff");
    write_source(&source, b"not-supported")?;

    let editor = Editor {
        game: create_new_game(test_game.name().to_string()),
        ..Default::default()
    };
    let _guard = EditorServicesGuard::install(editor);
    ImportAssetCmd::new(AssetKind::Font, &source).execute();

    assert!(!fonts_folder().join("Display.woff").exists());
    assert!(with_editor(|editor| editor.game.asset_registry.records().is_empty()));
    Ok(())
}

#[test]
fn import_font_uses_font_folder_and_persists_registry() -> Result<(), Box<dyn std::error::Error>> {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("import_font");
    set_game_name(test_game.name());
    let file_name = format!("Display.{}", extensions::OTF);
    let source = test_game.path().join("chosen").join(&file_name);
    write_source(&source, b"font-data")?;

    let editor = Editor {
        game: create_new_game(test_game.name().to_string()),
        ..Default::default()
    };
    let _guard = EditorServicesGuard::install(editor);
    ImportAssetCmd::new(AssetKind::Font, &source).execute();

    let imported = fonts_folder().join(&file_name);
    assert_eq!(fs::read(imported)?, b"font-data");
    let saved_game = load_game_by_name(test_game.name())?;
    assert!(saved_game
        .asset_registry
        .record(AssetKey::Font(FontId(1)))
        .is_some());
    Ok(())
}

fn write_source(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "source has no parent"))?;
    fs::create_dir_all(parent)?;
    fs::write(path, contents)
}
