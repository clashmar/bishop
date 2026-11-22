use crate::engine::GameInstance;
use bishop::prelude::Vec2;
use engine_core::ecs::{
    CurrentRoom, Entity, Player, PlayerProxy, Transform, WorldEntry, get_parent, set_parent,
    update_entity_position,
};
use engine_core::engine_global::set_game_name;
use engine_core::game::Game;
use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use engine_core::worlds::{Room, RoomId, RoomLayer, World, WorldId};
use mlua::Lua;
use std::collections::HashMap;
use crate::transitions::world_transitions::{TraversalRequest, WorldTransitionManager};

#[test]
fn game_startup_uses_configured_world_not_the_last_editor_world() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("start_world_separate_from_editor");
    set_game_name(test_game.name());

    let mut game = Game::with_name(test_game.name());
    let mut first = World::new(WorldId(1), "First".to_string(), 16.0);
    first.add_room(Room { id: RoomId(1), ..Default::default() });
    game.add_world(first);
    let mut second = World::new(WorldId(2), "Second".to_string(), 16.0);
    second.add_room(Room { id: RoomId(2), ..Default::default() });
    game.add_world(second);
    game.select_world(WorldId(1));
    game.set_start_world(WorldId(2)).unwrap();

    let prepared = GameInstance::prepare_loaded_game(&Lua::new(), game).unwrap();

    assert_eq!(prepared.game.current_world_id, Some(WorldId(2)));
    assert_eq!(prepared.room_id, RoomId(2));
}

#[test]
fn game_startup_without_a_start_entry_uses_first_room_at_origin() {
    let _lock = game_fs_test_lock().lock().unwrap_or_else(|poison| poison.into_inner());
    let folder = TestGameFolder::new("start_entry_fallback");
    set_game_name(folder.name());
    let mut game = Game::with_name(folder.name());
    let mut world = World::new(WorldId(1), "Start".to_string(), 16.0);
    world.add_room(Room::new(&mut game.ecs, RoomId(1), 16.0));
    game.add_world(world);
    let player = game.ecs.create_entity().with(Player)
        .with(Transform { position: Vec2::new(68.0, 72.0), ..Default::default() })
        .finish();

    let prepared = GameInstance::prepare_loaded_game(&Lua::new(), game).unwrap();
    assert_eq!(prepared.game.ecs.get::<CurrentRoom>(player).copied(),
        Some(CurrentRoom::new(RoomId(1), RoomLayer::Front)));
    assert_eq!(prepared.game.ecs.get::<Transform>(player).unwrap().position, Vec2::ZERO);
}

#[test]
fn start_world_moves_player_children_with_player() {
    let (instance, player, child) = assert_start_entry_spawn(None);

    let player_position = instance
        .game
        .ecs
        .get::<Transform>(player)
        .map(|transform| transform.position);
    assert_eq!(
        instance.game.ecs.get::<Transform>(child).map(|transform| transform.position),
        player_position.map(|position| position + Vec2::new(3.0, 4.0))
    );
    assert_eq!(
        instance.game.ecs.get::<CurrentRoom>(child),
        instance.game.ecs.get::<CurrentRoom>(player)
    );
}

#[test]
fn game_startup_transfers_proxy_children_to_the_player() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let folder = TestGameFolder::new("start_proxy_children_transfer");
    set_game_name(folder.name());
    let mut game = Game::with_name(folder.name());
    let mut world = World::new(WorldId(1), "Start".to_string(), 16.0);
    world.add_room(Room::new(&mut game.ecs, RoomId(1), 16.0));
    game.add_world(world);
    let proxy_position = Vec2::new(68.0, 72.0);
    let proxy = game.ecs.create_entity()
        .with(PlayerProxy)
        .with(Transform { position: proxy_position, ..Default::default() })
        .with_current_room_layer(RoomId(1), RoomLayer::Front)
        .finish();
    let emitter_position = proxy_position + Vec2::new(0.0, 4.0);
    let emitter = game.ecs.create_entity()
        .with(Transform { position: emitter_position, ..Default::default() })
        .with_current_room_layer(RoomId(1), RoomLayer::Front)
        .finish();
    set_parent(&mut game.ecs, emitter, proxy);
    let player = game.ecs.create_entity()
        .with(Player)
        .with(Transform::default())
        .finish();

    let mut prepared = GameInstance::prepare_loaded_game(&Lua::new(), game).unwrap();

    assert_eq!(get_parent(&prepared.game.ecs, emitter), Some(player));
    assert_eq!(
        prepared.game.ecs.get::<CurrentRoom>(emitter),
        prepared.game.ecs.get::<CurrentRoom>(player)
    );
    let moved_position = proxy_position + Vec2::new(12.0, -3.0);
    update_entity_position(&mut prepared.game.ecs, player, moved_position);
    assert_eq!(
        prepared.game.ecs.get::<Transform>(emitter).map(|transform| transform.position),
        Some(moved_position + Vec2::new(0.0, 4.0))
    );
}

#[test]
fn game_startup_places_a_roomed_player_at_the_start_room_proxy() {
    assert_start_entry_spawn(Some(RoomId(1)));
}

#[test]
fn game_startup_places_an_unroomed_player_at_the_start_room_proxy() {
    assert_start_entry_spawn(None);
}

#[test]
fn restored_location_overrides_the_initial_start_entry_spawn() {
    let (mut instance, player, child) = assert_start_entry_spawn(None);
    let restored_position = Vec2::new(91.0, 42.0);
    let request = TraversalRequest::restore_location(
        player, WorldId(1), RoomId(1), RoomLayer::Front, restored_position,
    );

    assert!(WorldTransitionManager::execute(&Lua::new(), &mut instance, &request));
    assert_eq!(instance.game.ecs.get::<Transform>(player).unwrap().position, restored_position);
    assert_eq!(instance.game.ecs.get::<CurrentRoom>(player).copied(),
        Some(CurrentRoom::new(RoomId(1), RoomLayer::Front)));
    assert_eq!(
        instance.game.ecs.get::<Transform>(child).map(|transform| transform.position),
        Some(restored_position + Vec2::new(3.0, 4.0))
    );
}

fn assert_start_entry_spawn(initial_room: Option<RoomId>) -> (GameInstance, Entity, Entity) {
    let _lock = game_fs_test_lock().lock().unwrap();
    let folder = TestGameFolder::new("start_entry_spawn");
    set_game_name(folder.name());
    let mut game = Game::with_name(folder.name());
    let mut world = World::new(WorldId(1), "Start".to_string(), 16.0);
    world.add_room(Room::new(&mut game.ecs, RoomId(1), 16.0));
    world.add_room(Room::new(&mut game.ecs, RoomId(2), 16.0));
    game.add_world(world);
    let start_position = Vec2::new(20.0, 16.0);
    let proxy_position = Vec2::new(68.0, 72.0);
    game.ecs.create_entity().with(PlayerProxy)
        .with(Transform { position: proxy_position, ..Default::default() })
        .with_current_room_layer(RoomId(1), RoomLayer::Front).finish();
    game.ecs.create_entity().with(PlayerProxy)
        .with(Transform { position: Vec2::new(123.0, 145.0), ..Default::default() })
        .with_current_room(RoomId(2)).finish();
    game.ecs.create_entity()
        .with(WorldEntry { name: WorldEntry::START_NAME.to_string(), is_start: true })
        .with(Transform { position: start_position, ..Default::default() })
        .with_current_room_layer(RoomId(1), RoomLayer::Back)
        .finish();
    let player = game.ecs.create_entity()
        .with(Player)
        .with(Transform { position: Vec2::new(5.0, 6.0), ..Default::default() })
        .finish();
    let child = game.ecs.create_entity()
        .with(Transform { position: Vec2::new(8.0, 10.0), ..Default::default() })
        .finish();
    set_parent(&mut game.ecs, child, player);
    if let Some(room_id) = initial_room {
        game.ecs.set_current_room_layer(player, room_id, RoomLayer::Front);
    }

    let prepared = GameInstance::prepare_loaded_game(&Lua::new(), game).unwrap();
    assert_eq!(prepared.room_id, RoomId(1));
    assert_eq!(prepared.room_layer, RoomLayer::Back);
    assert_eq!(prepared.game.ecs.get::<CurrentRoom>(player).copied(),
        Some(CurrentRoom::new(RoomId(1), RoomLayer::Back)));
    assert_eq!(prepared.game.ecs.get::<Transform>(player).unwrap().position, proxy_position);
    assert!(prepared.game.ecs.get_store::<PlayerProxy>().data.is_empty());
    (GameInstance {
        game: prepared.game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    }, player, child)
}

#[test]
fn game_startup_rejects_a_missing_start_world() {
    let mut game = Game::default();
    game.start_world_id = Some(WorldId(99));
    assert!(GameInstance::prepare_loaded_game(&Lua::new(), game)
        .err()
        .unwrap()
        .contains("start world"));
}
