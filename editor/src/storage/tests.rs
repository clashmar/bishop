use super::editor_storage::*;
use crate::editor_assets::write_prefabs_lua;
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::{ENGINE_DIR, PREFABS_FILE};
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
fn save_game_writes_prefabs_lua() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_lua_save");
    set_game_name(test_game.name());
    create_game_folders(test_game.name());

    let mut game = Game {
        name: test_game.name().to_string(),
        ..Default::default()
    };
    game.prefab_library.prefabs.insert(
        PrefabId(1),
        PrefabAsset {
            id: PrefabId(1),
            name: "Boss Attack".to_string(),
            next_node_id: 2,
            root_node_id: 1,
            nodes: vec![PrefabNode {
                node_id: 1,
                parent_node_id: None,
                components: vec![],
            }],
        },
    );

    save_game(&game).unwrap();

    let prefabs_path = scripts_folder().join(ENGINE_DIR).join(PREFABS_FILE);
    assert!(prefabs_path.is_file());
    let contents = std::fs::read_to_string(prefabs_path).unwrap();
    assert!(contents.contains("BossAttack = \"Boss Attack\""));
}

#[test]
fn save_game_rejects_duplicate_prefab_names() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_duplicate_names");
    set_game_name(test_game.name());
    create_game_folders(test_game.name());

    let mut game = Game {
        name: test_game.name().to_string(),
        ..Default::default()
    };
    let prefab_a = PrefabAsset {
        id: PrefabId(1),
        name: "Crate".to_string(),
        next_node_id: 2,
        root_node_id: 1,
        nodes: vec![PrefabNode {
            node_id: 1,
            parent_node_id: None,
            components: vec![],
        }],
    };
    let prefab_b = PrefabAsset {
        id: PrefabId(2),
        name: "Crate".to_string(),
        next_node_id: 2,
        root_node_id: 1,
        nodes: vec![PrefabNode {
            node_id: 1,
            parent_node_id: None,
            components: vec![],
        }],
    };
    game.prefab_library.prefabs.insert(prefab_a.id, prefab_a);
    game.prefab_library.prefabs.insert(prefab_b.id, prefab_b);

    let error = save_game(&game).unwrap_err();

    assert!(error.to_string().contains("duplicate prefab name"));
}

#[test]
fn write_prefabs_lua_sanitizes_collisions() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_lua_write");
    set_game_name(test_game.name());
    create_game_folders(test_game.name());

    write_prefabs_lua(
        &scripts_folder(),
        &[
            "Boss Attack".to_string(),
            "Boss-Attack".to_string(),
            "Crate".to_string(),
        ],
    )
    .unwrap();

    let prefabs_path = scripts_folder().join(ENGINE_DIR).join(PREFABS_FILE);
    let contents = std::fs::read_to_string(prefabs_path).unwrap();

    assert!(contents.contains("BossAttack = \"Boss Attack\""));
    assert!(contents.contains("BossAttack_2 = \"Boss-Attack\""));
    assert!(contents.contains("Crate = \"Crate\""));
}

#[test]
fn generated_lua_typings_hide_prefab_internal_components() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let components = std::fs::read_to_string(root.join("scripts/_engine/components.lua")).unwrap();
    let entity = std::fs::read_to_string(root.join("scripts/_engine/entity.lua")).unwrap();

    assert!(!components.contains("PrefabInstanceNode"));
    assert!(!components.contains("PrefabInstanceRoot"));
    assert!(!components.contains("PrefabOverrides"));
    assert!(!entity.contains("PrefabInstanceNode"));
    assert!(!entity.contains("PrefabInstanceRoot"));
    assert!(!entity.contains("PrefabOverrides"));
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
