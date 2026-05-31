use super::*;
use crate::assets::{AssetKey, AssetRegistry};
use crate::constants::{extensions, paths};
use crate::ecs::capture::ComponentSnapshot;
use crate::ecs::component::comp_type_name;
use crate::ecs::Name;
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

    persist_prefab(test_folder.name(), &valid, &AssetRegistry::default(), None).unwrap();
    fs::write(
        prefab_folder_for_game(test_folder.name()).join(format!("broken.{}", extensions::PREFAB)),
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

    fs::write(
        folder.join(format!("a_first.{}", extensions::PREFAB)),
        ron::to_string(&first).unwrap(),
    )
    .unwrap();
    fs::write(
        folder.join(format!("z_second.{}", extensions::PREFAB)),
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

    persist_prefab(test_folder.name(), &valid, &AssetRegistry::default(), None).unwrap();
    fs::write(
        folder.join(format!("broken_structure.{}", extensions::PREFAB)),
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
fn persist_prefab_rejects_id_zero() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_zero_id_save");
    let prefab = create_prefab(PrefabId::default(), "Zero".to_string());

    let error =
        persist_prefab(test_folder.name(), &prefab, &AssetRegistry::default(), None).unwrap_err();

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

    persist_prefab(test_folder.name(), &first, &AssetRegistry::default(), None).unwrap();
    persist_prefab(test_folder.name(), &second, &AssetRegistry::default(), None).unwrap();

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

    persist_prefab(test_folder.name(), &first, &AssetRegistry::default(), None).unwrap();
    fs::write(
        prefab_folder_for_game(test_folder.name())
            .join(format!("bullet_copy.{}", extensions::PREFAB)),
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

    persist_prefab(test_folder.name(), &prefab, &AssetRegistry::default(), None).unwrap();

    let manager = load_prefab_manager(test_folder.name(), &mut AssetRegistry::default()).unwrap();

    assert_eq!(manager.prefab_named("Bullet"), Some(&prefab));
    assert_eq!(manager.prefab_named("Missing"), None);
}

#[test]
fn load_prefab_manager_removes_stale_prefab_records_after_successful_reload() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_registry_cleanup_on_load");
    let prefab = create_prefab(PrefabId(5), "Crate".to_string());
    let stale_prefab_id = PrefabId(9);
    let stale_path =
        PathBuf::from(paths::PREFABS_FOLDER).join(format!("stale_prefab.{}", extensions::PREFAB));
    let mut game = Game::with_name(test_folder.name());

    persist_prefab(test_folder.name(), &prefab, &AssetRegistry::default(), None).unwrap();
    game.asset_registry
        .register_asset_relative_path(
            stale_prefab_id,
            format!("stale_prefab.{}", extensions::PREFAB),
        )
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
            "{}.{}",
            sanitise_name(&prefab.name),
            extensions::PREFAB
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
    let prefab_relative_path = PathBuf::from(format!(
        "{}.{}",
        sanitise_name(&prefab.name),
        extensions::PREFAB
    ));
    let expected_path = PathBuf::from(paths::PREFABS_FOLDER).join(&prefab_relative_path);
    let stale_prefab_id = PrefabId(9);
    let mut game = Game::with_name(test_folder.name());

    persist_prefab(test_folder.name(), &prefab, &AssetRegistry::default(), None).unwrap();
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
fn persist_prefab_uses_prefab_name_for_filename() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_name_filename");
    let prefab = create_prefab(PrefabId(5), "Big Crate".to_string());

    persist_prefab(test_folder.name(), &prefab, &AssetRegistry::default(), None).unwrap();

    let expected_path = prefab_folder_for_game(test_folder.name()).join(format!(
        "{}.{}",
        sanitise_name(&prefab.name),
        extensions::PREFAB
    ));
    assert!(expected_path.is_file());
    assert!(!prefab_folder_for_game(test_folder.name())
        .join(format!("5.{}", extensions::PREFAB))
        .exists());
}

#[test]
fn persist_prefab_reuses_existing_file_when_name_changes() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_rename_filename");
    let prefab_id = PrefabId(5);
    let first = create_prefab(prefab_id, "Big Crate".to_string());
    let second = create_prefab(prefab_id, "Huge Barrel".to_string());

    persist_prefab(test_folder.name(), &first, &AssetRegistry::default(), None).unwrap();
    persist_prefab(test_folder.name(), &second, &AssetRegistry::default(), None).unwrap();

    let first_path = prefab_folder_for_game(test_folder.name()).join(format!(
        "{}.{}",
        sanitise_name(&first.name),
        extensions::PREFAB
    ));
    let second_path = prefab_folder_for_game(test_folder.name()).join(format!(
        "{}.{}",
        sanitise_name(&second.name),
        extensions::PREFAB
    ));
    assert!(first_path.is_file());
    assert!(!second_path.exists());
    assert_eq!(
        load_prefab(test_folder.name(), prefab_id, &AssetRegistry::default()).unwrap(),
        second
    );
}

#[test]
fn find_prefab_path_uses_registry_fast_path_before_disk_scan() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_find_registry_fast_path");
    let prefab = create_prefab(PrefabId(5), "Crate".to_string());

    persist_prefab(test_folder.name(), &prefab, &AssetRegistry::default(), None).unwrap();

    let mut asset_registry = AssetRegistry::default();
    load_prefab_manager(test_folder.name(), &mut asset_registry).unwrap();

    let loaded_via_registry = load_prefab(test_folder.name(), prefab.id, &asset_registry).unwrap();
    assert_eq!(loaded_via_registry, prefab);

    let mut empty_registry = AssetRegistry::default();
    let _loaded_manager = load_prefab_manager(test_folder.name(), &mut empty_registry).unwrap();

    let loaded_via_fallback =
        load_prefab(test_folder.name(), prefab.id, &AssetRegistry::default()).unwrap();
    assert_eq!(loaded_via_fallback, prefab);
}

#[test]
fn save_prefab_and_sync_rejects_duplicate_prefab_name() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_save_duplicate_name");
    let first = create_prefab(PrefabId(1), "Crate".to_string());
    let second = create_prefab(PrefabId(2), "Crate".to_string());

    let mut manager = PrefabManager::default();
    manager.prefabs.insert(first.id, first.clone());
    manager.rebuild_name_lookup().unwrap();
    persist_prefab(test_folder.name(), &first, &AssetRegistry::default(), None).unwrap();

    let error = manager
        .save_prefab_and_sync(
            test_folder.name(),
            &mut AssetRegistry::default(),
            &second,
            None,
        )
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::AlreadyExists);
}

#[test]
fn save_prefab_and_sync_allows_resaving_same_prefab_under_its_own_name() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_save_same_name");
    let prefab = create_prefab(PrefabId(1), "Crate".to_string());

    let mut manager = PrefabManager::default();
    manager.prefabs.insert(prefab.id, prefab.clone());
    manager.rebuild_name_lookup().unwrap();
    persist_prefab(test_folder.name(), &prefab, &AssetRegistry::default(), None).unwrap();

    let updated = PrefabAsset {
        nodes: vec![PrefabNode {
            node_id: 1,
            parent_node_id: None,
            components: vec![ComponentSnapshot {
                type_name: comp_type_name::<Name>().to_string(),
                ron: "(\"Crate\")".to_string(),
            }],
        }],
        ..prefab.clone()
    };

    let saved = manager
        .save_prefab_and_sync(
            test_folder.name(),
            &mut AssetRegistry::default(),
            &updated,
            None,
        )
        .unwrap();
    assert_eq!(saved.id, prefab.id);
}

#[test]
fn save_prefab_and_sync_uses_explicit_initial_target_path() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_explicit_initial_target");
    let prefab = create_prefab(PrefabId(7), "Boss Ogre".to_string());
    let nested_relative = PathBuf::from("characters")
        .join("bosses")
        .join(format!("boss_ogre.{}", extensions::PREFAB));
    let explicit_path = prefab_folder_for_game(test_folder.name()).join(&nested_relative);
    let expected_registry_path = PathBuf::from(paths::PREFABS_FOLDER).join(&nested_relative);
    let mut registry = AssetRegistry::default();
    let mut manager = PrefabManager::default();

    let saved = manager
        .save_prefab_and_sync(
            test_folder.name(),
            &mut registry,
            &prefab,
            Some(explicit_path.as_path()),
        )
        .unwrap();

    assert_eq!(saved.name, prefab.name);
    assert_eq!(
        registry.relative_path(saved.id),
        Some(nested_relative.clone())
    );
    assert_eq!(
        registry.key_for_path(&expected_registry_path),
        Some(AssetKey::Prefab(saved.id))
    );
    assert!(explicit_path.is_file());
}

#[test]
fn save_prefab_and_sync_reuses_existing_nested_path_after_rename() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_nested_resave_path_stable");
    let prefab_id = PrefabId(11);
    let original = create_prefab(prefab_id, "Boss Ogre".to_string());
    let renamed = create_prefab(prefab_id, "Boss Ogre Elite".to_string());
    let nested_relative = PathBuf::from("characters")
        .join("bosses")
        .join(format!("boss_ogre.{}", extensions::PREFAB));
    let explicit_path = prefab_folder_for_game(test_folder.name()).join(&nested_relative);
    let mut registry = AssetRegistry::default();
    let mut manager = PrefabManager::default();

    manager
        .save_prefab_and_sync(
            test_folder.name(),
            &mut registry,
            &original,
            Some(explicit_path.as_path()),
        )
        .unwrap();
    manager
        .save_prefab_and_sync(test_folder.name(), &mut registry, &renamed, None)
        .unwrap();

    assert!(explicit_path.is_file());
    assert_eq!(
        load_prefab(test_folder.name(), prefab_id, &registry).unwrap(),
        renamed
    );
    assert_eq!(registry.relative_path(prefab_id), Some(nested_relative));
}

#[test]
fn save_prefab_and_sync_rejects_explicit_target_outside_prefab_root() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_explicit_target_outside_root");
    let prefab = create_prefab(PrefabId(17), "Outside Root".to_string());
    let prefab_root = prefab_folder_for_game(test_folder.name());
    let outside_path = prefab_root
        .parent()
        .expect("prefab root should have a parent")
        .join(format!("outside.{}", extensions::PREFAB));
    let mut registry = AssetRegistry::default();
    let mut manager = PrefabManager::default();

    let error = manager
        .save_prefab_and_sync(
            test_folder.name(),
            &mut registry,
            &prefab,
            Some(outside_path.as_path()),
        )
        .unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert!(!prefab_root.exists());
    assert!(!outside_path.exists());
    assert_eq!(registry.relative_path(prefab.id), None);
    assert_eq!(manager.prefabs.get(&prefab.id), None);
}

#[test]
fn save_prefab_and_sync_rejects_explicit_target_with_wrong_extension() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_explicit_target_wrong_extension");
    let prefab = create_prefab(PrefabId(19), "Wrong Extension".to_string());
    let wrong_extension_path =
        prefab_folder_for_game(test_folder.name()).join("wrong_extension.txt");
    let mut registry = AssetRegistry::default();
    let mut manager = PrefabManager::default();

    let error = manager
        .save_prefab_and_sync(
            test_folder.name(),
            &mut registry,
            &prefab,
            Some(wrong_extension_path.as_path()),
        )
        .unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert!(!wrong_extension_path.exists());
    assert_eq!(registry.relative_path(prefab.id), None);
    assert_eq!(manager.prefabs.get(&prefab.id), None);
}

#[test]
fn save_prefab_and_sync_rejects_explicit_target_owned_by_another_prefab() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_explicit_target_collision");
    let original = create_prefab(PrefabId(23), "Original".to_string());
    let conflicting = create_prefab(PrefabId(24), "Conflicting".to_string());
    let nested_relative = PathBuf::from("characters")
        .join("bosses")
        .join(format!("shared_path.{}", extensions::PREFAB));
    let explicit_path = prefab_folder_for_game(test_folder.name()).join(&nested_relative);
    let registry_path = PathBuf::from(paths::PREFABS_FOLDER).join(&nested_relative);
    let mut registry = AssetRegistry::default();
    let mut manager = PrefabManager::default();

    manager
        .save_prefab_and_sync(
            test_folder.name(),
            &mut registry,
            &original,
            Some(explicit_path.as_path()),
        )
        .unwrap();

    let error = manager
        .save_prefab_and_sync(
            test_folder.name(),
            &mut registry,
            &conflicting,
            Some(explicit_path.as_path()),
        )
        .unwrap_err();

    assert_eq!(error.kind(), ErrorKind::AlreadyExists);
    assert_eq!(
        load_prefab(test_folder.name(), original.id, &registry).unwrap(),
        original
    );
    assert_eq!(
        registry.key_for_path(&registry_path),
        Some(AssetKey::Prefab(original.id))
    );
    assert_eq!(registry.relative_path(conflicting.id), None);
    assert_eq!(manager.prefabs.get(&conflicting.id), None);
}

#[test]
fn save_prefab_and_sync_rejects_explicit_target_file_owned_by_another_prefab_when_registry_is_stale(
) {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_explicit_target_stale_registry_collision");
    let original = create_prefab(PrefabId(31), "Original".to_string());
    let conflicting = create_prefab(PrefabId(32), "Conflicting".to_string());
    let nested_relative = PathBuf::from("characters")
        .join("bosses")
        .join(format!("stale_registry_path.{}", extensions::PREFAB));
    let explicit_path = prefab_folder_for_game(test_folder.name()).join(&nested_relative);
    let mut manager = PrefabManager::default();

    manager
        .save_prefab_and_sync(
            test_folder.name(),
            &mut AssetRegistry::default(),
            &original,
            Some(explicit_path.as_path()),
        )
        .unwrap();

    let original_on_disk =
        load_prefab(test_folder.name(), original.id, &AssetRegistry::default()).unwrap();
    let error = manager
        .save_prefab_and_sync(
            test_folder.name(),
            &mut AssetRegistry::default(),
            &conflicting,
            Some(explicit_path.as_path()),
        )
        .unwrap_err();

    assert_eq!(error.kind(), ErrorKind::AlreadyExists);
    assert_eq!(
        load_prefab(test_folder.name(), original.id, &AssetRegistry::default()).unwrap(),
        original_on_disk
    );
    assert!(load_prefab(
        test_folder.name(),
        conflicting.id,
        &AssetRegistry::default()
    )
    .is_err());
    assert_eq!(manager.prefabs.get(&conflicting.id), None);
}

#[test]
fn load_prefab_manager_discovers_nested_prefab_files() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_nested_reload");
    let prefab = create_prefab(PrefabId(5), "Nested Boss".to_string());
    let nested_relative = PathBuf::from("characters")
        .join("bosses")
        .join(format!("nested_boss.{}", extensions::PREFAB));
    let nested_path = prefab_folder_for_game(test_folder.name()).join(&nested_relative);
    let expected_registry_path = PathBuf::from(paths::PREFABS_FOLDER).join(&nested_relative);
    let mut registry = AssetRegistry::default();

    fs::create_dir_all(
        nested_path
            .parent()
            .expect("nested prefab path should have a parent folder"),
    )
    .unwrap();
    fs::write(&nested_path, ron::to_string(&prefab).unwrap()).unwrap();

    let manager = load_prefab_manager(test_folder.name(), &mut registry).unwrap();

    assert_eq!(manager.prefabs.get(&prefab.id), Some(&prefab));
    assert_eq!(
        registry.relative_path(prefab.id),
        Some(nested_relative.clone())
    );
    assert_eq!(
        registry.key_for_path(&expected_registry_path),
        Some(AssetKey::Prefab(prefab.id))
    );
}
