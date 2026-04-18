use crate::assets::asset_registry::{AssetKey, AssetKind, AssetRecord, AssetRegistry};
use crate::assets::AssetManager;
use crate::constants::paths::{ASSETS_FOLDER, PREFABS_FOLDER};
use crate::ecs::SpriteId;
use crate::prefab::PrefabId;
use std::io::ErrorKind;
use std::path::PathBuf;

fn asset_path(folder: &str, file: &str) -> PathBuf {
    PathBuf::from(folder).join(file)
}

#[test]
fn editor_metadata_snapshot_rebuilds_path_lookup_from_records() {
    let mut registry = AssetRegistry::default();
    registry.records.insert(
        AssetKey::Prefab(PrefabId(4)),
        AssetRecord::new(AssetKind::Prefab, asset_path(PREFABS_FOLDER, "crate.ron")),
    );

    let snapshot = registry.editor_metadata_snapshot();

    assert_eq!(
        snapshot.key_for_path(asset_path(PREFABS_FOLDER, "crate.ron")),
        Some(AssetKey::Prefab(PrefabId(4)))
    );
}

#[test]
fn insert_rejects_mismatched_key_and_kind() {
    let mut registry = AssetRegistry::default();

    let error = registry
        .insert(
            AssetKey::Sprite(SpriteId(1)),
            AssetRecord::new(AssetKind::Prefab, asset_path(ASSETS_FOLDER, "shared.png")),
        )
        .expect_err("insert should fail");

    assert_eq!(error.kind(), ErrorKind::InvalidData);
}

#[test]
fn merge_editor_metadata_from_rejects_conflicting_paths_for_same_key() {
    let mut destination = AssetRegistry::default();
    destination
        .insert(
            AssetKey::Prefab(PrefabId(4)),
            AssetRecord::new(AssetKind::Prefab, asset_path(PREFABS_FOLDER, "crate.ron")),
        )
        .unwrap();

    let mut source = AssetRegistry::default();
    source
        .insert(
            AssetKey::Prefab(PrefabId(4)),
            AssetRecord::new(AssetKind::Prefab, asset_path(PREFABS_FOLDER, "barrel.ron")),
        )
        .unwrap();

    let before = destination.clone();
    let error = destination
        .merge_editor_metadata_from(&source)
        .expect_err("merge should fail");

    assert_eq!(error.kind(), ErrorKind::InvalidData);
    assert_eq!(destination, before);
}

#[test]
fn merge_editor_metadata_from_rejects_conflicting_keys_for_same_path() {
    let mut destination = AssetRegistry::default();
    destination
        .insert(
            AssetKey::Sprite(SpriteId(1)),
            AssetRecord::new(AssetKind::Sprite, asset_path(ASSETS_FOLDER, "shared.png")),
        )
        .unwrap();

    let mut source = AssetRegistry::default();
    source
        .insert(
            AssetKey::Sprite(SpriteId(2)),
            AssetRecord::new(AssetKind::Sprite, asset_path(ASSETS_FOLDER, "shared.png")),
        )
        .unwrap();

    let before = destination.clone();
    let error = destination
        .merge_editor_metadata_from(&source)
        .expect_err("merge should fail");

    assert_eq!(error.kind(), ErrorKind::InvalidData);
    assert_eq!(destination, before);
}
