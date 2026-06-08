use super::*;

#[test]
fn prefab_palette_state_round_trips_through_disk_helpers() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_palette_roundtrip");
    set_game_name(test_game.name());
    create_game_folders(test_game.name());

    let state = PrefabPaletteState {
        active_prefab_id: Some(PrefabId(7)),
        recent_prefab_ids: vec![
            PrefabId(10),
            PrefabId(9),
            PrefabId(8),
            PrefabId(7),
            PrefabId(6),
            PrefabId(5),
            PrefabId(4),
            PrefabId(3),
            PrefabId(2),
            PrefabId(1),
        ],
    };

    save_prefab_palette_state(test_game.name(), &state).unwrap();

    let loaded = load_prefab_palette_state(test_game.name()).unwrap();

    assert_eq!(loaded, state);
}

#[test]
fn load_prefab_palette_state_defaults_when_file_is_missing() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_palette_missing");

    let loaded = load_prefab_palette_state(test_game.name()).unwrap();

    assert_eq!(loaded, PrefabPaletteState::default());
}
