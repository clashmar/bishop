use super::*;

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
    let corrupt_ron = fs::read_to_string(&game_ron_path)
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
            PathBuf::from(paths::TEXT_FOLDER)
                .join("dialogue")
                .join("npcs")
                .join("npc.toml"),
        ))
    );
}

#[test]
fn save_game_round_trips_script_toml_field_values() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("script_toml_field_roundtrip");
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    let toml_id = TomlId(3);
    game.asset_registry
        .register_asset_relative_path(toml_id, PathBuf::from("dialogue/npcs/npc.toml"))
        .unwrap();

    let entity = game.ecs.create_entity().finish();
    game.ecs.add_component_to_entity(
        entity,
        Script {
            script_id: ScriptId(1),
            data: ScriptData {
                fields: [("dialogue_id".to_string(), ScriptField::Toml(toml_id))]
                    .into_iter()
                    .collect(),
            },
        },
    );

    save_game(&game).unwrap();
    let loaded = load_game_by_name(test_game.name()).unwrap();
    let loaded_script = loaded.ecs.get::<Script>(entity).unwrap();
    assert!(matches!(
        loaded_script.data.fields.get("dialogue_id"),
        Some(ScriptField::Toml(id)) if *id == toml_id
    ));
}
