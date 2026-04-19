use super::*;
use crate::assets::{AssetKey, AssetRegistry};
use crate::constants::paths;
use crate::game::Game;
use crate::storage::path_utils::sanitise_name;
use crate::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use std::path::PathBuf;

#[test]
fn load_prefab_manager_skips_invalid_prefab_files() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_partial_load");
    let valid = create_prefab(PrefabId(1), "Valid".to_string());

    save_prefab(test_folder.name(), &valid).unwrap();
    fs::write(
        prefab_folder_for_game(test_folder.name()).join("broken.ron"),
        "not valid ron",
    )
    .unwrap();

    let manager = load_prefab_manager(test_folder.name(), &mut AssetRegistry::default()).unwrap();

    assert_eq!(manager.prefabs.len(), 1);
    assert_eq!(manager.prefabs.get(&valid.id), Some(&valid));
}

#[test]
fn load_prefab_manager_rejects_duplicate_prefab_ids() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_duplicate_ids");
    let prefab_id = PrefabId(7);
    let first = PrefabAsset {
        id: prefab_id,
        name: "First".to_string(),
        next_node_id: 2,
        root_node_id: 1,
        nodes: vec![PrefabNode {
            node_id: 1,
            parent_node_id: None,
            components: vec![],
        }],
    };
    let second = PrefabAsset {
        name: "Second".to_string(),
        ..first.clone()
    };
    let folder = prefab_folder_for_game(test_folder.name());
    fs::create_dir_all(&folder).unwrap();

    fs::write(folder.join("a_first.ron"), ron::to_string(&first).unwrap()).unwrap();
    fs::write(
        folder.join("z_second.ron"),
        ron::to_string(&second).unwrap(),
    )
    .unwrap();

    let error = load_prefab_manager(test_folder.name(), &mut AssetRegistry::default()).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidData);
}

#[test]
fn load_prefab_manager_skips_structurally_invalid_prefabs() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_invalid_structure");
    let valid = create_prefab(PrefabId(1), "Valid".to_string());
    let invalid = PrefabAsset {
        id: PrefabId(2),
        name: "Broken".to_string(),
        next_node_id: 2,
        root_node_id: 99,
        nodes: vec![PrefabNode {
            node_id: 1,
            parent_node_id: None,
            components: vec![],
        }],
    };
    let folder = prefab_folder_for_game(test_folder.name());

    save_prefab(test_folder.name(), &valid).unwrap();
    fs::write(
        folder.join("broken_structure.ron"),
        ron::to_string(&invalid).unwrap(),
    )
    .unwrap();

    let manager = load_prefab_manager(test_folder.name(), &mut AssetRegistry::default()).unwrap();

    assert_eq!(manager.prefabs.len(), 1);
    assert_eq!(manager.prefabs.get(&valid.id), Some(&valid));
}

#[test]
fn validate_prefab_rejects_disconnected_and_cyclic_graphs() {
    let disconnected = PrefabAsset {
        id: PrefabId(1),
        name: "Disconnected".to_string(),
        next_node_id: 3,
        root_node_id: 1,
        nodes: vec![
            PrefabNode {
                node_id: 1,
                parent_node_id: None,
                components: vec![],
            },
            PrefabNode {
                node_id: 2,
                parent_node_id: None,
                components: vec![],
            },
        ],
    };
    let cyclic = PrefabAsset {
        id: PrefabId(2),
        name: "Cycle".to_string(),
        next_node_id: 3,
        root_node_id: 1,
        nodes: vec![
            PrefabNode {
                node_id: 1,
                parent_node_id: Some(2),
                components: vec![],
            },
            PrefabNode {
                node_id: 2,
                parent_node_id: Some(1),
                components: vec![],
            },
        ],
    };

    assert!(validate_prefab(&disconnected).is_err());
    assert!(validate_prefab(&cyclic).is_err());
}

#[test]
fn validate_prefab_rejects_id_zero() {
    let prefab = create_prefab(PrefabId::default(), "Zero".to_string());

    assert!(validate_prefab(&prefab).is_err());
}

#[test]
fn save_prefab_rejects_id_zero() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_zero_id_save");
    let prefab = create_prefab(PrefabId::default(), "Zero".to_string());

    let error = save_prefab(test_folder.name(), &prefab).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidData);
}

#[test]
fn load_prefab_manager_restores_next_prefab_id_from_loaded_assets() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_next_id");
    let first = create_prefab(PrefabId(3), "First".to_string());
    let second = create_prefab(PrefabId(9), "Second".to_string());

    save_prefab(test_folder.name(), &first).unwrap();
    save_prefab(test_folder.name(), &second).unwrap();

    let mut manager =
        load_prefab_manager(test_folder.name(), &mut AssetRegistry::default()).unwrap();

    assert_eq!(manager.next_prefab_id, 10);
    assert_eq!(manager.allocate_prefab_id(), PrefabId(10));
    assert_eq!(manager.allocate_prefab_id(), PrefabId(11));
}

#[test]
fn load_prefab_manager_rejects_duplicate_prefab_names() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_duplicate_names");
    let first = create_prefab(PrefabId(3), "Bullet".to_string());
    let second = create_prefab(PrefabId(9), "Bullet".to_string());

    save_prefab(test_folder.name(), &first).unwrap();
    fs::write(
        prefab_folder_for_game(test_folder.name()).join("bullet_copy.ron"),
        ron::to_string(&second).unwrap(),
    )
    .unwrap();

    let error = load_prefab_manager(test_folder.name(), &mut AssetRegistry::default()).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidData);
}

#[test]
fn load_prefab_manager_supports_lookup_by_name() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_name_lookup");
    let prefab = create_prefab(PrefabId(5), "Bullet".to_string());

    save_prefab(test_folder.name(), &prefab).unwrap();

    let manager = load_prefab_manager(test_folder.name(), &mut AssetRegistry::default()).unwrap();

    assert_eq!(manager.prefab_named("Bullet"), Some(&prefab));
    assert_eq!(manager.prefab_named("Missing"), None);
}

#[test]
fn load_prefab_manager_registers_prefab_paths_from_disk() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_registry_register_on_load");
    let prefab = create_prefab(PrefabId(5), "Big Crate".to_string());
    let prefab_file_name = "custom_prefab_path.ron";
    let prefab_path = prefab_folder_for_game(test_folder.name()).join(prefab_file_name);
    let expected_path = PathBuf::from(paths::PREFABS_FOLDER).join(prefab_file_name);
    let mut game = Game {
        name: test_folder.name().to_string(),
        ..Default::default()
    };

    fs::create_dir_all(prefab_folder_for_game(test_folder.name())).unwrap();
    fs::write(&prefab_path, ron::to_string(&prefab).unwrap()).unwrap();

    game.reload_prefab_manager();

    assert_eq!(game.prefab_manager.prefabs.get(&prefab.id), Some(&prefab));
    assert_eq!(
        game.asset_registry.relative_path(prefab.id),
        Some(PathBuf::from(prefab_file_name))
    );
    assert_eq!(
        game.asset_registry.key_for_path(&expected_path),
        Some(AssetKey::Prefab(prefab.id))
    );
}

#[test]
fn load_prefab_manager_removes_stale_prefab_records_after_successful_reload() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_registry_cleanup_on_load");
    let prefab = create_prefab(PrefabId(5), "Crate".to_string());
    let stale_prefab_id = PrefabId(9);
    let stale_path = PathBuf::from(paths::PREFABS_FOLDER).join("stale_prefab.ron");
    let mut game = Game {
        name: test_folder.name().to_string(),
        ..Default::default()
    };

    save_prefab(test_folder.name(), &prefab).unwrap();
    game.asset_registry
        .register_asset_relative_path(stale_prefab_id, "stale_prefab.ron")
        .unwrap();

    game.reload_prefab_manager();

    assert_eq!(game.prefab_manager.prefabs.get(&prefab.id), Some(&prefab));
    assert_eq!(
        game.asset_registry
            .record(AssetKey::Prefab(stale_prefab_id)),
        None
    );
    assert_eq!(game.asset_registry.key_for_path(&stale_path), None);
    assert_eq!(
        game.asset_registry.relative_path(prefab.id),
        Some(PathBuf::from(format!(
            "{}.ron",
            sanitise_name(&prefab.name)
        )))
    );
}

#[test]
fn load_prefab_manager_reuses_paths_owned_by_stale_prefab_records() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_registry_stale_path_reuse");
    let prefab = create_prefab(PrefabId(5), "Crate".to_string());
    let prefab_relative_path = PathBuf::from(format!("{}.ron", sanitise_name(&prefab.name)));
    let expected_path = PathBuf::from(paths::PREFABS_FOLDER).join(&prefab_relative_path);
    let stale_prefab_id = PrefabId(9);
    let mut game = Game {
        name: test_folder.name().to_string(),
        ..Default::default()
    };

    save_prefab(test_folder.name(), &prefab).unwrap();
    game.asset_registry
        .register_asset_relative_path(stale_prefab_id, &prefab_relative_path)
        .unwrap();

    game.reload_prefab_manager();

    assert_eq!(game.prefab_manager.prefabs.get(&prefab.id), Some(&prefab));
    assert_eq!(
        game.asset_registry
            .record(AssetKey::Prefab(stale_prefab_id)),
        None
    );
    assert_eq!(
        game.asset_registry.key_for_path(&expected_path),
        Some(AssetKey::Prefab(prefab.id))
    );
}

#[test]
fn load_prefab_manager_keeps_existing_prefab_records_when_reload_fails() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_registry_no_partial_cleanup");
    let first = create_prefab(PrefabId(3), "Bullet".to_string());
    let second = create_prefab(PrefabId(9), "Bullet".to_string());
    let stale_prefab_id = PrefabId(22);
    let stale_relative_path = PathBuf::from("stale_prefab.ron");
    let mut game = Game {
        name: test_folder.name().to_string(),
        ..Default::default()
    };

    save_prefab(test_folder.name(), &first).unwrap();
    fs::write(
        prefab_folder_for_game(test_folder.name()).join("bullet_copy.ron"),
        ron::to_string(&second).unwrap(),
    )
    .unwrap();
    game.asset_registry
        .register_asset_relative_path(stale_prefab_id, &stale_relative_path)
        .unwrap();
    game.prefab_manager.prefabs.insert(first.id, first.clone());
    game.prefab_manager.rebuild_name_lookup().unwrap();
    let before = game.asset_registry.clone();
    let before_prefab_manager = game.prefab_manager.clone();

    game.reload_prefab_manager();

    assert_eq!(game.asset_registry, before);
    assert_eq!(game.prefab_manager, before_prefab_manager);
}

#[test]
fn save_prefab_uses_prefab_name_for_filename() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_name_filename");
    let prefab = create_prefab(PrefabId(5), "Big Crate".to_string());

    save_prefab(test_folder.name(), &prefab).unwrap();

    let expected_path = prefab_folder_for_game(test_folder.name())
        .join(format!("{}.ron", sanitise_name(&prefab.name)));
    assert!(expected_path.is_file());
    assert!(!prefab_folder_for_game(test_folder.name())
        .join("5.ron")
        .exists());
}

#[test]
fn save_prefab_renames_existing_file_when_name_changes() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_rename_filename");
    let prefab_id = PrefabId(5);
    let first = create_prefab(prefab_id, "Big Crate".to_string());
    let second = create_prefab(prefab_id, "Huge Barrel".to_string());

    save_prefab(test_folder.name(), &first).unwrap();
    save_prefab(test_folder.name(), &second).unwrap();

    let first_path = prefab_folder_for_game(test_folder.name())
        .join(format!("{}.ron", sanitise_name(&first.name)));
    let second_path = prefab_folder_for_game(test_folder.name())
        .join(format!("{}.ron", sanitise_name(&second.name)));
    assert!(!first_path.exists());
    assert!(second_path.is_file());
    assert_eq!(load_prefab(test_folder.name(), prefab_id).unwrap(), second);
}
