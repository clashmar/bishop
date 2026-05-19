use super::*;
use crate::ecs::Ecs;
#[cfg(feature = "editor")]
use crate::ecs::{CurrentRoom, Name};
use crate::worlds::room::{Room, RoomId};

#[test]
fn game_ctx_mut_can_exist_without_a_current_world() {
    let mut ecs = Ecs::default();
    let mut asset_registry = AssetRegistry::default();
    let mut sprite_manager = SpriteManager::default();
    let mut script_manager = ScriptManager::default();
    let prefab_manager = PrefabManager::default();

    let ctx = GameCtxMut {
        ecs: &mut ecs,
        world: None,
        asset_registry: &mut asset_registry,
        sprite_manager: &mut sprite_manager,
        script_manager: &mut script_manager,
        prefab_manager: &prefab_manager,
    };

    assert!(ctx.world.is_none());
}

#[test]
fn current_world_mut_returns_none_when_no_worlds() {
    let mut game = Game::default();
    assert!(game.current_world_mut().is_none());
}

#[test]
fn get_world_mut_returns_none_for_missing_id() {
    let mut game = Game::default();
    assert!(game.get_world_mut(WorldId(99)).is_none());
}

#[cfg(feature = "editor")]
#[test]
fn delete_world_sets_current_to_dummy_when_empty() {
    let mut game = Game::default();
    let world_id = game.id_allocator.allocate_world_id();
    game.add_world(World::new(world_id, String::new(), 16.0));
    game.delete_world(world_id);
    assert_eq!(game.current_world_id, Some(WorldId::default()));
}

#[cfg(feature = "editor")]
#[test]
fn delete_world_sets_current_to_remaining_world() {
    let mut game = Game::default();
    let w1 = game.id_allocator.allocate_world_id();
    let w2 = game.id_allocator.allocate_world_id();
    game.add_world(World::new(w1, "a".to_string(), 16.0));
    game.add_world(World::new(w2, "b".to_string(), 16.0));
    game.delete_world(w2);
    assert_eq!(game.current_world_id, Some(w1));
}

#[cfg(feature = "editor")]
#[test]
fn delete_world_removes_all_room_entities() {
    let mut game = Game::default();
    let world_id = game.id_allocator.allocate_world_id();
    let room_id = game.id_allocator.allocate_room_id();
    game.add_world(World::from_rooms(
        world_id,
        String::new(),
        vec![Room {
            id: room_id,
            ..Default::default()
        }],
        16.0,
    ));

    let entity = game
        .ecs
        .create_entity()
        .with(Name("test_entity".into()))
        .with_current_room(room_id)
        .finish();

    game.delete_world(world_id);

    assert!(
        !game.ecs.has::<CurrentRoom>(entity),
        "CurrentRoom should be gone after world deletion"
    );
    assert!(
        !game.ecs.has::<Name>(entity),
        "Name should be gone after world deletion"
    );
    assert!(
        game.ecs.room_entities.is_empty(),
        "room_entities should be empty after world deletion"
    );
}

#[cfg(feature = "editor")]
#[test]
fn delete_world_collects_entities_from_room_index_only_for_target_world() {
    let mut game = Game::default();
    let world_a = game.id_allocator.allocate_world_id();
    let world_b = game.id_allocator.allocate_world_id();
    let room_a = game.id_allocator.allocate_room_id();
    let room_b = game.id_allocator.allocate_room_id();

    game.add_world(World::from_rooms(
        world_a,
        String::new(),
        vec![Room {
            id: room_a,
            ..Default::default()
        }],
        16.0,
    ));
    game.add_world(World::from_rooms(
        world_b,
        String::new(),
        vec![Room {
            id: room_b,
            ..Default::default()
        }],
        16.0,
    ));

    let entity_a = game.ecs.create_entity().with_current_room(room_a).finish();
    let entity_b = game.ecs.create_entity().with_current_room(room_b).finish();

    game.delete_world(world_a);

    assert!(!game.ecs.has::<CurrentRoom>(entity_a));
    assert!(game.ecs.has::<CurrentRoom>(entity_b));
}

#[cfg(feature = "editor")]
#[test]
fn delete_world_preserves_other_world_entities() {
    let mut game = Game::default();
    let world_a = game.id_allocator.allocate_world_id();
    let world_b = game.id_allocator.allocate_world_id();
    let room_a = game.id_allocator.allocate_room_id();
    let room_b = game.id_allocator.allocate_room_id();

    game.add_world(World::from_rooms(
        world_a,
        String::new(),
        vec![Room {
            id: room_a,
            ..Default::default()
        }],
        16.0,
    ));
    game.add_world(World::from_rooms(
        world_b,
        String::new(),
        vec![Room {
            id: room_b,
            ..Default::default()
        }],
        16.0,
    ));

    let entity_a = game.ecs.create_entity().with_current_room(room_a).finish();
    let entity_b = game.ecs.create_entity().with_current_room(room_b).finish();

    game.delete_world(world_a);

    assert!(
        !game.ecs.has::<CurrentRoom>(entity_a),
        "entity_a should be gone after its world is deleted"
    );
    assert!(
        game.ecs.has::<CurrentRoom>(entity_b),
        "entity_b should still exist after the other world is deleted"
    );
    assert_eq!(
        game.ecs.room_entities.len(),
        1,
        "only room_b should remain in room_entities"
    );
    assert!(
        !game.ecs.room_entities.contains_key(&room_a),
        "room_a should not be in room_entities"
    );
    assert!(
        game.ecs
            .room_entities
            .get(&room_b)
            .unwrap()
            .contains(&entity_b),
        "entity_b should be tracked for room_b in room_entities"
    );
}

#[test]
fn world_index_current_world_returns_selected_world() {
    let mut game = Game::default();
    game.add_world(World::new(WorldId(1), "a".to_string(), 16.0));
    game.add_world(World::new(WorldId(2), "b".to_string(), 16.0));

    game.select_world(WorldId(1));

    assert_eq!(game.current_world().id, WorldId(1));
}

#[test]
fn world_index_get_world_mut_returns_inserted_world() {
    let mut game = Game::default();
    let world = World::new(WorldId(7), "inserted".to_string(), 16.0);

    game.insert_world(0, world);

    assert_eq!(game.get_world(WorldId(7)).map(|world| world.id), Some(WorldId(7)));
    assert!(game.get_world_mut(WorldId(7)).is_some());
}

#[test]
fn world_index_rebuild_tracks_swap_remove_after_rebuild() {
    let mut game = Game::default();
    game.add_world(World::new(WorldId(1), "a".to_string(), 16.0));
    game.add_world(World::new(WorldId(2), "b".to_string(), 16.0));

    game.worlds.swap_remove(0);
    game.rebuild_world_index();

    assert_eq!(game.get_world(WorldId(2)).map(|world| world.id), Some(WorldId(2)));
    assert!(game.get_world(WorldId(1)).is_none());
}

#[test]
fn initialize_rebuilds_id_allocator() {
    let mut game = Game::default();
    let w1 = game.id_allocator.allocate_world_id();
    let r1 = game.id_allocator.allocate_room_id();
    game.add_world(World::from_rooms(
        w1,
        String::new(),
        vec![Room {
            id: r1,
            ..Default::default()
        }],
        16.0,
    ));
    game.id_allocator = IdAllocator::default();
    game.id_allocator = IdAllocator::from_game(&game);
    assert!(game.id_allocator.allocate_world_id().0 > w1.0);
    let next_room = game.id_allocator.allocate_room_id();
    assert!(next_room.0 > r1.0);
}

#[cfg(feature = "editor")]
#[test]
fn reload_prefab_manager_keeps_existing_records_when_reload_fails() {
    use crate::constants::extensions;
    use crate::engine_global::set_game_name;
    use crate::prefab::{create_prefab, persist_prefab, PrefabId};
    use crate::storage::path_utils::prefabs_folder;
    use crate::storage::test_utils::{game_fs_test_lock, TestGameFolder};
    use std::path::PathBuf;

    let _lock = game_fs_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let test_folder = TestGameFolder::new("prefab_registry_no_partial_cleanup");
    set_game_name(test_folder.name());
    let first = create_prefab(PrefabId(3), "Bullet".to_string());
    let second = create_prefab(PrefabId(9), "Bullet".to_string());
    let stale_prefab_id = PrefabId(22);
    let stale_relative_path = PathBuf::from(format!("stale_prefab.{}", extensions::PREFAB));
    let mut game = Game {
        name: test_folder.name().to_string(),
        ..Default::default()
    };

    persist_prefab(test_folder.name(), &first, &AssetRegistry::default(), None).unwrap();
    game.reload_prefab_manager();
    std::fs::write(
        prefabs_folder().join(format!("bullet_copy.{}", extensions::PREFAB)),
        ron::to_string(&second).unwrap(),
    )
    .unwrap();
    game.asset_registry
        .register_asset_relative_path(stale_prefab_id, &stale_relative_path)
        .unwrap();

    let before = game.asset_registry.clone();
    let before_prefab_manager = game.prefab_manager.clone();

    game.reload_prefab_manager();

    assert_eq!(game.asset_registry, before);
    assert_eq!(game.prefab_manager, before_prefab_manager);
}

#[test]
fn room_new_indexes_room_camera() {
    let mut ecs = Ecs::default();
    let room_id = RoomId(1);
    let grid_size = 16.0;

    let room = Room {
        id: room_id,
        ..Default::default()
    };

    room.create_room_camera(&mut ecs, room_id, grid_size);

    // Camera entity should be tracked in ecs.room_entities via the ECS index
    let camera_entities = ecs.entities_in_room(room_id);
    assert_eq!(camera_entities.len(), 1, "camera should be tracked in room_entities");
}
