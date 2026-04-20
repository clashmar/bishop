use super::*;

#[test]
fn save_game_round_trips_toml_asset_registry_records() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("toml_asset_registry_roundtrip");
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    let toml_id = TomlId(4);
    let relative_path = PathBuf::from("dialogue").join("npcs").join("npc.toml");

    game.asset_registry
        .register_asset_relative_path(toml_id, &relative_path)
        .unwrap();

    save_game(&game).unwrap();
    let loaded = load_game_by_name(test_game.name()).unwrap();

    assert_eq!(
        loaded.asset_registry.relative_path(toml_id),
        Some(relative_path)
    );
    assert_eq!(
        loaded.asset_registry.record(AssetKey::Toml(toml_id)),
        Some(&AssetRecord::new(
            AssetKind::Toml,
            PathBuf::from(paths::TEXT_FOLDER)
                .join("dialogue")
                .join("npcs")
                .join("npc.toml"),
        ))
    );
}
