use super::*;

const RESERVED_RUNTIME_SAVES_FOLDER: &str = "_runtime_saves";

struct ReservedRuntimeSavesRoot {
    path: std::path::PathBuf,
}

impl ReservedRuntimeSavesRoot {
    fn new() -> Self {
        let path = absolute_save_root().join(RESERVED_RUNTIME_SAVES_FOLDER);
        let _ = fs::remove_dir_all(&path);
        Self { path }
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for ReservedRuntimeSavesRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

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
fn shipped_demo_game_loads_with_slim_asset_registry_records() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());

    let loaded = load_game_by_name("Demo").expect("shipped Demo game should load");

    assert!(!loaded.asset_registry.records().is_empty());
}

#[test]
fn load_game_by_name_rebuilds_room_entities_after_deserialize() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("room_entities_rebuild_on_load");
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    let room_id = game.worlds()[0].rooms()[0].id;
    let entity = game
        .ecs
        .create_entity()
        .with(Animation::default())
        .with_current_room(room_id)
        .finish();

    save_game(&game).unwrap();
    let loaded = load_game_by_name(test_game.name()).unwrap();

    assert!(
        loaded.ecs.entities_in_room(room_id).contains(&entity),
        "loaded game should rebuild room_entities for deserialized CurrentRoom components"
    );
}

#[test]
fn load_game_by_name_rebuilds_world_and_room_indexes_after_deserialize() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("world_room_index_rebuild_on_load");
    set_game_name(test_game.name());

    let game = create_new_game(test_game.name().to_string());
    let world_id = game.current_world().id;
    let room_id = game
        .current_world()
        .rooms()
        .first()
        .expect("new game should have one room")
        .id;

    save_game(&game).unwrap();
    let loaded = load_game_by_name(test_game.name()).unwrap();

    assert_eq!(
        loaded.get_world(world_id).map(|world| world.id),
        Some(world_id)
    );
    assert!(loaded.current_world().get_room(room_id).is_some());
}

#[test]
fn game_save_load_when_tile_registry_exists_then_loaded_registry_preserves_next_id() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("tile_registry_roundtrip");
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    let tile_id = game.tile_registry.insert(TileDef {
        sprite_id: SpriteId(3),
        components: vec![TileComponent::Walkable(false)],
    });

    save_game(&game).expect("save should succeed");
    let mut loaded = load_game_by_name(test_game.name()).expect("load should succeed");

    assert!(loaded.tile_registry.get(tile_id).is_some());

    let next_id = loaded.tile_registry.insert(TileDef {
        sprite_id: SpriteId(4),
        components: vec![TileComponent::Solid(true)],
    });
    assert!(next_id.0 > tile_id.0);
}

#[test]
fn room_save_load_when_tile_placements_are_entities_then_tile_links_persist() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("tile_placement_roundtrip");
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    let room_id = game.current_world().rooms()[0].id;
    let tile_id = game.tile_registry.insert(TileDef {
        sprite_id: SpriteId(13),
        components: vec![TileComponent::Walkable(false)],
    });
    let entity = game
        .ecs
        .create_entity()
        .with(TilePlacement::new(tile_id, 4, 1))
        .with_current_room(room_id)
        .finish();

    save_game(&game).expect("save should succeed");
    let loaded = load_game_by_name(test_game.name()).expect("load should succeed");

    assert!(loaded.ecs.entities_in_room(room_id).contains(&entity));

    let loaded_tile = loaded
        .ecs
        .get::<TilePlacement>(entity)
        .expect("tile placement should load");
    assert_eq!(loaded_tile.definition, tile_id);
    assert_eq!((loaded_tile.grid_x, loaded_tile.grid_y), (4, 1));
}

#[test]
fn game_save_load_when_tile_registry_has_multiple_defs_then_all_defs_persist() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("tile_registry_multiple_defs");
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    let walkable_id = game.tile_registry.insert(TileDef {
        sprite_id: SpriteId(4),
        components: vec![TileComponent::Walkable(true)],
    });
    let solid_id = game.tile_registry.insert(TileDef {
        sprite_id: SpriteId(5),
        components: vec![TileComponent::Solid(true)],
    });

    save_game(&game).expect("save should succeed");
    let loaded = load_game_by_name(test_game.name()).expect("load should succeed");

    assert!(loaded.tile_registry.get(walkable_id).is_some());
    assert!(loaded.tile_registry.get(solid_id).is_some());
}

#[test]
fn list_game_names_ignores_non_game_directories() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("editor_game_name_listing");
    let reserved = ReservedRuntimeSavesRoot::new();
    set_game_name(test_game.name());
    create_new_game(test_game.name().to_string());
    fs::create_dir_all(reserved.path()).unwrap();

    let game_names = list_game_names();
    let expected = sanitise_name(test_game.name());

    assert!(game_names.iter().any(|name| name == &expected));
    assert!(!game_names
        .iter()
        .any(|name| name == RESERVED_RUNTIME_SAVES_FOLDER));
}

#[test]
fn most_recent_game_name_ignores_reserved_runtime_save_folder() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("editor_most_recent_game");
    let reserved = ReservedRuntimeSavesRoot::new();
    set_game_name(test_game.name());
    create_new_game(test_game.name().to_string());

    fs::create_dir_all(reserved.path()).unwrap();
    fs::write(reserved.path().join("touch.txt"), "latest").unwrap();

    let expected = sanitise_name(test_game.name());
    assert_eq!(most_recent_game_name().as_deref(), Some(expected.as_str()));
}

#[test]
fn save_game_writes_split_layout() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("split_layout_save");
    set_game_name(test_game.name());

    let game = create_new_game(test_game.name().to_string());
    save_game(&game).unwrap();

    let resources = resources_folder(test_game.name());
    assert!(resources.join(paths::GAME_RON).is_file());
    assert!(resources.join(paths::ASSET_REGISTRY_RON).is_file());
    assert!(resources.join(paths::TILE_REGISTRY_RON).is_file());
    assert!(resources.join(paths::WORLDS_FOLDER).join("world-1.ron").is_file());
    assert!(resources.join(paths::PAYLOADS_FOLDER).join("room-1.ron").is_file());
}

#[test]
fn save_game_when_worlds_and_entries_exist_regenerates_world_navigation_lua_files() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("world_navigation_lua_save");
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    let mut extra_world = crate::world::world_creation::create_new_world(&mut game);
    extra_world.name = "Second World".to_string();
    let extra_world_id = extra_world.id;
    let extra_room_id = extra_world.rooms()[0].id;
    game.add_world(extra_world);

    game.ecs
        .create_entity()
        .with(WorldEntry {
            name: "Main Entry".to_string(),
            ..Default::default()
        })
        .with_current_room(extra_room_id)
        .finish();

    save_game(&game).unwrap();

    let engine_folder = scripts_folder().join(lua_dirs::ENGINE);
    let worlds =
        fs::read_to_string(engine_folder.join("data/worlds.lua")).expect("worlds.lua should exist");
    let entries = fs::read_to_string(engine_folder.join("data/entries.lua"))
        .expect("entries.lua should exist");

    assert!(
        worlds.contains("SecondWorld = { Id = 2, Name = \"Second World\" }"),
        "{worlds}"
    );
    assert!(entries.contains("SecondWorld = {"), "{entries}");
    assert!(
        entries.contains(&format!(
            "MainEntry = {{ WorldId = {}, RoomId = {}, EntryName = \"Main Entry\" }}",
            extra_world_id.0, extra_room_id.0
        )),
        "{entries}"
    );
}

#[test]
fn load_game_by_name_restores_room_entities_from_split_layout() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("split_layout_load");
    set_game_name(test_game.name());

    let mut game = create_new_game(test_game.name().to_string());
    let room_id = game.current_world().rooms()[0].id;
    let entity = game
        .ecs
        .create_entity()
        .with(Animation::default())
        .with_current_room(room_id)
        .finish();

    save_game(&game).unwrap();
    let loaded = load_game_by_name(&game.name).unwrap();

    assert!(loaded.ecs.entities_in_room(room_id).contains(&entity));
}
