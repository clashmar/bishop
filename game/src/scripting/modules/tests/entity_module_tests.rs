use crate::engine::game_instance::GameInstance;
use crate::game_global::{drain_commands, take_pending_world_transition};
use crate::transitions::world_transitions::{DestinationSelector, WorldSelector};
use crate::scripting::commands::lua_command::LuaCommand;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::EntityHandle;
use engine_core::audio::command_queue::drain_audio_commands;
use engine_core::audio::{AudioCommand, AudioPlaybackOwner};
use engine_core::ecs::*;
use engine_core::game::Game;
use engine_core::worlds::*;
use engine_core::worlds::world::WorldExitTrigger;
use engine_core::scripting::lua_constants::lua_entity;
use engine_core::scripting::to_snake_case;
use mlua::Lua;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn setup_entity_lua() -> (Lua, Rc<RefCell<GameInstance>>, Entity) {
    let lua = Lua::new();
    let mut game = Game::default();
    let mut world = World::new(WorldId(1), "Overworld".to_string(), 16.0);
    world.add_room(Room {
        id: RoomId(1),
        ..Default::default()
    });
    world.add_room(Room {
        id: RoomId(2),
        ..Default::default()
    });
    world.current_room_id = Some(RoomId(1));
    game.add_world(world);

    let entity = game
        .ecs
        .create_entity()
        .with(Transform::default())
        .with_current_room(RoomId(1))
        .finish();
    let game_instance = Rc::new(RefCell::new(GameInstance {
        game,
        prev_positions: HashMap::new(), 
        traversal_residency_diagnostics: None,
    }));

    LuaGameCtx {
        game_instance: game_instance.clone(),
    }
    .set_lua_ctx(&lua)
    .unwrap();

    let entity_handle = lua.create_userdata(EntityHandle { entity }).unwrap();
    lua.globals().set("entity", entity_handle).unwrap();

    (lua, game_instance, entity)
}

fn assert_private_component_error(err: mlua::Error, type_name: &str) {
    let expected = format!("Component '{type_name}' is not available to Lua");
    assert!(
        err.to_string().contains(&expected),
        "unexpected error: {err:?}"
    );
}

#[test]
fn despawn_is_safe_to_call_on_an_already_removed_entity() {
    let (lua, game_instance, entity) = setup_entity_lua();

    let result = lua.load("entity:despawn(); entity:despawn()").exec();

    assert!(result.is_ok(), "unexpected error: {result:?}");
    assert!(game_instance
        .borrow()
        .game
        .ecs
        .get::<Transform>(entity)
        .is_none());
}

#[test]
fn get_errors_for_private_component_names() {
    let (lua, _game_instance, _entity) = setup_entity_lua();
    let type_name = comp_type_name::<PrefabInstanceRoot>();

    let err = lua
        .load(format!("return entity:get('{type_name}')"))
        .eval::<bool>()
        .unwrap_err();

    assert_private_component_error(err, type_name);
}

#[test]
fn has_errors_for_private_component_names() {
    let (lua, _game_instance, _entity) = setup_entity_lua();
    let type_name = comp_type_name::<PrefabInstanceRoot>();

    let err = lua
        .load(format!("return entity:has('{type_name}')"))
        .eval::<bool>()
        .unwrap_err();

    assert_private_component_error(err, type_name);
}

#[test]
fn set_errors_for_private_component_names() {
    let (lua, _game_instance, _entity) = setup_entity_lua();
    let type_name = comp_type_name::<PrefabInstanceRoot>();

    let err = lua
        .load(format!("entity:set('{type_name}', {{}})"))
        .exec()
        .unwrap_err();

    assert_private_component_error(err, type_name);
}

#[test]
fn variadic_has_errors_for_private_component_names() {
    let (lua, _game_instance, _entity) = setup_entity_lua();
    let private_type = comp_type_name::<PrefabInstanceRoot>();
    let public_transform = comp_type_name::<Transform>();
    let public_name = comp_type_name::<Name>();

    let has_any_err = lua
        .load(format!("return entity:has_any('{private_type}')"))
        .eval::<bool>()
        .unwrap_err();
    assert_private_component_error(has_any_err, private_type);

    let has_all_err = lua
        .load(format!("return entity:has_all('{private_type}')"))
        .eval::<bool>()
        .unwrap_err();
    assert_private_component_error(has_all_err, private_type);

    let mixed_has_any_err = lua
        .load(format!(
            "return entity:has_any('{public_transform}', '{private_type}')"
        ))
        .eval::<bool>()
        .unwrap_err();
    assert_private_component_error(mixed_has_any_err, private_type);

    let mixed_has_all_err = lua
        .load(format!(
            "return entity:has_all('{public_name}', '{private_type}')"
        ))
        .eval::<bool>()
        .unwrap_err();
    assert_private_component_error(mixed_has_all_err, private_type);
}

#[test]
fn typed_setter_is_not_registered_for_private_components() {
    let (lua, _game_instance, _entity) = setup_entity_lua();
    let public_typed_setter = format!(
        "entity.set_{} ~= nil",
        to_snake_case(comp_type_name::<Transform>())
    );
    let private_typed_setter = format!(
        "entity.set_{} ~= nil",
        to_snake_case(comp_type_name::<PrefabInstanceRoot>())
    );
    let current_room_typed_setter = format!(
        "entity.set_{} ~= nil",
        to_snake_case(comp_type_name::<CurrentRoom>())
    );

    let has_public_typed_setter = lua
        .load(format!("return {public_typed_setter}"))
        .eval::<bool>()
        .unwrap();
    assert!(has_public_typed_setter);

    let has_private_typed_setter = lua
        .load(format!("return {private_typed_setter}"))
        .eval::<bool>()
        .unwrap();
    assert!(!has_private_typed_setter);

    let has_current_room_typed_setter = lua
        .load(format!("return {current_room_typed_setter}"))
        .eval::<bool>()
        .unwrap();
    assert!(!has_current_room_typed_setter);
}

#[test]
fn move_to_room_records_restore_location_and_remove_from_room_still_queues_command() {
    let (lua, _game_instance, _entity) = setup_entity_lua();

    lua.load("entity:move_to_room(2); entity:remove_from_room()")
        .exec()
        .unwrap();

    let recorded = take_pending_world_transition().expect("transition should be recorded");
    assert!(matches!(
        recorded.destination,
        DestinationSelector::RestoreLocation {
            world_id,
            room_id,
            layer,
            x,
            y,
        } if world_id == WorldId(1)
            && room_id == RoomId(2)
            && layer == RoomLayer::Front
            && x == 0.0
            && y == 0.0
    ));

    let commands: Vec<Box<dyn LuaCommand>> = drain_commands().collect();
    assert_eq!(commands.len(), 1);
}

#[test]
fn move_to_entry_when_called_records_entry_target_request() {
    let (lua, _game_instance, _entity) = setup_entity_lua();
    let entries = lua.create_table().unwrap();
    let arcade = lua.create_table().unwrap();
    let from_main = lua.create_table().unwrap();
    from_main.set("WorldId", 2).unwrap();
    from_main.set("RoomId", 20).unwrap();
    from_main.set("Layer", "Back").unwrap();
    from_main.set("EntryName", "FromMain").unwrap();
    arcade.set("FromMain", from_main).unwrap();
    entries.set("Arcade", arcade).unwrap();
    lua.globals().set("Entries", entries).unwrap();

    lua.load(format!(
        "entity:{}(Entries.Arcade.FromMain)",
        lua_entity::MOVE_TO_ENTRY
    ))
    .exec()
    .unwrap();

    let recorded = take_pending_world_transition().expect("transition should be recorded");
    assert!(matches!(
        recorded.destination,
        DestinationSelector::Entry(handle)
            if handle.world_id == WorldId(2)
                && handle.room_id == RoomId(20)
                && handle.layer == RoomLayer::Back
                && handle.entry_name == "FromMain"
    ));
}

#[test]
fn lua_entity_current_layer_and_move_to_layer_follow_room_layer_rules() {
    let (lua, game_instance, entity) = setup_entity_lua();

    assert_eq!(
        lua.load("return entity:current_layer()")
            .eval::<String>()
            .unwrap(),
        "Front"
    );

    lua.load("entity:move_to_layer('Back')").exec().unwrap();

    assert_eq!(
        lua.load("return entity:current_layer()")
            .eval::<String>()
            .unwrap(),
        "Back"
    );
    assert_eq!(
        game_instance.borrow().game.ecs.get::<CurrentRoom>(entity).map(|room| room.layer),
        Some(RoomLayer::Back)
    );
}

#[test]
fn move_to_world_records_pending_transition() {
    const DEST_WORLD: &str = "Arcade";
    const DEST_ENTRY: &str = "FromMain";

    let (lua, _game_instance, _entity) = setup_entity_lua();

    lua.load(format!(
        "entity:{}(\"{DEST_WORLD}\", \"{DEST_ENTRY}\")",
        lua_entity::MOVE_TO_WORLD
    ))
    .exec()
    .unwrap();

    let recorded = take_pending_world_transition().expect("transition should be recorded");
    assert!(matches!(
        &recorded.destination,
        DestinationSelector::World {
            selector: WorldSelector::ByName(name),
            entry_name: Some(entry_name),
        } if name == DEST_WORLD && entry_name == DEST_ENTRY
    ));
}

#[test]
fn trigger_world_exit_records_transition_by_id() {
    const DEST: WorldId = WorldId(1);
    let (lua, game_instance, entity) = setup_entity_lua();
    {
        let mut gi = game_instance.borrow_mut();
        gi.game.ecs.add_component_to_entity(
            entity,
            WorldExit {
                destination: Some(ExitDestination::World(DEST)),
                entry: None,
                trigger: WorldExitTrigger::OnInteract,
            },
        );
    }

    lua.load(format!("entity:{}()", lua_entity::TRIGGER_WORLD_EXIT))
        .exec()
        .unwrap();

    let recorded = take_pending_world_transition().expect("a transition should be recorded");
    assert!(matches!(
        recorded.destination,
        DestinationSelector::World {
            selector: WorldSelector::ById(id),
            entry_name: None,
        } if id == DEST
    ));
}

#[test]
fn trigger_world_exit_transport_targets_the_player() {
    const DEST: WorldId = WorldId(1);
    let (lua, game_instance, entity) = setup_entity_lua();
    {
        let mut gi = game_instance.borrow_mut();
        gi.game.ecs.create_entity().with(Player).finish();
        gi.game.ecs.add_component_to_entity(
            entity,
            WorldExit {
                destination: Some(ExitDestination::World(DEST)),
                entry: None,
                trigger: WorldExitTrigger::OnInteract,
            },
        );
    }

    lua.load(format!("entity:{}()", lua_entity::TRIGGER_WORLD_EXIT))
        .exec()
        .unwrap();

    let recorded = take_pending_world_transition().expect("a transition should be recorded");
    let expected_player = game_instance.borrow().game.ecs.get_player_entity();
    assert_eq!(recorded.entity, expected_player);
}

#[test]
fn trigger_world_exit_without_component_records_nothing() {
    let (lua, _game_instance, _entity) = setup_entity_lua();
    // entity has NO WorldExit component.
    lua.load(format!("entity:{}()", lua_entity::TRIGGER_WORLD_EXIT))
        .exec()
        .unwrap();

    assert!(take_pending_world_transition().is_none());
}

#[test]
fn teleport_and_move_by_queue_commands() {
    let (lua, _game_instance, _entity) = setup_entity_lua();

    lua.load("entity:teleport({ x = 10, y = 20 }); entity:move_by({ x = 3, y = -2 })")
        .exec()
        .unwrap();

    let commands: Vec<Box<dyn LuaCommand>> = drain_commands().collect();
    assert_eq!(commands.len(), 2);
}

#[test]
fn teleport_and_move_by_reject_indexed_vec_tables() {
    let (lua, _game_instance, _entity) = setup_entity_lua();

    assert!(lua.load("entity:teleport({ 10, 20 })").exec().is_err());
    assert!(lua.load("entity:move_by({ 3, -2 })").exec().is_err());
}

fn setup_entity_with_looping_audio(stop_behavior: AudioStopBehavior) -> (Lua, Rc<RefCell<GameInstance>>, Entity) {
    let (lua, game_instance, entity) = setup_entity_lua();
    {
        let mut gi = game_instance.borrow_mut();
        gi.game.ecs.insert_component(
            entity,
            AudioSource {
                groups: HashMap::from([(
                    SoundGroupId::Custom("Ambience".to_string()),
                    AudioGroup {
                        sounds: vec![SoundId(1)],
                        looping: true,
                        stop_behavior,
                        ..Default::default()
                    },
                )]),
                ..Default::default()
            },
        );
    }
    (lua, game_instance, entity)
}

#[test]
fn entity_stop_sound_without_opts_uses_authored_stop_behavior() {
    let (lua, _game_instance, entity) =
        setup_entity_with_looping_audio(AudioStopBehavior::FadeOut { duration: 0.75 });

    lua.load("entity:stop_sound()").exec().unwrap();
    let commands = drain_audio_commands();

    match &commands[0] {
        AudioCommand::StopLoops { owner, fade_out } => {
            assert_eq!(owner, &AudioPlaybackOwner::Entity(entity));
            assert_eq!(*fade_out, Some(0.75));
        }
        _other => panic!("unexpected command: expected StopLoops"),
    }
}

#[test]
fn entity_stop_sound_with_override_can_force_immediate_stop() {
    let (lua, _game_instance, _entity) =
        setup_entity_with_looping_audio(AudioStopBehavior::FadeOut { duration: 0.75 });

    lua.load("entity:stop_sound({ immediate = true })").exec().unwrap();
    let commands = drain_audio_commands();

    match &commands[0] {
        AudioCommand::StopLoops { fade_out, .. } => assert_eq!(*fade_out, None),
        other => panic!("unexpected command: expected StopLoops, got {:?}", std::mem::discriminant(other)),
    }
}
