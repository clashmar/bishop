use super::*;
use std::path::Path;

#[test]
fn saving_prefab_registers_prefab_record_in_asset_registry() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_save_registers_asset_record");
    let (editor, _, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let staged_state = editor.active_prefab_staged_state();
        let prefab = match staged_state {
            Some(StagedPrefabState::PrefabAsset(prefab)) => prefab,
            _ => unreachable!(),
        };
        let expected_path = saved_prefab_path(&prefab);
        let expected_relative_path = Path::new(
            expected_path
                .file_name()
                .expect("saved prefab path should have file name"),
        );

        assert!(editor.commit_prefab_asset_save(prefab));
        assert_eq!(
            editor
                .game
                .asset_registry
                .relative_path(prefab_id)
                .as_deref(),
            Some(expected_relative_path)
        );
        assert_eq!(
            editor
                .game
                .asset_registry
                .key_for_path(PathBuf::from(paths::PREFABS_FOLDER).join(expected_relative_path)),
            Some(AssetKey::Prefab(prefab_id))
        );
    });
}

#[test]
fn saving_prefab_rename_keeps_prefab_record_path_and_file() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_save_rename_keeps_existing_path");
    let (editor, _, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let original_prefab = match editor.active_prefab_staged_state() {
            Some(StagedPrefabState::PrefabAsset(prefab)) => prefab,
            _ => unreachable!(),
        };
        assert!(editor.commit_prefab_asset_save(original_prefab.clone()));

        let original_relative_path = editor.game.asset_registry.relative_path(prefab_id).unwrap();
        let original_full_path = prefabs_folder().join(&original_relative_path);
        let renamed_prefab = PrefabAsset {
            name: "Barrel".to_string(),
            ..original_prefab
        };

        assert!(editor.commit_prefab_asset_save(renamed_prefab));
        assert_eq!(
            editor.game.asset_registry.relative_path(prefab_id),
            Some(original_relative_path)
        );
        assert!(original_full_path.is_file());
    });
}

#[test]
fn saving_empty_prefab_delete_supports_undo_and_redo_preview_sync() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_empty_save_undo_redo");
    let (editor, _, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let root = editor.prefab_editor.as_ref().unwrap().root_entity.unwrap();
        push_command(Box::new(DeleteEntityCmd::new(
            root,
            EditorMode::Prefab(prefab_id),
        )));
    });
    apply_pending_commands();

    with_editor(|editor| {
        editor.reconcile_active_prefab_room_preview();
        editor.confirm_empty_prefab_save_delete();
        assert!(!editor.game.prefab_manager.prefabs.contains_key(&prefab_id));
        assert!(linked_root_entities(&editor.game.ecs, prefab_id).is_empty());
    });

    request_undo();
    apply_pending_commands();

    with_editor(|editor| {
        editor.reconcile_active_prefab_room_preview();
        assert!(editor.prefab_editor.as_ref().unwrap().root_entity.is_some());
        assert_eq!(linked_root_entities(&editor.game.ecs, prefab_id).len(), 1);
    });

    request_redo();
    apply_pending_commands();

    with_editor(|editor| {
        editor.reconcile_active_prefab_room_preview();
        assert!(editor.prefab_editor.as_ref().unwrap().root_entity.is_none());
        assert!(linked_root_entities(&editor.game.ecs, prefab_id).is_empty());
    });
}

#[test]
fn saving_prefab_canonicalizes_root_component_order_on_disk() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_save_component_order_canonical");
    set_game_name(test_game.name());

    let mut prefab = create_prefab(PrefabId(1), "Crate".to_string());
    let root_node = prefab
        .nodes
        .iter_mut()
        .find(|node| node.node_id == prefab.root_node_id)
        .expect("root node should exist");
    root_node.components = vec![
        ComponentSnapshot {
            type_name: comp_type_name::<Transform>().to_string(),
            ron: ron::to_string(&Transform::default()).expect("transform should serialize"),
        },
        ComponentSnapshot {
            type_name: comp_type_name::<Name>().to_string(),
            ron: ron::to_string(&Name("Root".to_string())).expect("name should serialize"),
        },
    ];
    root_node
        .components
        .sort_by(|left, right| right.type_name.cmp(&left.type_name));

    persist_prefab(test_game.name(), &prefab, &AssetRegistry::default(), None)
        .expect("prefab should save");

    let saved_prefab = load_prefab_manager(test_game.name(), &mut AssetRegistry::default())
        .expect("prefab should load")
        .prefabs
        .get(&prefab.id)
        .cloned()
        .expect("saved prefab should exist in manager");
    let saved_root = saved_prefab
        .nodes
        .iter()
        .find(|node| node.node_id == saved_prefab.root_node_id)
        .expect("saved root node should exist");
    let component_names = saved_root
        .components
        .iter()
        .map(|component| component.type_name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        component_names,
        vec![comp_type_name::<Name>(), comp_type_name::<Transform>(),]
    );
}

#[test]
fn saving_prefab_canonicalizes_node_order_on_disk() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_save_node_order_canonical");
    set_game_name(test_game.name());

    let mut prefab = create_prefab(PrefabId(1), "Crate".to_string());
    prefab.nodes.push(PrefabNode {
        node_id: 2,
        parent_node_id: Some(prefab.root_node_id),
        components: vec![ComponentSnapshot {
            type_name: comp_type_name::<Name>().to_string(),
            ron: ron::to_string(&Name("Child".to_string())).expect("name should serialize"),
        }],
    });
    prefab.nodes.push(PrefabNode {
        node_id: 3,
        parent_node_id: Some(prefab.root_node_id),
        components: vec![ComponentSnapshot {
            type_name: comp_type_name::<Name>().to_string(),
            ron: ron::to_string(&Name("Sibling".to_string())).expect("name should serialize"),
        }],
    });
    prefab.next_node_id = 4;
    prefab.nodes.swap(0, 2);

    persist_prefab(test_game.name(), &prefab, &AssetRegistry::default(), None)
        .expect("prefab should save");

    let saved_prefab = load_prefab_manager(test_game.name(), &mut AssetRegistry::default())
        .expect("prefab should load")
        .prefabs
        .get(&prefab.id)
        .cloned()
        .expect("saved prefab should exist in manager");
    let node_ids = saved_prefab
        .nodes
        .iter()
        .map(|node| node.node_id)
        .collect::<Vec<_>>();

    assert_eq!(node_ids, vec![1, 2, 3]);
}

#[test]
fn saving_prefab_records_canonical_committed_state() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_save_committed_state_canonical");
    let (editor, _, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let mut staged_prefab = match editor.active_prefab_staged_state() {
            Some(StagedPrefabState::PrefabAsset(prefab)) => prefab,
            _ => unreachable!(),
        };
        let root_node = staged_prefab
            .nodes
            .iter_mut()
            .find(|node| node.node_id == staged_prefab.root_node_id)
            .expect("root node should exist");
        assert!(root_node.components.len() >= 2);
        root_node
            .components
            .sort_by(|left, right| right.type_name.cmp(&left.type_name));

        assert!(editor.commit_prefab_asset_save(staged_prefab));
        assert!(editor.active_prefab_is_clean());

        let committed_prefab = match editor
            .prefab_editor
            .as_ref()
            .expect("prefab editor should exist")
            .last_committed_prefab
            .clone()
        {
            StagedPrefabState::PrefabAsset(prefab) => prefab,
            StagedPrefabState::Empty => unreachable!(),
        };
        let committed_root = committed_prefab
            .nodes
            .iter()
            .find(|node| node.node_id == committed_prefab.root_node_id)
            .expect("committed root node should exist");
        let component_names = committed_root
            .components
            .iter()
            .map(|component| component.type_name.as_str())
            .collect::<Vec<_>>();
        let node_ids = committed_prefab
            .nodes
            .iter()
            .map(|node| node.node_id)
            .collect::<Vec<_>>();

        assert_eq!(
            component_names,
            vec![comp_type_name::<Name>(), comp_type_name::<Transform>(),]
        );
        assert_eq!(node_ids, vec![1]);
        assert_eq!(committed_prefab.id, prefab_id);
    });
}

#[test]
fn reopening_saved_prefab_uses_canonical_in_memory_prefab_order() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_reopen_uses_canonical_save_order");
    let (editor, room_id, prefab_id, _) = make_prefab_session_editor(&test_game);
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let mut staged_prefab = match editor.active_prefab_staged_state() {
            Some(StagedPrefabState::PrefabAsset(prefab)) => prefab,
            _ => unreachable!(),
        };
        let root_node = staged_prefab
            .nodes
            .iter_mut()
            .find(|node| node.node_id == staged_prefab.root_node_id)
            .expect("root node should exist");
        root_node
            .components
            .sort_by(|left, right| right.type_name.cmp(&left.type_name));

        staged_prefab.nodes.push(PrefabNode {
            node_id: staged_prefab.next_node_id,
            parent_node_id: Some(staged_prefab.root_node_id),
            components: vec![ComponentSnapshot {
                type_name: comp_type_name::<Name>().to_string(),
                ron: ron::to_string(&Name("Child".to_string())).expect("name should serialize"),
            }],
        });
        staged_prefab.next_node_id += 1;
        staged_prefab.nodes.swap(0, 1);

        assert!(editor.commit_prefab_asset_save(staged_prefab));
        editor.close_active_prefab_editor();
        assert_eq!(editor.mode, EditorMode::Room(room_id));

        editor.open_prefab_editor_for_id(prefab_id);
        let reopened_prefab = match editor
            .prefab_editor
            .as_ref()
            .expect("prefab editor should exist")
            .last_committed_prefab
            .clone()
        {
            StagedPrefabState::PrefabAsset(prefab) => prefab,
            StagedPrefabState::Empty => unreachable!(),
        };
        let node_ids = reopened_prefab
            .nodes
            .iter()
            .map(|node| node.node_id)
            .collect::<Vec<_>>();
        let reopened_root = reopened_prefab
            .nodes
            .iter()
            .find(|node| node.node_id == reopened_prefab.root_node_id)
            .expect("reopened root node should exist");
        let component_names = reopened_root
            .components
            .iter()
            .map(|component| component.type_name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(node_ids, vec![1, 2]);
        assert_eq!(
            component_names,
            vec![comp_type_name::<Name>(), comp_type_name::<Transform>(),]
        );
    });
}

#[test]
fn saving_new_prefab_session_marks_prefab_clean_for_exit() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_new_session_clean_after_save");
    set_game_name(test_game.name());
    let prefab = create_prefab(PrefabId(1), "Crate".to_string());
    let (mut prefab_editor, mut prefab_stage) = PrefabEditor::open_existing(
        test_game.name(),
        prefab.clone(),
        PrefabRoomSyncState {
            staged_prefab: StagedPrefabState::PrefabAsset(prefab),
            linked_instance_snapshots: Vec::new(),
        },
    );

    let entity = prefab_editor.create_prefab_entity(&mut prefab_stage.ecs, None);
    prefab_editor.set_selected_entity(Some(entity));

    let editor = Editor {
        game: create_new_game(test_game.name().to_string()),
        mode: EditorMode::Prefab(PrefabId(1)),
        prefab_editor: Some(prefab_editor),
        prefab_stage: Some(prefab_stage),
        ..Default::default()
    };
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let staged_state = editor.active_prefab_staged_state();
        assert!(matches!(
            staged_state,
            Some(StagedPrefabState::PrefabAsset(_))
        ));
        editor.commit_prefab_asset_save(match staged_state {
            Some(StagedPrefabState::PrefabAsset(prefab)) => prefab,
            _ => unreachable!(),
        });
        assert!(editor.active_prefab_is_clean());
    });
}

#[test]
fn saving_prefab_syncs_stage_sprite_registry_into_game_and_disk() {
    const BUILDING_SPRITE_PATH: &str = "sprites/building.png";

    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_save_sprite_registry_sync");
    set_game_name(test_game.name());
    let prefab = create_prefab(PrefabId(1), "Building".to_string());
    let base_game = create_new_game(test_game.name().to_string());
    let (mut prefab_editor, mut prefab_stage) = PrefabEditor::open_existing_from_game(
        &base_game,
        prefab.clone(),
        PrefabRoomSyncState {
            staged_prefab: StagedPrefabState::PrefabAsset(prefab),
            linked_instance_snapshots: Vec::new(),
        },
    );

    let entity = prefab_editor.create_prefab_entity(&mut prefab_stage.ecs, None);
    prefab_editor.set_selected_entity(Some(entity));
    prefab_stage.ecs.add_component_to_entity(
        entity,
        Sprite {
            sprite: SpriteId(9),
        },
    );
    prefab_stage
        .asset_registry
        .register_asset_relative_path(SpriteId(9), BUILDING_SPRITE_PATH)
        .expect("sprite path should register");
    SpriteManager::init_editor_metadata(
        &prefab_stage.asset_registry,
        &mut prefab_stage.sprite_manager,
    );

    let editor = Editor {
        game: create_new_game(test_game.name().to_string()),
        mode: EditorMode::Prefab(PrefabId(1)),
        prefab_editor: Some(prefab_editor),
        prefab_stage: Some(prefab_stage),
        ..Default::default()
    };
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let staged_state = editor.active_prefab_staged_state();
        editor.commit_prefab_asset_save(match staged_state {
            Some(StagedPrefabState::PrefabAsset(prefab)) => prefab,
            _ => unreachable!(),
        });

        assert_eq!(
            editor
                .game
                .asset_registry
                .relative_path(SpriteId(9))
                .as_deref(),
            Some(Path::new(BUILDING_SPRITE_PATH))
        );
        assert_eq!(
            editor.game.sprite_manager.path_for_id(SpriteId(9)),
            Some(Path::new(BUILDING_SPRITE_PATH))
        );
    });

    let mut saved_game = load_game_by_name(test_game.name()).expect("saved game should load");
    SpriteManager::init_editor_metadata(&saved_game.asset_registry, &mut saved_game.sprite_manager);
    assert_eq!(
        saved_game
            .asset_registry
            .relative_path(SpriteId(9))
            .as_deref(),
        Some(Path::new(BUILDING_SPRITE_PATH))
    );
    assert_eq!(
        saved_game.sprite_manager.path_for_id(SpriteId(9)),
        Some(Path::new(BUILDING_SPRITE_PATH))
    );
}

#[test]
fn saving_prefab_activates_it_in_room_palette_and_persists_state() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_save_updates_palette");
    set_game_name(test_game.name());

    let prefab = create_prefab(PrefabId(1), "Crate".to_string());
    let other_prefab = create_prefab(PrefabId(2), "Torch".to_string());
    let mut base_game = create_new_game(test_game.name().to_string());
    base_game
        .prefab_manager
        .prefabs
        .insert(prefab.id, prefab.clone());
    let (mut prefab_editor, mut prefab_stage) = PrefabEditor::open_existing_from_game(
        &base_game,
        prefab.clone(),
        PrefabRoomSyncState {
            staged_prefab: StagedPrefabState::PrefabAsset(prefab.clone()),
            linked_instance_snapshots: Vec::new(),
        },
    );

    let entity = prefab_editor.create_prefab_entity(&mut prefab_stage.ecs, None);
    prefab_editor.set_selected_entity(Some(entity));

    let mut editor = Editor {
        game: create_new_game(test_game.name().to_string()),
        mode: EditorMode::Prefab(prefab.id),
        prefab_editor: Some(prefab_editor),
        prefab_stage: Some(prefab_stage),
        ..Default::default()
    };
    editor
        .game
        .prefab_manager
        .prefabs
        .insert(other_prefab.id, other_prefab);
    editor.room_editor.mode = RoomEditorMode::Tilemap;
    editor.room_editor.mode_selector.current = RoomEditorMode::Tilemap;
    editor.room_editor.scene_sub_mode = RoomSceneSubMode::Scene;
    editor.room_editor.view_preview = true;
    editor.room_editor.active_prefab_id = Some(PrefabId(2));
    editor.room_editor.recent_prefab_ids = vec![PrefabId(2), prefab.id];

    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let staged_state = editor.active_prefab_staged_state();
        editor.commit_prefab_asset_save(match staged_state {
            Some(StagedPrefabState::PrefabAsset(prefab)) => prefab,
            _ => unreachable!(),
        });

        assert_eq!(editor.room_editor.active_prefab_id, Some(PrefabId(1)));
        assert_eq!(
            editor.room_editor.recent_prefab_ids,
            vec![PrefabId(1), PrefabId(2)]
        );
        assert_eq!(editor.room_editor.mode, RoomEditorMode::Tilemap);
        assert_eq!(
            editor.room_editor.mode_selector.current,
            RoomEditorMode::Tilemap
        );
        assert_eq!(editor.room_editor.scene_sub_mode, RoomSceneSubMode::Scene);
        assert!(editor.room_editor.view_preview);
    });

    let saved_state =
        load_prefab_palette_state(test_game.name()).expect("prefab palette state should save");
    assert_eq!(saved_state.active_prefab_id, Some(PrefabId(1)));
    assert_eq!(
        saved_state.recent_prefab_ids,
        vec![PrefabId(1), PrefabId(2)]
    );
}

#[test]
fn saving_prefab_syncs_stage_script_registry_into_game_and_disk() {
    const BUILDING_SCRIPT_PATH: &str = "building.lua";

    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_save_script_registry_sync");
    set_game_name(test_game.name());
    let prefab = create_prefab(PrefabId(1), "Building".to_string());
    let base_game = create_new_game(test_game.name().to_string());
    let (mut prefab_editor, mut prefab_stage) = PrefabEditor::open_existing_from_game(
        &base_game,
        prefab.clone(),
        PrefabRoomSyncState {
            staged_prefab: StagedPrefabState::PrefabAsset(prefab),
            linked_instance_snapshots: Vec::new(),
        },
    );

    let entity = prefab_editor.create_prefab_entity(&mut prefab_stage.ecs, None);
    prefab_editor.set_selected_entity(Some(entity));
    prefab_stage.ecs.add_component_to_entity(
        entity,
        Script {
            script_id: ScriptId(9),
            ..Default::default()
        },
    );
    prefab_stage
        .asset_registry
        .register_asset_relative_path(ScriptId(9), BUILDING_SCRIPT_PATH)
        .expect("script path should register");
    ScriptManager::init_editor_metadata(
        &prefab_stage.asset_registry,
        &mut prefab_stage.script_manager,
    );

    let editor = Editor {
        game: create_new_game(test_game.name().to_string()),
        mode: EditorMode::Prefab(PrefabId(1)),
        prefab_editor: Some(prefab_editor),
        prefab_stage: Some(prefab_stage),
        ..Default::default()
    };
    let _guard = EditorServicesGuard::install(editor);

    with_editor(|editor| {
        let staged_state = editor.active_prefab_staged_state();
        editor.commit_prefab_asset_save(match staged_state {
            Some(StagedPrefabState::PrefabAsset(prefab)) => prefab,
            _ => unreachable!(),
        });

        assert_eq!(
            editor
                .game
                .asset_registry
                .relative_path(ScriptId(9))
                .as_deref(),
            Some(Path::new(BUILDING_SCRIPT_PATH))
        );
        assert_eq!(
            editor.game.script_manager.path_for_id(ScriptId(9)),
            Some(Path::new(BUILDING_SCRIPT_PATH))
        );
    });

    let mut saved_game = load_game_by_name(test_game.name()).expect("saved game should load");
    ScriptManager::init_editor_metadata(&saved_game.asset_registry, &mut saved_game.script_manager);
    assert_eq!(
        saved_game
            .asset_registry
            .relative_path(ScriptId(9))
            .as_deref(),
        Some(Path::new(BUILDING_SCRIPT_PATH))
    );
    assert_eq!(
        saved_game.script_manager.path_for_id(ScriptId(9)),
        Some(Path::new(BUILDING_SCRIPT_PATH))
    );
}

#[test]
fn create_blank_prefab_uses_selected_save_target() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("prefab_blank_initial_save_target");
    let mut editor = blank_prefab_session_editor(&test_game);
    let picked_path = prefabs_folder()
        .join("props")
        .join(format!("Barrel.{}", extensions::PREFAB));

    let prompt = editor.request_blank_prefab_transition("Barrel".to_string(), picked_path.clone());
    assert_eq!(prompt, PrefabTransitionPrompt::None);

    assert_eq!(editor.mode, EditorMode::Prefab(PrefabId(1)));
    assert_eq!(
        editor
            .game
            .asset_registry
            .relative_path(PrefabId(1))
            .as_deref(),
        Some(
            Path::new("props")
                .join(format!("Barrel.{}", extensions::PREFAB))
                .as_path()
        )
    );
    assert!(picked_path.is_file());
}
