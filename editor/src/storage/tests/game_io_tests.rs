use super::*;

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

    assert_eq!(loaded.get_world(world_id).map(|world| world.id), Some(world_id));
    assert!(loaded.current_world().get_room(room_id).is_some());
}

#[test]
fn list_game_names_ignores_non_game_directories() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("editor_game_name_listing");
    set_game_name(test_game.name());
    create_new_game(test_game.name().to_string());
    fs::create_dir_all(absolute_save_root().join("_runtime_saves")).unwrap();

    let game_names = list_game_names();
    let expected = sanitise_name(test_game.name());

    assert!(game_names.iter().any(|name| name == &expected));
    assert!(!game_names.iter().any(|name| name == "_runtime_saves"));
}

#[test]
fn most_recent_game_name_ignores_reserved_runtime_save_folder() {
    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_game = TestGameFolder::new("editor_most_recent_game");
    set_game_name(test_game.name());
    create_new_game(test_game.name().to_string());

    let reserved = absolute_save_root().join("_runtime_saves");
    fs::create_dir_all(&reserved).unwrap();
    fs::write(reserved.join("touch.txt"), "latest").unwrap();

    let expected = sanitise_name(test_game.name());
    assert_eq!(most_recent_game_name().as_deref(), Some(expected.as_str()));
}
