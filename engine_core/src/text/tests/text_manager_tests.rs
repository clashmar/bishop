use crate::assets::AssetRegistry;
use crate::prelude::*;
use crate::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use crate::text::{SelectionMode, TextEntry, TextFile};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const DIALOGUE_KEY: &str = "greeting";
const ENGLISH_TEXT: &str = "Hello";
const SPANISH_TEXT: &str = "Hola";
const SPANISH_EXHAUSTED_TEXT: &str = "Se acabaron";
const DIALOGUE_RELATIVE_PATH: &str = "dialogue/npcs/npc.toml";
const MANIFEST_FILE: &str = "_manifest.toml";
const ENGLISH_LANGUAGE: &str = "en";
const SPANISH_LANGUAGE: &str = "es";

fn write_text_entry(root: &std::path::Path, language: &str, relative_path: &str, entry: TextEntry) {
    let file_path = root.join(language).join(relative_path);
    fs::create_dir_all(
        file_path
            .parent()
            .expect("text file path should have a parent"),
    )
    .expect("text file parent should be creatable");
    let mut entries = HashMap::new();
    entries.insert(DIALOGUE_KEY.to_string(), entry);
    fs::write(
        file_path,
        toml::to_string(&TextFile { entries }).expect("text file should serialize"),
    )
    .expect("text file should be writable");
}

fn write_text_file(root: &std::path::Path, language: &str, relative_path: &str, text: &str) {
    write_text_entry(
        root,
        language,
        relative_path,
        TextEntry {
            variants: vec![text.to_string()],
            ..Default::default()
        },
    );
}

#[test]
fn text_manager_toml_id_loads_current_language_file() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let folder = TestGameFolder::new("text_manager_toml_id_lookup");
    set_game_name(folder.name());

    let text_root = text_folder();
    fs::create_dir_all(&text_root).expect("text root should be creatable");
    fs::write(
        text_root.join(MANIFEST_FILE),
        format!(
            "default_language = \"{ENGLISH_LANGUAGE}\"\navailable = [\"{ENGLISH_LANGUAGE}\"]\n"
        ),
    )
    .expect("manifest should be writable");
    write_text_file(
        &text_root,
        ENGLISH_LANGUAGE,
        DIALOGUE_RELATIVE_PATH,
        ENGLISH_TEXT,
    );

    let mut registry = AssetRegistry::default();
    let toml_id = TomlId(9);
    registry
        .register_asset_relative_path(toml_id, PathBuf::from(DIALOGUE_RELATIVE_PATH))
        .expect("dialogue asset should register");

    let manager = TextManager::new(text_root.clone());

    assert_eq!(
        manager.select_text(&registry, toml_id, DIALOGUE_KEY),
        Some(ENGLISH_TEXT.to_string())
    );
}

#[test]
fn text_manager_toml_id_switches_languages_without_changing_id() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let folder = TestGameFolder::new("text_manager_toml_id_language_switch");
    set_game_name(folder.name());

    let text_root = text_folder();
    fs::create_dir_all(&text_root).expect("text root should be creatable");
    fs::write(
        text_root.join(MANIFEST_FILE),
        format!(
            "default_language = \"{ENGLISH_LANGUAGE}\"\navailable = [\"{ENGLISH_LANGUAGE}\", \"{SPANISH_LANGUAGE}\"]\n"
        ),
    )
    .expect("manifest should be writable");
    write_text_file(
        &text_root,
        ENGLISH_LANGUAGE,
        DIALOGUE_RELATIVE_PATH,
        ENGLISH_TEXT,
    );
    write_text_file(
        &text_root,
        SPANISH_LANGUAGE,
        DIALOGUE_RELATIVE_PATH,
        SPANISH_TEXT,
    );

    let mut registry = AssetRegistry::default();
    let toml_id = TomlId(10);
    registry
        .register_asset_relative_path(toml_id, PathBuf::from(DIALOGUE_RELATIVE_PATH))
        .expect("dialogue asset should register");

    let mut manager = TextManager::new(text_root);
    assert_eq!(
        manager.select_text(&registry, toml_id, DIALOGUE_KEY),
        Some(ENGLISH_TEXT.to_string())
    );

    assert!(manager.set_language(SPANISH_LANGUAGE));
    assert_eq!(
        manager.select_text(&registry, toml_id, DIALOGUE_KEY),
        Some(SPANISH_TEXT.to_string())
    );
}

#[test]
fn text_manager_toml_id_resets_selection_state_when_language_changes() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let folder = TestGameFolder::new("text_manager_toml_id_state_reset");
    set_game_name(folder.name());

    let text_root = text_folder();
    fs::create_dir_all(&text_root).expect("text root should be creatable");
    fs::write(
        text_root.join(MANIFEST_FILE),
        format!(
            "default_language = \"{ENGLISH_LANGUAGE}\"\navailable = [\"{ENGLISH_LANGUAGE}\", \"{SPANISH_LANGUAGE}\"]\n"
        ),
    )
    .expect("manifest should be writable");
    write_text_entry(
        &text_root,
        ENGLISH_LANGUAGE,
        DIALOGUE_RELATIVE_PATH,
        TextEntry {
            selection: SelectionMode::Once,
            exhausted: None,
            variants: vec![ENGLISH_TEXT.to_string()],
        },
    );
    write_text_entry(
        &text_root,
        SPANISH_LANGUAGE,
        DIALOGUE_RELATIVE_PATH,
        TextEntry {
            selection: SelectionMode::Once,
            exhausted: Some(SPANISH_EXHAUSTED_TEXT.to_string()),
            variants: vec![SPANISH_TEXT.to_string()],
        },
    );

    let mut registry = AssetRegistry::default();
    let toml_id = TomlId(11);
    registry
        .register_asset_relative_path(toml_id, PathBuf::from(DIALOGUE_RELATIVE_PATH))
        .expect("dialogue asset should register");

    let mut manager = TextManager::new(text_root);
    assert_eq!(
        manager.select_text(&registry, toml_id, DIALOGUE_KEY),
        Some(ENGLISH_TEXT.to_string())
    );

    assert!(manager.set_language(SPANISH_LANGUAGE));
    assert_eq!(
        manager.select_text(&registry, toml_id, DIALOGUE_KEY),
        Some(SPANISH_TEXT.to_string())
    );
}

#[test]
fn text_manager_toml_id_preserves_state_when_reapplying_same_language() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let folder = TestGameFolder::new("text_manager_toml_id_same_language");
    set_game_name(folder.name());

    let text_root = text_folder();
    fs::create_dir_all(&text_root).expect("text root should be creatable");
    fs::write(
        text_root.join(MANIFEST_FILE),
        format!(
            "default_language = \"{ENGLISH_LANGUAGE}\"\navailable = [\"{ENGLISH_LANGUAGE}\"]\n"
        ),
    )
    .expect("manifest should be writable");
    write_text_entry(
        &text_root,
        ENGLISH_LANGUAGE,
        DIALOGUE_RELATIVE_PATH,
        TextEntry {
            selection: SelectionMode::Once,
            exhausted: Some(SPANISH_EXHAUSTED_TEXT.to_string()),
            variants: vec![ENGLISH_TEXT.to_string()],
        },
    );

    let mut registry = AssetRegistry::default();
    let toml_id = TomlId(12);
    registry
        .register_asset_relative_path(toml_id, PathBuf::from(DIALOGUE_RELATIVE_PATH))
        .expect("dialogue asset should register");

    let mut manager = TextManager::new(text_root);
    assert_eq!(
        manager.select_text(&registry, toml_id, DIALOGUE_KEY),
        Some(ENGLISH_TEXT.to_string())
    );

    assert!(manager.set_language(ENGLISH_LANGUAGE));
    assert_eq!(
        manager.select_text(&registry, toml_id, DIALOGUE_KEY),
        Some(SPANISH_EXHAUSTED_TEXT.to_string())
    );
}

#[test]
fn text_manager_toml_id_reapplies_same_language_and_refreshes_cached_files() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let folder = TestGameFolder::new("text_manager_toml_id_same_language_refresh");
    set_game_name(folder.name());

    let text_root = text_folder();
    fs::create_dir_all(&text_root).expect("text root should be creatable");
    fs::write(
        text_root.join(MANIFEST_FILE),
        format!(
            "default_language = \"{ENGLISH_LANGUAGE}\"\navailable = [\"{ENGLISH_LANGUAGE}\"]\n"
        ),
    )
    .expect("manifest should be writable");
    write_text_file(
        &text_root,
        ENGLISH_LANGUAGE,
        DIALOGUE_RELATIVE_PATH,
        ENGLISH_TEXT,
    );

    let mut registry = AssetRegistry::default();
    let toml_id = TomlId(13);
    registry
        .register_asset_relative_path(toml_id, PathBuf::from(DIALOGUE_RELATIVE_PATH))
        .expect("dialogue asset should register");

    let mut manager = TextManager::new(text_root.clone());
    assert_eq!(
        manager.select_text(&registry, toml_id, DIALOGUE_KEY),
        Some(ENGLISH_TEXT.to_string())
    );

    write_text_file(
        &text_root,
        ENGLISH_LANGUAGE,
        DIALOGUE_RELATIVE_PATH,
        SPANISH_TEXT,
    );
    assert!(manager.set_language(ENGLISH_LANGUAGE));
    assert_eq!(
        manager.select_text(&registry, toml_id, DIALOGUE_KEY),
        Some(SPANISH_TEXT.to_string())
    );
}

#[test]
fn text_manager_toml_id_resolves_language_prefixed_registry_paths() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let folder = TestGameFolder::new("text_manager_toml_id_language_prefixed_path");
    set_game_name(folder.name());

    let text_root = text_folder();
    fs::create_dir_all(&text_root).expect("text root should be creatable");
    fs::write(
        text_root.join(MANIFEST_FILE),
        format!(
            "default_language = \"{ENGLISH_LANGUAGE}\"\navailable = [\"{ENGLISH_LANGUAGE}\", \"{SPANISH_LANGUAGE}\"]\n"
        ),
    )
    .expect("manifest should be writable");
    write_text_file(
        &text_root,
        ENGLISH_LANGUAGE,
        DIALOGUE_RELATIVE_PATH,
        ENGLISH_TEXT,
    );
    write_text_file(
        &text_root,
        SPANISH_LANGUAGE,
        DIALOGUE_RELATIVE_PATH,
        SPANISH_TEXT,
    );

    let mut registry = AssetRegistry::default();
    let toml_id = TomlId(14);
    registry
        .register_asset_relative_path(
            toml_id,
            PathBuf::from(ENGLISH_LANGUAGE).join(DIALOGUE_RELATIVE_PATH),
        )
        .expect("dialogue asset should register");

    let mut manager = TextManager::new(text_root);
    assert_eq!(
        manager.select_text(&registry, toml_id, DIALOGUE_KEY),
        Some(ENGLISH_TEXT.to_string())
    );

    assert!(manager.set_language(SPANISH_LANGUAGE));
    assert_eq!(
        manager.select_text(&registry, toml_id, DIALOGUE_KEY),
        Some(SPANISH_TEXT.to_string())
    );
}
