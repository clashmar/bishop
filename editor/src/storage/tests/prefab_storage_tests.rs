use super::*;

#[test]
fn reload_prefab_manager_reconciles_prefab_registry_records() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_registry_reload");
    set_game_name(test_game.name());
    create_game_folders(test_game.name());

    let prefab = create_prefab(PrefabId(9), "Crate".to_string());
    let prefab_file_name = format!("disk_prefab.{}", extensions::PREFAB);
    let prefab_path = prefabs_folder().join(&prefab_file_name);
    let expected_path = PathBuf::from(paths::PREFABS_FOLDER).join(&prefab_file_name);
    let stale_prefab_id = PrefabId(21);
    let stale_path =
        PathBuf::from(paths::PREFABS_FOLDER).join(format!("stale_prefab.{}", extensions::PREFAB));
    let mut game = create_new_game(test_game.name().to_string());

    fs::write(&prefab_path, ron::to_string(&prefab).unwrap()).unwrap();
    game.asset_registry
        .register_asset_relative_path(
            stale_prefab_id,
            format!("stale_prefab.{}", extensions::PREFAB),
        )
        .unwrap();

    game.reload_prefab_manager();

    assert_eq!(game.prefab_manager.prefabs.get(&prefab.id), Some(&prefab));
    assert_eq!(
        game.asset_registry.key_for_path(&expected_path),
        Some(AssetKey::Prefab(prefab.id))
    );
    assert_eq!(
        game.asset_registry.relative_path(prefab.id),
        Some(PathBuf::from(&prefab_file_name))
    );
    assert_eq!(
        game.asset_registry.record(AssetKey::Prefab(stale_prefab_id)),
        None
    );
    assert_eq!(game.asset_registry.key_for_path(&stale_path), None);
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

    persist_prefab(test_game.name(), &prefab, &AssetRegistry::default(), None).unwrap();

    let expected_path = prefabs_folder().join(format!(
        "{}.{}",
        sanitise_name(&prefab.name),
        extensions::PREFAB
    ));
    assert!(expected_path.is_file());

    let loaded_manager =
        load_prefab_manager(test_game.name(), &mut AssetRegistry::default()).unwrap();
    let loaded = loaded_manager.prefabs.get(&prefab.id).cloned().unwrap();
    let mut listed: Vec<_> = loaded_manager.prefabs.into_values().collect();
    listed.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.id.cmp(&right.id))
    });

    assert_eq!(loaded, prefab);
    assert_eq!(listed, vec![prefab.clone()]);
    assert_eq!(
        load_prefab_manager(test_game.name(), &mut AssetRegistry::default())
            .unwrap()
            .prefabs
            .get(&prefab.id),
        Some(&prefab)
    );

    assert!(delete_prefab(test_game.name(), prefab.id, &AssetRegistry::default()).unwrap());
    let after_manager =
        load_prefab_manager(test_game.name(), &mut AssetRegistry::default()).unwrap();
    assert!(after_manager.prefabs.is_empty());
}

#[test]
fn save_game_writes_prefabs_lua() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_lua_save");
    set_game_name(test_game.name());
    create_game_folders(test_game.name());

    let mut game = Game::default();
    game.name = test_game.name().to_string();
    game.prefab_manager.prefabs.insert(
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

    let prefabs_path = scripts_folder()
        .join(lua_dirs::ENGINE)
        .join(engine_core::scripting::lua_project::engine_relative_path(
            lua_files::PREFABS,
        ));
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

    let mut game = Game::default();
    game.name = test_game.name().to_string();
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
    game.prefab_manager.prefabs.insert(prefab_a.id, prefab_a);
    game.prefab_manager.prefabs.insert(prefab_b.id, prefab_b);

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

    let prefabs_path = scripts_folder()
        .join(lua_dirs::ENGINE)
        .join(engine_core::scripting::lua_project::engine_relative_path(
            lua_files::PREFABS,
        ));
    let contents = std::fs::read_to_string(prefabs_path).unwrap();

    assert!(contents.contains("BossAttack = \"Boss Attack\""));
    assert!(contents.contains("BossAttack_2 = \"Boss-Attack\""));
    assert!(contents.contains("Crate = \"Crate\""));
}

#[test]
fn generated_lua_typings_hide_prefab_internal_components() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let engine_dir = root.join(lua_dirs::SCRIPTS).join(lua_dirs::ENGINE);
    let components = std::fs::read_to_string(
        engine_dir.join(engine_core::scripting::lua_project::engine_relative_path(
            lua_files::COMPONENTS,
        )),
    )
    .unwrap();
    let entity = std::fs::read_to_string(
        engine_dir.join(engine_core::scripting::lua_project::engine_relative_path(
            lua_files::ENTITY,
        )),
    )
    .unwrap();
    let public_type = comp_type_name::<Transform>();
    let hidden = [
        comp_type_name::<PrefabInstanceNode>(),
        comp_type_name::<PrefabInstanceRoot>(),
        comp_type_name::<PrefabOverrides>(),
    ];

    assert!(components.contains(public_type));
    assert!(entity.contains(public_type));
    for type_name in hidden {
        assert!(!components.contains(type_name));
        assert!(!entity.contains(type_name));
    }
}

#[test]
fn save_game_writes_event_tags_lua_from_global_usage() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("event_tags_lua_save");
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    let world = game
        .current_world_mut()
        .expect("new game should have a current world");
    world.rooms_mut()[0].tags = vec![
        EventTag::Custom("Zebra".to_string()),
        EventTag::Autosave,
        EventTag::Custom("Alpha".to_string()),
    ];

    save_game(&game).unwrap();

    let event_tags_path = scripts_folder()
        .join(lua_dirs::ENGINE)
        .join(engine_core::scripting::lua_project::engine_relative_path(
            lua_files::EVENT_TAGS,
        ));
    let contents = std::fs::read_to_string(event_tags_path).unwrap();
    let alpha_pos = contents.find("Alpha = \"Alpha\"").unwrap();
    let zebra_pos = contents.find("Zebra = \"Zebra\"").unwrap();

    assert!(contents.contains("Autosave = \"Autosave\""));
    assert!(alpha_pos < zebra_pos);
}
