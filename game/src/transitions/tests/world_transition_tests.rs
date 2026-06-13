use crate::engine::game_instance::GameInstance;
use crate::transitions::world_transitions::*;
use bishop::prelude::Vec2;
use engine_core::ecs::*;
use engine_core::game::Game;
use engine_core::scripting::event_bus::EventBus;
use engine_core::scripting::lua_constants::lua_events;
use engine_core::worlds::*;
use mlua::{Lua, Value, Variadic};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const OVERWORLD: &str = "Overworld";
const ARCADE: &str = "Arcade";
const ARCADE_ENTRY: &str = "FromOverworld";
const NONEXISTENT_WORLD: &str = "Nowhere";
const NONEXISTENT_ENTRY: &str = "NoSuchEntry";
const BARE_WORLD: &str = "Bare";
const OVERWORLD_ID: WorldId = WorldId(1);
const ARCADE_ID: WorldId = WorldId(2);

fn named_world(id: usize, name: &str, room_id: usize) -> World {
    let mut world = World::new(WorldId(id), name.to_string(), 16.0);
    world.add_room(Room {
        id: RoomId(room_id),
        ..Default::default()
    });
    world.starting_room_id = Some(RoomId(room_id));
    world.starting_position = Some(Vec2::new(8.0, 8.0));
    world
}

fn two_world_instance() -> GameInstance {
    let mut game = Game::default();
    game.add_world(named_world(OVERWORLD_ID.0, OVERWORLD, 1));
    game.add_world(named_world(ARCADE_ID.0, ARCADE, 2));
    game.select_world(OVERWORLD_ID);
    GameInstance {
        game,
        prev_positions: HashMap::new(),
    }
}

fn spawn_player(game: &mut Game, room: RoomId) -> Entity {
    game.ecs
        .create_entity()
        .with(Transform::default())
        .with(Player)
        .with_current_room(room)
        .finish()
}

fn transport_request(entity: Entity, world: &str, entry: Option<&str>) -> WorldTransitionRequest {
    WorldTransitionRequest {
        entity: Some(entity),
        world: WorldSelector::ByName(world.to_string()),
        entry_name: entry.map(str::to_string),
        mode: WorldTransitionMode::Transport,
    }
}

fn activate_request(world: &str) -> WorldTransitionRequest {
    WorldTransitionRequest {
        entity: None,
        world: WorldSelector::ByName(world.to_string()),
        entry_name: None,
        mode: WorldTransitionMode::Activate,
    }
}

#[test]
fn transport_by_id_resolves_destination_world() {
    let mut instance = two_world_instance();
    let player = spawn_player(&mut instance.game, RoomId(1));

    let ok = WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &WorldTransitionRequest {
            entity: Some(player),
            world: WorldSelector::ById(ARCADE_ID),
            entry_name: None,
            mode: WorldTransitionMode::Transport,
        },
    );

    assert!(ok);
    assert_eq!(instance.game.current_world().id, ARCADE_ID);
}

#[test]
fn transport_by_unknown_id_returns_false_without_state_change() {
    let mut instance = two_world_instance();
    let player = spawn_player(&mut instance.game, RoomId(1));

    let ok = WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &WorldTransitionRequest {
            entity: Some(player),
            world: WorldSelector::ById(WorldId(99)),
            entry_name: None,
            mode: WorldTransitionMode::Transport,
        },
    );

    assert!(!ok);
    assert_eq!(instance.game.current_world().id, OVERWORLD_ID);
}

#[test]
fn transport_moves_player_and_switches_active_world() {
    let mut instance = two_world_instance();
    let player = spawn_player(&mut instance.game, RoomId(1));

    let ok = WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &transport_request(player, ARCADE, None),
    );

    assert!(ok);
    assert_eq!(instance.game.current_world().id, ARCADE_ID);
    assert_eq!(
        instance.game.ecs.get::<CurrentRoom>(player).map(|r| r.0),
        Some(RoomId(2))
    );
    assert_eq!(
        instance.game.ecs.get::<Transform>(player).map(|t| t.position),
        Some(Vec2::new(8.0, 8.0))
    );
    assert_eq!(instance.game.current_world().current_room_id, Some(RoomId(2)));
}

#[test]
fn transport_uses_named_entry_room_and_position() {
    let mut instance = two_world_instance();
    let player = spawn_player(&mut instance.game, RoomId(1));
    instance
        .game
        .ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(42.0, 17.0),
            ..Default::default()
        })
        .with(WorldEntry {
            name: ARCADE_ENTRY.to_string(),
        })
        .with_current_room(RoomId(2))
        .finish();

    let ok = WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &transport_request(player, ARCADE, Some(ARCADE_ENTRY)),
    );

    assert!(ok);
    assert_eq!(
        instance.game.ecs.get::<Transform>(player).map(|t| t.position),
        Some(Vec2::new(42.0, 17.0))
    );
}

#[test]
fn transport_of_non_player_does_not_switch_active_world() {
    let mut instance = two_world_instance();
    let npc = instance
        .game
        .ecs
        .create_entity()
        .with(Transform::default())
        .with_current_room(RoomId(1))
        .finish();

    let ok = WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &transport_request(npc, ARCADE, None),
    );

    assert!(ok);
    assert_eq!(instance.game.current_world().id, OVERWORLD_ID);
    assert_eq!(
        instance.game.ecs.get::<CurrentRoom>(npc).map(|r| r.0),
        Some(RoomId(2))
    );
}

#[test]
fn transport_rejects_unknown_world_without_state_change() {
    let mut instance = two_world_instance();
    let player = spawn_player(&mut instance.game, RoomId(1));

    let ok = WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &transport_request(player, NONEXISTENT_WORLD, None),
    );

    assert!(!ok);
    assert_eq!(instance.game.current_world().id, OVERWORLD_ID);
    assert_eq!(
        instance.game.ecs.get::<CurrentRoom>(player).map(|r| r.0),
        Some(RoomId(1))
    );
}

#[test]
fn transport_rejects_unknown_entry_without_state_change() {
    let mut instance = two_world_instance();
    let player = spawn_player(&mut instance.game, RoomId(1));

    let ok = WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &transport_request(player, ARCADE, Some(NONEXISTENT_ENTRY)),
    );

    assert!(!ok);
    assert_eq!(instance.game.current_world().id, OVERWORLD_ID);
}

#[test]
fn transport_requires_a_subject_entity() {
    let mut instance = two_world_instance();

    let ok = WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &WorldTransitionRequest {
            entity: None,
            world: WorldSelector::ByName(ARCADE.to_string()),
            entry_name: None,
            mode: WorldTransitionMode::Transport,
        },
    );

    assert!(!ok);
}

#[test]
fn activate_switches_world_without_moving_player() {
    let mut instance = two_world_instance();
    let player = spawn_player(&mut instance.game, RoomId(1));

    let ok = WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &activate_request(ARCADE),
    );

    assert!(ok);
    assert_eq!(instance.game.current_world().id, ARCADE_ID);
    assert_eq!(instance.game.current_world().current_room_id, Some(RoomId(2)));
    assert_eq!(
        instance.game.ecs.get::<CurrentRoom>(player).map(|r| r.0),
        Some(RoomId(1))
    );
}

#[test]
fn activate_rejects_world_with_no_start_and_no_entry() {
    let mut instance = two_world_instance();
    let mut bare = World::new(WorldId(3), BARE_WORLD.to_string(), 16.0);
    bare.add_room(Room {
        id: RoomId(9),
        ..Default::default()
    });
    instance.game.add_world(bare);
    instance.game.select_world(OVERWORLD_ID);

    let ok = WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &activate_request(BARE_WORLD),
    );

    assert!(!ok);
    assert_eq!(instance.game.current_world().id, OVERWORLD_ID);
}

#[test]
fn player_transport_emits_world_entered() {
    let lua = Lua::new();
    let event_bus = EventBus::default();
    let received = Arc::new(Mutex::new(Vec::<String>::new()));
    let captured = received.clone();
    let handler = lua
        .create_function(move |_lua, args: Variadic<Value>| {
            let mut values = captured.lock().unwrap();
            for arg in args {
                match arg {
                    Value::Integer(n) => values.push(n.to_string()),
                    Value::String(s) => values.push(s.to_str().unwrap().to_string()),
                    other => panic!("unexpected value: {other:?}"),
                }
            }
            Ok(())
        })
        .unwrap();
    event_bus.on(lua_events::WORLD_ENTERED.to_string(), handler);

    let mut instance = two_world_instance();
    instance.game.script_manager.event_bus = event_bus;
    let player = spawn_player(&mut instance.game, RoomId(1));

    let ok = WorldTransitionManager::execute(
        &lua,
        &mut instance,
        &transport_request(player, ARCADE, None),
    );

    assert!(ok);
    let values = received.lock().unwrap();
    assert_eq!(*values, vec![ARCADE_ID.0.to_string(), ARCADE.to_string()]);
}
