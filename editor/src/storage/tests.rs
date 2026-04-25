use super::editor_storage::*;
use crate::editor_assets::write_prefabs_lua;
use engine_core::constants::{extensions, paths};
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::{lua_dirs, lua_files};
use engine_core::storage::path_utils::sanitise_name;
use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;

mod toml_asset_registry_tests;

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
fn create_new_game_initializes_empty_asset_registry() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("asset_registry_default");
    set_game_name(test_game.name());

    let game = create_new_game(test_game.name().to_string());

    assert!(game.asset_registry.records().is_empty());
}

#[test]
fn save_game_round_trips_asset_registry_records() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("asset_registry_roundtrip");
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    game.asset_registry
        .insert(
            AssetKey::Sprite(SpriteId(7)),
            AssetRecord::new(PathBuf::from(paths::ASSETS_FOLDER).join("hero.png")),
        )
        .unwrap();
    game.asset_registry
        .insert(
            AssetKey::Prefab(PrefabId(9)),
            AssetRecord::new(
                PathBuf::from(paths::PREFABS_FOLDER).join(format!("crate.{}", extensions::PREFAB)),
            ),
        )
        .unwrap();

    save_game(&game).unwrap();

    let loaded = load_game_by_name(test_game.name()).unwrap();

    assert_eq!(
        loaded.asset_registry.records(),
        game.asset_registry.records()
    );
    assert_eq!(
        loaded
            .asset_registry
            .key_for_path(PathBuf::from(paths::ASSETS_FOLDER).join("hero.png")),
        Some(AssetKey::Sprite(SpriteId(7)))
    );
}

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
        game.asset_registry
            .record(AssetKey::Prefab(stale_prefab_id)),
        None
    );
    assert_eq!(game.asset_registry.key_for_path(&stale_path), None);
}

#[test]
fn save_game_persists_asset_identities_in_asset_registry() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("asset_registry_manager_cache_schema");
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    game.asset_registry
        .register_asset_relative_path(SpriteId(1), "sprites/player.png")
        .expect("sprite path should register");
    game.asset_registry
        .register_asset_relative_path(ScriptId(1), "player.lua")
        .expect("script path should register");

    save_game(&game).expect("game should save");

    let ron = std::fs::read_to_string(resources_folder(test_game.name()).join(paths::GAME_RON))
        .expect("saved game.ron should be readable");
    let mut loaded = load_game_by_name(test_game.name()).expect("saved game should load");
    SpriteManager::init_editor_metadata(&loaded.asset_registry, &mut loaded.sprite_manager);
    ScriptManager::init_editor_metadata(&loaded.asset_registry, &mut loaded.script_manager);

    assert_eq!(
        loaded.asset_registry.records(),
        game.asset_registry.records()
    );
    assert_eq!(
        loaded.sprite_manager.path_for_id(SpriteId(1)),
        Some(Path::new("sprites/player.png"))
    );
    assert_eq!(
        loaded.script_manager.path_for_id(ScriptId(1)),
        Some(Path::new("player.lua"))
    );

    assert!(ron.contains("asset_registry"));
    assert!(!ron.contains("kind:"));
}

#[test]
fn load_game_accepts_legacy_asset_registry_records_with_kind_field() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("asset_registry_legacy_kind_load");
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    game.asset_registry
        .register_asset_relative_path(SpriteId(1), "sprites/player.png")
        .expect("sprite path should register");

    save_game(&game).expect("game should save");

    let game_ron_path = resources_folder(test_game.name()).join(paths::GAME_RON);
    let legacy_ron = fs::read_to_string(&game_ron_path)
        .expect("saved game.ron should be readable")
        .replacen(
            "path: \"assets/sprites/player.png\"",
            "kind: Sprite,\n                path: \"assets/sprites/player.png\"",
            1,
        );
    fs::write(&game_ron_path, legacy_ron).expect("legacy schema should be writable");

    let loaded = load_game_by_name(test_game.name()).expect("legacy asset registry should load");

    assert_eq!(
        loaded.asset_registry.relative_path(SpriteId(1)),
        Some(PathBuf::from("sprites/player.png"))
    );
}

#[test]
fn shipped_demo_game_loads_with_slim_asset_registry_records() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());

    let loaded = load_game_by_name("Demo").expect("shipped Demo game should load");

    assert!(!loaded.asset_registry.records().is_empty());
}

#[test]
fn save_game_round_trips_sound_asset_registry_records() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("sound_asset_registry_roundtrip");
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    let sound_id = SoundId(4);
    let relative_path = PathBuf::from(paths::SFX_FOLDER).join("jump.wav");
    game.asset_registry
        .register_asset_relative_path(sound_id, &relative_path)
        .unwrap();

    let entity = game.ecs.create_entity().finish();
    let mut source = AudioSource::default();
    source.groups.insert(
        SoundGroupId::Custom("Jump".to_string()),
        AudioGroup {
            sounds: vec![sound_id],
            ..Default::default()
        },
    );
    game.ecs.add_component_to_entity(entity, source);

    save_game(&game).unwrap();
    let loaded = load_game_by_name(test_game.name()).unwrap();
    let loaded_source = AudioSource::store(&loaded.ecs)
        .data
        .values()
        .next()
        .unwrap();

    assert_eq!(
        loaded.asset_registry.relative_path(sound_id),
        Some(relative_path)
    );
    assert_eq!(
        loaded_source
            .groups
            .get(&SoundGroupId::Custom("Jump".to_string()))
            .unwrap()
            .sounds,
        vec![sound_id]
    );
}

#[test]
fn load_game_by_name_returns_invalid_data_for_corrupt_asset_registry() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("asset_registry_corrupt_load");
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    game.asset_registry
        .insert(
            AssetKey::Sprite(SpriteId(7)),
            AssetRecord::new(PathBuf::from(paths::ASSETS_FOLDER).join("hero.png")),
        )
        .unwrap();
    game.asset_registry
        .insert(
            AssetKey::Sprite(SpriteId(8)),
            AssetRecord::new(PathBuf::from(paths::ASSETS_FOLDER).join("villain.png")),
        )
        .unwrap();

    save_game(&game).unwrap();

    let game_ron_path = resources_folder(test_game.name()).join(paths::GAME_RON);
    let corrupt_ron =
        fs::read_to_string(&game_ron_path)
            .unwrap()
            .replacen("villain.png", "hero.png", 1);
    fs::write(&game_ron_path, corrupt_ron).unwrap();

    let error = match load_game_by_name(test_game.name()) {
        Ok(_) => panic!("corrupt asset registry should fail"),
        Err(error) => error,
    };

    assert_eq!(error.kind(), ErrorKind::InvalidData);
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

    persist_prefab(test_game.name(), &prefab).unwrap();

    let expected_path = prefabs_folder().join(format!(
        "{}.{}",
        sanitise_name(&prefab.name),
        extensions::PREFAB
    ));
    assert!(expected_path.is_file());

    let loaded = load_prefab(test_game.name(), prefab.id).unwrap();
    let listed = list_prefabs(test_game.name()).unwrap();

    assert_eq!(loaded, prefab);
    assert_eq!(listed, vec![prefab.clone()]);
    assert_eq!(
        load_prefab_manager(test_game.name(), &mut AssetRegistry::default())
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
        .join(lua_files::PREFABS);
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
        .join(lua_files::PREFABS);
    let contents = std::fs::read_to_string(prefabs_path).unwrap();

    assert!(contents.contains("BossAttack = \"Boss Attack\""));
    assert!(contents.contains("BossAttack_2 = \"Boss-Attack\""));
    assert!(contents.contains("Crate = \"Crate\""));
}

#[test]
fn generated_lua_typings_hide_prefab_internal_components() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let engine_dir = root.join(lua_dirs::SCRIPTS).join(lua_dirs::ENGINE);
    let components = std::fs::read_to_string(engine_dir.join(lua_files::COMPONENTS)).unwrap();
    let entity = std::fs::read_to_string(engine_dir.join(lua_files::ENTITY)).unwrap();
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
