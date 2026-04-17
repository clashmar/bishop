use super::*;
use crate::storage::path_utils::sanitise_name;
use crate::storage::test_utils::{game_fs_test_lock, TestGameFolder};

#[test]
fn load_prefab_library_skips_invalid_prefab_files() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_folder = TestGameFolder::new("prefab_partial_load");
    let valid = create_prefab(PrefabId(1), "Valid".to_string());

    save_prefab(test_folder.name(), &valid).unwrap();
    fs::write(
        prefab_folder_for_game(test_folder.name()).join("broken.ron"),
        "not valid ron",
    )
    .unwrap();

    let library = load_prefab_library(test_folder.name()).unwrap();

    assert_eq!(library.prefabs.len(), 1);
    assert_eq!(library.prefabs.get(&valid.id), Some(&valid));
}

#[test]
fn load_prefab_library_rejects_duplicate_prefab_ids() {
    let _lock = game_fs_test_lock().lock().unwrap();
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

    let error = load_prefab_library(test_folder.name()).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidData);
}

#[test]
fn load_prefab_library_skips_structurally_invalid_prefabs() {
    let _lock = game_fs_test_lock().lock().unwrap();
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

    let library = load_prefab_library(test_folder.name()).unwrap();

    assert_eq!(library.prefabs.len(), 1);
    assert_eq!(library.prefabs.get(&valid.id), Some(&valid));
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
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_folder = TestGameFolder::new("prefab_zero_id_save");
    let prefab = create_prefab(PrefabId::default(), "Zero".to_string());

    let error = save_prefab(test_folder.name(), &prefab).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidData);
}

#[test]
fn load_prefab_library_restores_next_prefab_id_from_loaded_assets() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_folder = TestGameFolder::new("prefab_next_id");
    let first = create_prefab(PrefabId(3), "First".to_string());
    let second = create_prefab(PrefabId(9), "Second".to_string());

    save_prefab(test_folder.name(), &first).unwrap();
    save_prefab(test_folder.name(), &second).unwrap();

    let mut library = load_prefab_library(test_folder.name()).unwrap();

    assert_eq!(library.next_prefab_id, 10);
    assert_eq!(library.allocate_prefab_id(), PrefabId(10));
    assert_eq!(library.allocate_prefab_id(), PrefabId(11));
}

#[test]
fn load_prefab_library_rejects_duplicate_prefab_names() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_folder = TestGameFolder::new("prefab_duplicate_names");
    let first = create_prefab(PrefabId(3), "Bullet".to_string());
    let second = create_prefab(PrefabId(9), "Bullet".to_string());

    save_prefab(test_folder.name(), &first).unwrap();
    fs::write(
        prefab_folder_for_game(test_folder.name()).join("bullet_copy.ron"),
        ron::to_string(&second).unwrap(),
    )
    .unwrap();

    let error = load_prefab_library(test_folder.name()).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidData);
}

#[test]
fn load_prefab_library_supports_lookup_by_name() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_folder = TestGameFolder::new("prefab_name_lookup");
    let prefab = create_prefab(PrefabId(5), "Bullet".to_string());

    save_prefab(test_folder.name(), &prefab).unwrap();

    let library = load_prefab_library(test_folder.name()).unwrap();

    assert_eq!(library.prefab_named("Bullet"), Some(&prefab));
    assert_eq!(library.prefab_named("Missing"), None);
}

#[test]
fn save_prefab_uses_prefab_name_for_filename() {
    let _lock = game_fs_test_lock().lock().unwrap();
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
    let _lock = game_fs_test_lock().lock().unwrap();
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

#[test]
fn load_and_delete_prefab_support_legacy_id_filename() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_folder = TestGameFolder::new("prefab_legacy_filename");
    let prefab = create_prefab(PrefabId(12), "Legacy".to_string());
    let folder = prefab_folder_for_game(test_folder.name());
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("12.ron"), ron::to_string(&prefab).unwrap()).unwrap();

    assert_eq!(load_prefab(test_folder.name(), prefab.id).unwrap(), prefab);
    assert!(delete_prefab(test_folder.name(), prefab.id).unwrap());
    assert!(!folder.join("12.ron").exists());
}
