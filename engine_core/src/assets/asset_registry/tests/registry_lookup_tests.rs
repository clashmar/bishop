use crate::assets::asset_registry::{AssetKey, AssetKind, AssetRecord, AssetRegistry};
use crate::constants::paths::{ASSETS_FOLDER, SCRIPTS_FOLDER};
use crate::ecs::{ScriptId, SpriteId};
use std::collections::HashMap;
use std::path::PathBuf;

fn asset_path(folder: &str, file: &str) -> PathBuf {
    PathBuf::from(folder).join(file)
}

const PLAYER_SPRITE_FILE: &str = "player.png";
const PLAYER_SCRIPT_FILE: &str = "player.lua";

#[test]
fn init_editor_metadata_rebuilds_path_lookup_from_records() {
    let mut registry = AssetRegistry::default();
    registry.records = HashMap::from([
        (
            AssetKey::Sprite(SpriteId(1)),
            AssetRecord::new(
                AssetKind::Sprite,
                asset_path(ASSETS_FOLDER, PLAYER_SPRITE_FILE),
            ),
        ),
        (
            AssetKey::Script(ScriptId(2)),
            AssetRecord::new(
                AssetKind::Script,
                asset_path(SCRIPTS_FOLDER, PLAYER_SCRIPT_FILE),
            ),
        ),
    ]);

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
