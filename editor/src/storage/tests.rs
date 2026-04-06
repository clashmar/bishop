use super::editor_storage::*;
use engine_core::prelude::*;
use engine_core::storage::path_utils::sanitise_name;
use engine_core::storage::test_utils::{TestGameFolder, game_fs_test_lock};

#[test]
fn create_new_game_creates_prefabs_folder() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_folder");

    let _game = create_new_game(test_game.name().to_string());

    assert!(prefabs_folder().is_dir());
}

#[test]
fn prefab_storage_round_trips_through_disk_helpers() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_roundtrip");
    set_game_name(test_game.name());
    create_game_folders(test_game.name());

    let prefab = PrefabAsset {
        id: PrefabId(1),
        name: "Crate".to_string(),
        next_node_id: 3,
        root_node_id: 1,
        nodes: vec![
            PrefabNode {
                node_id: 1,
                parent_node_id: None,
                components: vec![ComponentSnapshot {
                    type_name: "Name".to_string(),
                    ron: "(\"Root\")".to_string(),
                }],
            },
            PrefabNode {
                node_id: 2,
                parent_node_id: Some(1),
                components: vec![ComponentSnapshot {
                    type_name: "Name".to_string(),
                    ron: "(\"Child\")".to_string(),
                }],
            },
        ],
    };

    save_prefab(test_game.name(), &prefab).unwrap();

    let expected_path = prefabs_folder().join(format!("{}.ron", sanitise_name(&prefab.name)));
    assert!(expected_path.is_file());

    let loaded = load_prefab(test_game.name(), prefab.id).unwrap();
    let listed = list_prefabs(test_game.name()).unwrap();

    assert_eq!(loaded, prefab);
    assert_eq!(listed, vec![prefab.clone()]);
    assert_eq!(
        load_prefab_library(test_game.name())
            .unwrap()
            .prefabs
            .get(&prefab.id),
        Some(&prefab)
    );

    assert!(delete_prefab(test_game.name(), prefab.id).unwrap());
    assert!(list_prefabs(test_game.name()).unwrap().is_empty());
}

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
