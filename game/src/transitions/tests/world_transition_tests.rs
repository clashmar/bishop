use crate::engine::game_instance::GameInstance;
use crate::transitions::world_transitions::*;
use bishop::prelude::Vec2;
use engine_core::ecs::*;
use engine_core::game::Game;
use engine_core::scripting::event_bus::EventBus;
use engine_core::scripting::lua_constants::lua_events;
use engine_core::scripting::lua_constants::lua_event_tag;
use engine_core::worlds::*;
use mlua::{Lua, Value, Variadic};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const ARCADE_WORLD_TAG: &str = "minigame";

const OVERWORLD: &str = "Overworld";
const ARCADE: &str = "Arcade";
const ARCADE_ENTRY: &str = "FromOverworld";
const NONEXISTENT_WORLD: &str = "Nowhere";
const NONEXISTENT_ENTRY: &str = "NoSuchEntry";
const BARE_WORLD: &str = "Bare";
const OVERWORLD_ID: WorldId = WorldId(1);
const ARCADE_ID: WorldId = WorldId(2);

const START_ENTRY: &str = WorldEntry::START_NAME;
const START_POS: Vec2 = Vec2::new(8.0, 8.0);

fn named_world(id: usize, name: &str, room_id: usize) -> World {
    let mut world = World::new(WorldId(id), name.to_string(), 16.0);
    world.add_room(Room {
        id: RoomId(room_id),
        ..Default::default()
    });
    world
}

/// Adds a "Start" WorldEntry entity to the ECS for the given world and room.
fn add_start_entry(game: &mut Game, room_id: RoomId, pos: Vec2) {
    game.ecs
        .create_entity()
        .with(WorldEntry { name: START_ENTRY.to_string(), is_start: true })
        .with(Transform { position: pos, ..Default::default() })
        .with_current_room(room_id)
        .finish();
}

fn two_world_instance() -> GameInstance {
    let mut game = Game::default();
    game.add_world(named_world(OVERWORLD_ID.0, OVERWORLD, 1));
    game.add_world(named_world(ARCADE_ID.0, ARCADE, 2));
    add_start_entry(&mut game, RoomId(1), START_POS);
    add_start_entry(&mut game, RoomId(2), START_POS);
    game.select_world(OVERWORLD_ID);
    GameInstance {
        game,
        prev_positions: HashMap::new(), 
        traversal_residency_diagnostics: None,
    }
}

#[test]
fn transport_uses_start_entry_position_when_present() {
    let specific_pos = Vec2::new(99.0, 55.0);
    let mut game = Game::default();
    game.add_world(named_world(OVERWORLD_ID.0, OVERWORLD, 1));
    game.add_world(named_world(ARCADE_ID.0, ARCADE, 2));
    add_start_entry(&mut game, RoomId(1), START_POS);
    add_start_entry(&mut game, RoomId(2), specific_pos);
    game.select_world(OVERWORLD_ID);
    let player = spawn_player(&mut game, RoomId(1));
    let mut instance = GameInstance { 
        game, 
        prev_positions: HashMap::new(), 
        traversal_residency_diagnostics: None 
    };

    let ok = WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &transport_request(player, ARCADE, None),
    );

    assert!(ok);
    assert_eq!(
        instance.game.ecs.get::<Transform>(player).map(|t| t.position),
        Some(specific_pos)
    );
}

#[test]
fn transport_falls_back_to_first_room_at_origin_when_no_start_entry() {
    let mut game = Game::default();
    game.add_world(named_world(OVERWORLD_ID.0, OVERWORLD, 1));
    game.add_world(named_world(ARCADE_ID.0, ARCADE, 2));
    // No Start entry for Arcade
    add_start_entry(&mut game, RoomId(1), START_POS);
    game.select_world(OVERWORLD_ID);
    let player = spawn_player(&mut game, RoomId(1));
    let mut instance = GameInstance { game, prev_positions: HashMap::new(), traversal_residency_diagnostics: None };

    let ok = WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &transport_request(player, ARCADE, None),
    );

    assert!(ok);
    assert_eq!(
        instance.game.ecs.get::<Transform>(player).map(|t| t.position),
        Some(Vec2::ZERO)
    );
}

fn spawn_player(game: &mut Game, room: RoomId) -> Entity {
    game.ecs
        .create_entity()
        .with(Transform::default())
        .with(Player)
        .with_current_room(room)
        .finish()
}

fn transport_request(entity: Entity, world: &str, entry: Option<&str>) -> TraversalRequest {
    TraversalRequest::transport(
        entity,
        WorldSelector::ByName(world.to_string()),
        entry.map(str::to_string),
    )
}

fn overlay_request(world: &str) -> TraversalRequest {
    TraversalRequest::overlay_world(WorldSelector::ByName(world.to_string()), None)
}

#[test]
fn transport_by_id_resolves_destination_world() {
    let mut instance = two_world_instance();
    let player = spawn_player(&mut instance.game, RoomId(1));

    let ok = WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &TraversalRequest::transport(player, WorldSelector::ById(ARCADE_ID), None),
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
        &TraversalRequest::transport(player, WorldSelector::ById(WorldId(99)), None),
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
        Some(START_POS)
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
            ..Default::default()
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
        &TraversalRequest {
            entity: None,
            destination: DestinationSelector::World {
                selector: WorldSelector::ByName(ARCADE.to_string()),
                entry_name: None,
            },
            mode: WorldTransitionMode::Transport,
        },
    );

    assert!(!ok);
}

#[test]
fn overlay_switches_world_without_moving_player() {
    let mut instance = two_world_instance();
    let player = spawn_player(&mut instance.game, RoomId(1));

    let ok = WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &overlay_request(ARCADE),
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
fn overlay_succeeds_for_bare_world_with_no_start_entry_using_first_room() {
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
        &overlay_request(BARE_WORLD),
    );

    assert!(ok);
    assert_eq!(instance.game.current_world().id, WorldId(3));
    assert_eq!(instance.game.current_world().current_room_id, Some(RoomId(9)));
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
                    Value::Table(_) => {} // ignore tags table
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

#[test]
fn overlay_pushes_frame_onto_stack() {
    let mut instance = two_world_instance();

    WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &overlay_request(ARCADE),
    );

    assert_eq!(instance.game.overlay_stack.len(), 1);
    assert_eq!(instance.game.overlay_stack[0].world, OVERWORLD_ID);
}

#[test]
fn transport_does_not_push_to_stack() {
    let mut instance = two_world_instance();
    let player = spawn_player(&mut instance.game, RoomId(1));

    WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &transport_request(player, ARCADE, None),
    );

    assert!(instance.game.overlay_stack.is_empty());
}

#[test]
fn return_from_world_pops_stack_and_transitions_back() {
    let mut instance = two_world_instance();

    // Overlay Arcade (pushes Overworld onto stack)
    WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &overlay_request(ARCADE),
    );
    assert_eq!(instance.game.current_world().id, ARCADE_ID);

    // Return (should pop and go back to Overworld)
    let returned = WorldTransitionManager::execute_return(&Lua::new(), &mut instance);

    assert!(returned);
    assert_eq!(instance.game.current_world().id, OVERWORLD_ID);
    assert!(instance.game.overlay_stack.is_empty());
}

#[test]
fn return_from_world_on_empty_stack_is_noop() {
    let mut instance = two_world_instance();

    let returned = WorldTransitionManager::execute_return(&Lua::new(), &mut instance);

    assert!(!returned);
    assert_eq!(instance.game.current_world().id, OVERWORLD_ID);
}

#[test]
fn nested_activation_pops_correctly() {
    let mut game = Game::default();
    game.add_world(named_world(1, "A", 1));
    game.add_world(named_world(2, "B", 2));
    game.add_world(named_world(3, "C", 3));
    add_start_entry(&mut game, RoomId(1), START_POS);
    add_start_entry(&mut game, RoomId(2), START_POS);
    add_start_entry(&mut game, RoomId(3), START_POS);
    game.select_world(WorldId(1));
    let mut instance = GameInstance { game, prev_positions: HashMap::new(), traversal_residency_diagnostics: None };

    WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &TraversalRequest::overlay_world(WorldSelector::ById(WorldId(2)), None),
    );
    WorldTransitionManager::execute(
        &Lua::new(),
        &mut instance,
        &TraversalRequest::overlay_world(WorldSelector::ById(WorldId(3)), None),
    );

    assert_eq!(instance.game.overlay_stack.len(), 2);

    WorldTransitionManager::execute_return(&Lua::new(), &mut instance);
    assert_eq!(instance.game.current_world().id, WorldId(2));
    assert_eq!(instance.game.overlay_stack.len(), 1);

    WorldTransitionManager::execute_return(&Lua::new(), &mut instance);
    assert_eq!(instance.game.current_world().id, WorldId(1));
    assert!(instance.game.overlay_stack.is_empty());
}

#[test]
fn world_exit_return_destination_pops_stack() {
    let mut instance = two_world_instance();

    // Overlay Arcade (pushes Overworld)
    WorldTransitionManager::execute(&Lua::new(), &mut instance, &overlay_request(ARCADE));
    assert_eq!(instance.game.current_world().id, ARCADE_ID);

    // A WorldExit with Return destination
    let exit_entity = instance.game.ecs.create_entity()
        .with(WorldExit {
            destination: Some(ExitDestination::Return),
            trigger: WorldExitTrigger::OnInteract,
            ..Default::default()
        })
        .with_current_room(RoomId(2))
        .finish();

    WorldTransitionManager::execute_from_exit(&Lua::new(), &mut instance, exit_entity);

    assert_eq!(instance.game.current_world().id, OVERWORLD_ID);
    assert!(instance.game.overlay_stack.is_empty());
}

#[test]
fn world_entered_event_includes_world_tags() {
    use engine_core::scripting::event_tags::event_tag::EventTag;

    let lua = Lua::new();
    let event_bus = engine_core::scripting::event_bus::EventBus::default();
    let received = Arc::new(Mutex::new(Vec::<String>::new()));
    let captured = received.clone();
    let handler = lua.create_function(move |_lua, args: Variadic<Value>| {
        let mut values = captured.lock().unwrap();
        for arg in args {
            match arg {
                Value::Integer(n) => values.push(n.to_string()),
                Value::String(s) => values.push(s.to_str().unwrap().to_string()),
                Value::Table(t) => {
                    let mut tag_strings: Vec<String> = t.sequence_values::<String>().flatten().collect();
                    tag_strings.sort();
                    values.extend(tag_strings);
                }
                _ => {}
            }
        }
        Ok(())
    }).unwrap();
    event_bus.on(lua_events::WORLD_ENTERED.to_string(), handler);

    let mut game = Game::default();
    let mut arcade = named_world(ARCADE_ID.0, ARCADE, 2);
    arcade.tags = vec![EventTag::Autosave, EventTag::Custom(ARCADE_WORLD_TAG.to_string())];
    game.add_world(named_world(OVERWORLD_ID.0, OVERWORLD, 1));
    game.add_world(arcade);
    add_start_entry(&mut game, RoomId(1), START_POS);
    add_start_entry(&mut game, RoomId(2), START_POS);
    game.select_world(OVERWORLD_ID);
    game.script_manager.event_bus = event_bus;
    let mut instance = GameInstance { game, prev_positions: HashMap::new(), traversal_residency_diagnostics: None };

    WorldTransitionManager::execute(&lua, &mut instance, &overlay_request(ARCADE));

    let values = received.lock().unwrap();
    // Payload: world_id, world_name, [sorted tag strings]
    assert!(values.contains(&ARCADE_ID.0.to_string()));
    assert!(values.contains(&ARCADE.to_string()));
    assert!(values.contains(&lua_event_tag::AUTOSAVE.to_string()));
    assert!(values.contains(&ARCADE_WORLD_TAG.to_string()));
}

#[test]
fn resolve_world_start_finds_is_start_entry() {
    let mut game = Game::default();
    let mut world = named_world(1, "TestWorld", 1);
    world.add_room(Room { id: RoomId(2), ..Default::default() });
    game.add_world(world);

    game.ecs
        .create_entity()
        .with(WorldEntry { name: "Side".to_string(), is_start: false })
        .with(Transform { position: Vec2::new(4.0, 4.0), ..Default::default() })
        .with_current_room(RoomId(1))
        .finish();

    game.ecs
        .create_entity()
        .with(WorldEntry { name: "Main".to_string(), is_start: true })
        .with(Transform { position: Vec2::new(8.0, 8.0), ..Default::default() })
        .with_current_room(RoomId(2))
        .finish();

    let world_ref = game.get_world(WorldId(1)).unwrap();
    let dest = resolve_world_start(&game, world_ref);
    assert!(dest.is_some());
    assert_eq!(dest.unwrap().room_id, RoomId(2));
}

#[test]
fn resolve_world_start_falls_back_to_first_room_when_no_is_start_entry() {
    let mut game = Game::default();
    let world = named_world(1, "TestWorld", 1);
    game.add_world(world);

    game.ecs
        .create_entity()
        .with(WorldEntry { name: "Side".to_string(), is_start: false })
        .with_current_room(RoomId(1))
        .finish();

    let world_ref = game.get_world(WorldId(1)).unwrap();
    let dest = resolve_world_start(&game, world_ref);
    assert!(dest.is_some());
    assert_eq!(dest.unwrap().room_id, RoomId(1));
}
