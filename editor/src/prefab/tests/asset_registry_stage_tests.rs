use super::*;
use std::path::Path;

const TREE_FILE: &str = "tree.png";
const ENEMY_SCRIPT_FILE: &str = "enemy.lua";

#[test]
fn prefab_stage_sync_editor_services_merges_asset_registry_metadata() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_asset_registry_stage");
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    game.asset_registry
        .insert(
            AssetKey::Sprite(SpriteId(3)),
            AssetRecord::new(PathBuf::from(paths::ASSETS_FOLDER).join(TREE_FILE)),
        )
        .unwrap();

    let mut stage = PrefabStage::from_editor_services(&game);
    assert!(stage
        .asset_registry
        .record(AssetKey::Sprite(SpriteId(3)))
        .is_some());
    stage
        .asset_registry
        .insert(
            AssetKey::Script(ScriptId(5)),
            AssetRecord::new(PathBuf::from(paths::SCRIPTS_FOLDER).join(ENEMY_SCRIPT_FILE)),
        )
        .unwrap();

    stage.sync_editor_services(&mut game).unwrap();

    assert!(game
        .asset_registry
        .record(AssetKey::Sprite(SpriteId(3)))
        .is_some());
    assert_eq!(
        game.asset_registry
            .record(AssetKey::Script(ScriptId(5)))
            .expect("script asset should be merged")
            .path,
        PathBuf::from(paths::SCRIPTS_FOLDER).join(ENEMY_SCRIPT_FILE)
    );
}

#[test]
fn prefab_stage_sync_editor_services_rebuilds_sprite_and_script_caches_from_asset_registry() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_asset_registry_manager_cache");
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    game.asset_registry
        .register_asset_relative_path(SpriteId(1), "sprites/building.png")
        .expect("sprite path should register");
    game.asset_registry
        .register_asset_relative_path(ScriptId(1), "building.lua")
        .expect("script path should register");

    let stage = PrefabStage::from_editor_services(&game);

    assert_eq!(
        stage.sprite_manager.path_for_id(SpriteId(1)),
        Some(Path::new("sprites/building.png"))
    );
    assert_eq!(
        stage.script_manager.path_for_id(ScriptId(1)),
        Some(Path::new("building.lua"))
    );
}
