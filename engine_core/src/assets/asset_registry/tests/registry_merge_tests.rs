use crate::assets::asset_registry::{AssetKey, AssetRecord, AssetRegistry};
use crate::assets::AssetManager;
use crate::constants::extensions;
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
    registry
        .insert(
            AssetKey::Prefab(PrefabId(4)),
            AssetRecord::new(asset_path(
                PREFABS_FOLDER,
                &format!("crate.{}", extensions::PREFAB),
            )),
        )
        .unwrap();

    let snapshot = registry.editor_metadata_snapshot();

    assert_eq!(
        snapshot.key_for_path(asset_path(
            PREFABS_FOLDER,
            &format!("crate.{}", extensions::PREFAB),
        )),
        Some(AssetKey::Prefab(PrefabId(4)))
    );
}

#[test]
fn insert_rejects_prefab_path_for_sprite_key() {
    let mut registry = AssetRegistry::default();

    let error = registry
        .insert(
            AssetKey::Sprite(SpriteId(1)),
            AssetRecord::new(asset_path(
                PREFABS_FOLDER,
                &format!("shared.{}", extensions::PREFAB),
            )),
        )
        .expect_err("insert should fail");

    assert_eq!(error.kind(), ErrorKind::InvalidInput);
}

#[test]
fn merge_editor_metadata_from_rejects_conflicting_paths_for_same_key() {
    let mut destination = AssetRegistry::default();
    destination
        .insert(
            AssetKey::Prefab(PrefabId(4)),
            AssetRecord::new(asset_path(
                PREFABS_FOLDER,
                &format!("crate.{}", extensions::PREFAB),
            )),
        )
        .unwrap();

    let mut source = AssetRegistry::default();
    source
        .insert(
            AssetKey::Prefab(PrefabId(4)),
            AssetRecord::new(asset_path(
                PREFABS_FOLDER,
                &format!("barrel.{}", extensions::PREFAB),
            )),
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
fn replace_record_allows_same_prefab_key_to_move_to_a_new_path() {
    let mut registry = AssetRegistry::default();
    let key = AssetKey::Prefab(PrefabId(4));
    let old_path = asset_path(PREFABS_FOLDER, &format!("crate.{}", extensions::PREFAB));
    let new_path = asset_path(PREFABS_FOLDER, &format!("barrel.{}", extensions::PREFAB));

    registry
        .insert(key, AssetRecord::new(old_path.clone()))
        .unwrap();

    registry
        .replace_record(key, AssetRecord::new(new_path.clone()))
        .unwrap();

    assert_eq!(registry.key_for_path(&old_path), None);
    assert_eq!(registry.key_for_path(&new_path), Some(key));
    assert_eq!(
        registry.record(key).map(|record| &record.path),
        Some(&new_path)
    );
}

#[test]
fn replace_record_rejects_path_owned_by_different_key() {
    let mut registry = AssetRegistry::default();
    let first_key = AssetKey::Prefab(PrefabId(4));
    let second_key = AssetKey::Prefab(PrefabId(8));
    let first_path = asset_path(PREFABS_FOLDER, &format!("crate.{}", extensions::PREFAB));
    let second_path = asset_path(PREFABS_FOLDER, &format!("barrel.{}", extensions::PREFAB));

    registry
        .insert(first_key, AssetRecord::new(first_path.clone()))
        .unwrap();
    registry
        .insert(second_key, AssetRecord::new(second_path.clone()))
        .unwrap();

    let before = registry.clone();
    let error = registry
        .replace_record(first_key, AssetRecord::new(second_path.clone()))
        .expect_err("replace_record should fail");

    assert_eq!(error.kind(), ErrorKind::InvalidData);
    assert_eq!(registry, before);
}

#[test]
fn remove_record_clears_record_and_path_lookup() {
    let mut registry = AssetRegistry::default();
    let key = AssetKey::Prefab(PrefabId(4));
    let path = asset_path(PREFABS_FOLDER, &format!("crate.{}", extensions::PREFAB));
    let record = AssetRecord::new(path.clone());

    registry.insert(key, record.clone()).unwrap();

    assert_eq!(registry.remove_record(key), Some(record));
    assert_eq!(registry.record(key), None);
    assert_eq!(registry.key_for_path(&path), None);
}

#[test]
fn remove_record_missing_key_returns_none_and_preserves_lookup() {
    let mut registry = AssetRegistry::default();
    let present_key = AssetKey::Prefab(PrefabId(4));
    let missing_key = AssetKey::Prefab(PrefabId(8));
    let path = asset_path(PREFABS_FOLDER, &format!("crate.{}", extensions::PREFAB));

    registry
        .insert(present_key, AssetRecord::new(path.clone()))
        .unwrap();

    let before = registry.clone();

    assert_eq!(registry.remove_record(missing_key), None);
    assert_eq!(registry, before);
    assert_eq!(registry.key_for_path(&path), Some(present_key));
}

#[test]
fn merge_editor_metadata_from_rejects_conflicting_keys_for_same_path() {
    let mut destination = AssetRegistry::default();
    destination
        .insert(
            AssetKey::Sprite(SpriteId(1)),
            AssetRecord::new(asset_path(ASSETS_FOLDER, "shared.png")),
        )
        .unwrap();

    let mut source = AssetRegistry::default();
    source
        .insert(
            AssetKey::Sprite(SpriteId(2)),
            AssetRecord::new(asset_path(ASSETS_FOLDER, "shared.png")),
        )
        .unwrap();

    let before = destination.clone();
    let error = destination
        .merge_editor_metadata_from(&source)
        .expect_err("merge should fail");

    assert_eq!(error.kind(), ErrorKind::InvalidData);
    assert_eq!(destination, before);
}
