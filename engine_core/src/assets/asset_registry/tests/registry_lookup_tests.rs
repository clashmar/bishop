use crate::assets::asset_registry::{AssetKey, AssetKind, AssetRecord, AssetRegistry};
use crate::constants::paths::{ASSETS_FOLDER, PREFABS_FOLDER, SCRIPTS_FOLDER};
use crate::ecs::{ScriptId, SpriteId};
use crate::prefab::PrefabId;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

fn asset_path(folder: &str, file: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(folder).join(file)
}

const SPRITE_RELATIVE_FOLDER: &str = "sprites";
const PLAYER_SPRITE_FILE: &str = "player.png";
const PLAYER_SCRIPT_FILE: &str = "player.lua";

fn canonical_sprite_relative_path() -> PathBuf {
    PathBuf::from(SPRITE_RELATIVE_FOLDER).join(PLAYER_SPRITE_FILE)
}

fn redundant_sprite_relative_path() -> PathBuf {
    PathBuf::from(format!("{SPRITE_RELATIVE_FOLDER}//{PLAYER_SPRITE_FILE}"))
}

#[test]
fn init_editor_metadata_rebuilds_path_lookup_from_records() {
    let mut registry = AssetRegistry::default();
    registry
        .insert(
            AssetKey::Sprite(SpriteId(1)),
            AssetRecord::new(
                AssetKind::Sprite,
                asset_path(ASSETS_FOLDER, PLAYER_SPRITE_FILE),
            ),
        )
        .unwrap();
    registry
        .insert(
            AssetKey::Script(ScriptId(2)),
            AssetRecord::new(
                AssetKind::Script,
                asset_path(SCRIPTS_FOLDER, PLAYER_SCRIPT_FILE),
            ),
        )
        .unwrap();

    registry.init_editor_metadata();

    assert_eq!(
        registry.key_for_path(asset_path(ASSETS_FOLDER, PLAYER_SPRITE_FILE)),
        Some(AssetKey::Sprite(SpriteId(1)))
    );
    assert_eq!(
        registry.key_for_path(asset_path(SCRIPTS_FOLDER, PLAYER_SCRIPT_FILE)),
        Some(AssetKey::Script(ScriptId(2)))
    );
}

#[test]
fn register_asset_relative_path_stores_canonical_assets_path_for_sprite_key() {
    let mut registry = AssetRegistry::default();
    let sprite_id = SpriteId(7);
    let relative_path = canonical_sprite_relative_path();

    registry
        .register_asset_relative_path(sprite_id, &relative_path)
        .unwrap();

    assert_eq!(
        registry.relative_path(sprite_id),
        Some(relative_path.clone())
    );
    assert_eq!(
        registry.record(AssetKey::Sprite(sprite_id)),
        Some(&AssetRecord::new(
            AssetKind::Sprite,
            asset_path(ASSETS_FOLDER, &relative_path),
        ))
    );
    assert_eq!(
        registry.key_for_path(asset_path(ASSETS_FOLDER, &relative_path)),
        Some(AssetKey::Sprite(sprite_id))
    );
    registry
        .register_asset_relative_path(sprite_id, &relative_path)
        .unwrap();
}

#[test]
fn register_asset_relative_path_rejects_absolute_sprite_path() {
    let mut registry = AssetRegistry::default();
    let absolute_path = std::env::current_dir()
        .expect("current directory should be available")
        .join(canonical_sprite_relative_path());

    let error = registry
        .register_asset_relative_path(SpriteId(1), absolute_path)
        .expect_err("absolute sprite paths should be rejected");

    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert!(registry.records().is_empty());
}

#[test]
fn insert_rejects_parent_dir_sprite_path() {
    let mut registry = AssetRegistry::default();
    let invalid_path = asset_path(ASSETS_FOLDER, "../player.png");

    let error = registry
        .insert(
            AssetKey::Sprite(SpriteId(1)),
            AssetRecord::new(AssetKind::Sprite, invalid_path),
        )
        .expect_err("traversal sprite paths should be rejected");

    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert!(registry.records().is_empty());
}

#[test]
fn register_asset_relative_path_stores_canonical_scripts_path_for_script_key() {
    let mut registry = AssetRegistry::default();
    let script_id = ScriptId(2);
    let relative_path = PathBuf::from("bullet.lua");
    registry
        .register_asset_relative_path(script_id, &relative_path)
        .unwrap();

    assert_eq!(
        registry.relative_path(script_id),
        Some(relative_path.clone())
    );
    assert_eq!(
        registry.record(AssetKey::Script(script_id)),
        Some(&AssetRecord::new(
            AssetKind::Script,
            asset_path(SCRIPTS_FOLDER, "bullet.lua"),
        ))
    );
    assert_eq!(
        registry.key_for_path(asset_path(SCRIPTS_FOLDER, "bullet.lua")),
        Some(AssetKey::Script(script_id))
    );
    registry
        .register_asset_relative_path(script_id, &relative_path)
        .unwrap();
}

#[test]
fn register_asset_relative_path_stores_canonical_prefabs_path_for_prefab_key() {
    let mut registry = AssetRegistry::default();
    let prefab_id = PrefabId(3);
    let relative_path = PathBuf::from("crate.ron");

    registry
        .register_asset_relative_path(prefab_id, &relative_path)
        .unwrap();

    assert_eq!(
        registry.relative_path(prefab_id),
        Some(relative_path.clone())
    );
    assert_eq!(
        registry.record(AssetKey::Prefab(prefab_id)),
        Some(&AssetRecord::new(
            AssetKind::Prefab,
            asset_path(PREFABS_FOLDER, "crate.ron"),
        ))
    );
    assert_eq!(
        registry.key_for_path(asset_path(PREFABS_FOLDER, "crate.ron")),
        Some(AssetKey::Prefab(prefab_id))
    );
}

#[test]
fn register_asset_relative_path_rejects_parent_dir_script_path() {
    let mut registry = AssetRegistry::default();

    let error = registry
        .register_asset_relative_path(ScriptId(1), "../player.lua")
        .expect_err("paths that escape the scripts folder should be rejected");

    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert!(registry.records().is_empty());
}

#[test]
fn register_asset_relative_path_normalizes_redundant_sprite_separators() {
    let mut registry = AssetRegistry::default();
    let sprite_id = SpriteId(1);
    let redundant_path = redundant_sprite_relative_path();
    let canonical_path = canonical_sprite_relative_path();

    registry
        .register_asset_relative_path(sprite_id, &redundant_path)
        .expect("redundant separators should normalize");

    assert_eq!(
        registry.relative_path(sprite_id),
        Some(canonical_path.clone())
    );
    assert_eq!(
        registry.record(AssetKey::Sprite(sprite_id)),
        Some(&AssetRecord::new(
            AssetKind::Sprite,
            asset_path(ASSETS_FOLDER, &canonical_path),
        ))
    );
}

#[test]
fn insert_rejects_absolute_script_path() {
    let mut registry = AssetRegistry::default();
    let invalid_path = std::env::current_dir()
        .expect("current directory should be available")
        .join("scripts/player.lua");

    let error = registry
        .insert(
            AssetKey::Script(ScriptId(1)),
            AssetRecord::new(AssetKind::Script, invalid_path),
        )
        .expect_err("absolute script paths should be rejected");

    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert!(registry.records().is_empty());
}

#[test]
fn register_asset_relative_path_rejects_absolute_prefab_path() {
    let mut registry = AssetRegistry::default();
    let absolute_path = std::env::current_dir()
        .expect("current directory should be available")
        .join("prefabs/crate.ron");

    let error = registry
        .register_asset_relative_path(PrefabId(4), absolute_path)
        .expect_err("absolute prefab paths should be rejected");

    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert!(registry.records().is_empty());
}

#[test]
#[should_panic(expected = "maps to both")]
fn init_editor_metadata_panics_for_duplicate_paths() {
    let mut registry = AssetRegistry::default();
    let duplicate_path = asset_path(ASSETS_FOLDER, "shared.png");

    registry.records_mut_for_test().insert(
        AssetKey::Sprite(SpriteId(1)),
        AssetRecord::new(AssetKind::Sprite, duplicate_path.clone()),
    );
    registry.records_mut_for_test().insert(
        AssetKey::Sprite(SpriteId(2)),
        AssetRecord::new(AssetKind::Sprite, duplicate_path),
    );

    registry.init_editor_metadata();
}

#[test]
fn try_init_editor_metadata_rejects_duplicate_paths() {
    let mut registry = AssetRegistry::default();
    let duplicate_path = asset_path(ASSETS_FOLDER, "shared.png");

    registry.records_mut_for_test().insert(
        AssetKey::Sprite(SpriteId(1)),
        AssetRecord::new(AssetKind::Sprite, duplicate_path.clone()),
    );
    registry.records_mut_for_test().insert(
        AssetKey::Sprite(SpriteId(2)),
        AssetRecord::new(AssetKind::Sprite, duplicate_path),
    );

    let error = registry
        .try_init_editor_metadata()
        .expect_err("duplicate paths should fail to rebuild metadata");

    assert_eq!(error.kind(), ErrorKind::InvalidData);
}
