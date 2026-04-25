use crate::assets::asset_registry::{AssetKey, AssetRecord, AssetRegistry};
use crate::constants::paths::TEXT_FOLDER;
use crate::ecs::TomlId;
use std::io::ErrorKind;
use std::path::PathBuf;

#[test]
fn register_asset_relative_path_stores_canonical_text_path_for_toml_key() {
    let mut registry = AssetRegistry::default();
    let toml_id = TomlId(5);
    let relative_path = PathBuf::from("dialogue").join("npcs").join("npc.toml");

    registry
        .register_asset_relative_path(toml_id, &relative_path)
        .unwrap();

    assert_eq!(registry.relative_path(toml_id), Some(relative_path.clone()));
    assert_eq!(
        registry.record(AssetKey::Toml(toml_id)),
        Some(&AssetRecord::new(
            PathBuf::from(TEXT_FOLDER).join(&relative_path)
        ))
    );
}

#[test]
fn register_asset_relative_path_rejects_non_toml_text_path() {
    let mut registry = AssetRegistry::default();

    let error = registry
        .register_asset_relative_path(
            TomlId(6),
            PathBuf::from("dialogue").join("npcs").join("npc.txt"),
        )
        .expect_err("managed text assets must use toml files");

    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert!(registry.records().is_empty());
}

#[test]
fn register_asset_relative_path_accepts_language_prefixed_toml_path() {
    let mut registry = AssetRegistry::default();
    let toml_id = TomlId(7);
    let relative_path = PathBuf::from("en")
        .join("dialogue")
        .join("npcs")
        .join("npc.toml");

    registry
        .register_asset_relative_path(toml_id, &relative_path)
        .unwrap();

    assert_eq!(registry.relative_path(toml_id), Some(relative_path.clone()));
    assert_eq!(
        registry.record(AssetKey::Toml(toml_id)),
        Some(&AssetRecord::new(
            PathBuf::from(TEXT_FOLDER).join(&relative_path)
        ))
    );
}

#[test]
fn register_asset_relative_path_accepts_root_level_toml_path() {
    let mut registry = AssetRegistry::default();
    let toml_id = TomlId(8);
    let relative_path = PathBuf::from("_manifest.toml");

    registry
        .register_asset_relative_path(toml_id, &relative_path)
        .unwrap();

    assert_eq!(registry.relative_path(toml_id), Some(relative_path.clone()));
    assert_eq!(
        registry.record(AssetKey::Toml(toml_id)),
        Some(&AssetRecord::new(
            PathBuf::from(TEXT_FOLDER).join(&relative_path)
        ))
    );
}
