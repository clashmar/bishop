use super::*;

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
            AssetRecord::new(
                AssetKind::Sprite,
                PathBuf::from(paths::ASSETS_FOLDER).join(TREE_FILE),
            ),
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
            AssetRecord::new(
                AssetKind::Script,
                PathBuf::from(paths::SCRIPTS_FOLDER).join(ENEMY_SCRIPT_FILE),
            ),
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
