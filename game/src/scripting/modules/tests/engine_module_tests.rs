use crate::engine::GameInstance;
use crate::game_global::take_pending_world_transition;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::engine_module::EngineModule;
use crate::transitions::world_transitions::DestinationSelector;
use engine_core::ecs::{Entity, Global, Name, Player, Script, ScriptId, Transform};
use engine_core::game::Game;
use engine_core::scripting::lua_constants::{lua_engine, lua_events, lua_fields};
use engine_core::scripting::modules::lua_module::{LuaApi, LuaApiWriter};
use engine_core::scripting::LuaModule;
use engine_core::worlds::{RoomId, RoomLayer, World, WorldId, WorldTransitionMode};
use mlua::{Function, Lua, Table};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[test]
fn call_when_name_is_room_local_invokes_only_the_active_room_script() -> mlua::Result<()> {
    let (lua, game_instance, _player) = setup_engine_lua();
    let active = named_script(&lua, &game_instance, RoomId(1), TEST_ENTITY_NAME, false)?;
    let other = named_script(&lua, &game_instance, RoomId(2), TEST_ENTITY_NAME, false)?;

    lua.load(format!(
        "engine.{}({:?}, {:?}, 4)",
        lua_engine::CALL,
        TEST_ENTITY_NAME,
        SCRIPT_METHOD,
    ))
    .exec()?;

    assert_eq!(active.get::<i64>("calls")?, 4);
    assert_eq!(other.get::<i64>("calls")?, 0);
    Ok(())
}

#[test]
fn call_when_only_global_exists_and_active_room_is_absent_invokes_global() -> mlua::Result<()> {
    let (lua, game_instance, _player) = setup_engine_lua();
    let global = named_script(&lua, &game_instance, RoomId(1), TEST_ENTITY_NAME, true)?;
    {
        let mut instance = game_instance.borrow_mut();
        let world = instance.game.current_world_mut().ok_or_else(|| {
            mlua::Error::RuntimeError("missing active test world".to_string())
        })?;
        world.current_room_id = None;
    }

    lua.load(format!(
        "engine.{}({:?}, {:?}, 2)",
        lua_engine::CALL,
        TEST_ENTITY_NAME,
        SCRIPT_METHOD,
    ))
    .exec()?;

    assert_eq!(global.get::<i64>("calls")?, 2);
    Ok(())
}

#[test]
fn call_when_name_exists_only_in_another_room_reports_not_found() -> mlua::Result<()> {
    let (lua, game_instance, _player) = setup_engine_lua();
    let other = named_script(&lua, &game_instance, RoomId(2), TEST_ENTITY_NAME, false)?;

    let error = lua
        .load(format!(
            "engine.{}({:?}, {:?}, 4)",
            lua_engine::CALL,
            TEST_ENTITY_NAME,
            SCRIPT_METHOD,
        ))
        .exec()
        .err()
        .ok_or_else(|| mlua::Error::RuntimeError("expected a missing target".to_string()))?;

    assert!(error.to_string().contains("not found"), "{error}");
    assert_eq!(other.get::<i64>("calls")?, 0);
    Ok(())
}

#[test]
fn call_when_two_active_room_entities_share_name_reports_ambiguity() -> mlua::Result<()> {
    let (lua, game_instance, _player) = setup_engine_lua();
    let first = named_script(&lua, &game_instance, RoomId(1), TEST_ENTITY_NAME, false)?;
    let second = named_script(&lua, &game_instance, RoomId(1), TEST_ENTITY_NAME, false)?;

    let error = lua
        .load(format!(
            "engine.{}({:?}, {:?}, 4)",
            lua_engine::CALL,
            TEST_ENTITY_NAME,
            SCRIPT_METHOD,
        ))
        .exec()
        .err()
        .ok_or_else(|| mlua::Error::RuntimeError("expected an ambiguous target".to_string()))?;

    assert!(error.to_string().contains("ambiguous"), "{error}");
    assert_eq!(first.get::<i64>("calls")?, 0);
    assert_eq!(second.get::<i64>("calls")?, 0);
    Ok(())
}

#[test]
fn call_when_room_and_global_names_overlap_reports_ambiguity() -> mlua::Result<()> {
    let (lua, game_instance, _player) = setup_engine_lua();
    let local = named_script(&lua, &game_instance, RoomId(1), TEST_ENTITY_NAME, false)?;
    let global = named_script(&lua, &game_instance, RoomId(1), TEST_ENTITY_NAME, true)?;

    let error = lua
        .load(format!(
            "engine.{}({:?}, {:?}, 4)",
            lua_engine::CALL,
            TEST_ENTITY_NAME,
            SCRIPT_METHOD,
        ))
        .exec()
        .err()
        .ok_or_else(|| mlua::Error::RuntimeError("expected an ambiguous target".to_string()))?;

    assert!(error.to_string().contains("ambiguous"), "{error}");
    assert_eq!(local.get::<i64>("calls")?, 0);
    assert_eq!(global.get::<i64>("calls")?, 0);
    Ok(())
}

#[test]
fn kinematic_switch_interact_with_named_target_calls_without_requesting_a_room() -> mlua::Result<()> {
    let lua = Lua::new();
    let engine = lua.create_table()?;
    let called = lua.create_table()?;
    let recorded = called.clone();
    engine.set(
        lua_engine::CALL,
        lua.create_function(move |_, (name, method): (String, String)| {
            recorded.set("name", name)?;
            recorded.set("method", method)
        })?,
    )?;
    let log = lua.create_table()?;
    log.set("info", lua.create_function(|_, _: String| Ok(()))?)?;
    engine.set(lua_engine::LOG, log)?;
    lua.globals().set(lua_engine::ENGINE, engine)?;

    let switch = super::demo_save_flow_tests::load_demo_script(&lua, SWITCH_SCRIPT);
    let public: Table = switch.get("public")?;
    public.set("target_name", TEST_ENTITY_NAME)?;
    let interact: Function = switch.get("interact")?;

    interact.call::<()>(switch)?;

    assert_eq!(called.get::<String>("name")?, TEST_ENTITY_NAME);
    assert_eq!(called.get::<String>("method")?, SCRIPT_METHOD);
    Ok(())
}

#[test]
fn overlay_entry_when_called_records_entry_target_request() {
    let (lua, _game_instance, _player) = setup_engine_lua();

    let result = lua
        .load(format!(
            r#"
            engine.{}({{
              WorldId = 2,
              RoomId = 9,
              Layer = "Back",
              EntryName = "UnderworldStart",
              X = 3.5,
              Y = -4.5,
            }})
            "#,
            lua_engine::OVERLAY_ENTRY,
        ))
        .exec();

    assert!(result.is_ok(), "unexpected error: {result:?}");

    let recorded = take_pending_world_transition().expect("transition should be recorded");
    assert_eq!(recorded.mode, WorldTransitionMode::Overlay);
    assert!(matches!(
        recorded.destination,
        DestinationSelector::Entry(handle)
            if handle.world_id == WorldId(2)
                && handle.room_id == RoomId(9)
                && handle.layer == RoomLayer::Back
                && handle.entry_name == "UnderworldStart"
                && handle.x == Some(3.5)
                && handle.y == Some(-4.5)
    ));
}

#[test]
fn restore_location_when_called_records_restore_location_request() {
    let (lua, _game_instance, _player) = setup_engine_lua();

    let result = lua
        .load(format!(
            r#"
            engine.{}({{
              ["{}"] = 2,
              ["{}"] = 9,
              ["{}"] = 3.5,
              ["{}"] = -4.5,
            }})
            "#,
            lua_engine::RESTORE_LOCATION,
            lua_fields::WORLD_ID,
            lua_fields::ROOM_ID,
            lua_fields::X,
            lua_fields::Y,
        ))
        .exec();

    assert!(result.is_ok(), "unexpected error: {result:?}");

    let recorded = take_pending_world_transition().expect("transition should be recorded");
    assert_eq!(recorded.mode, WorldTransitionMode::Transport);
    assert!(matches!(
        recorded.destination,
        DestinationSelector::RestoreLocation {
            world_id,
            room_id,
            layer,
            x,
            y,
        } if world_id == WorldId(2)
            && room_id == RoomId(9)
            && layer == RoomLayer::Front
            && x == 3.5
            && y == -4.5
    ));
}

#[test]
fn emit_api_when_called_documents_restore_location_payload() {
    let mut out = LuaApiWriter::default();
    EngineModule.emit_api(&mut out);

    assert!(out.buf.contains(&format!(
        "function engine.{}(name, method, ...) end",
        lua_engine::CALL
    )));
    assert!(out.buf.contains(&format!("function engine.{}(entry) end", lua_engine::OVERLAY_ENTRY)));
    assert!(out.buf.contains("---@class RestoreLocation"));
    assert!(out.buf.contains("---@param location RestoreLocation"));
    assert!(out.buf.contains(&format!(
        "function engine.{}(location) end",
        lua_engine::RESTORE_LOCATION
    )));
    assert!(out.buf.contains(&format!(
        "engine.events.{} = \"{}\"",
        lua_events::COLLISION_ENTER_FIELD,
        lua_events::COLLISION_ENTER
    )));
    assert!(out.buf.contains(&format!(
        "engine.events.{} = \"{}\"",
        lua_events::COLLISION_EXIT_FIELD,
        lua_events::COLLISION_EXIT
    )));
    assert!(out.buf.contains(&format!(
        "engine.events.{} = \"{}\"",
        lua_events::COLLISION_SQUEEZE_FIELD,
        lua_events::COLLISION_SQUEEZE
    )));
}

#[test]
fn engine_module_register_when_loaded_exposes_sensor_event_constants() {
    let (lua, _game_instance, _player) = setup_engine_lua();
    let globals = lua.globals();
    let engine: mlua::Table = globals.get(lua_engine::ENGINE).unwrap();
    let events: mlua::Table = engine.get(lua_events::EVENTS).unwrap();

    assert_eq!(
        events.get::<String>(lua_events::SENSOR_ENTER_FIELD).unwrap(),
        lua_events::SENSOR_ENTER
    );
    assert_eq!(
        events.get::<String>(lua_events::SENSOR_STAY_FIELD).unwrap(),
        lua_events::SENSOR_STAY
    );
    assert_eq!(
        events.get::<String>(lua_events::SENSOR_EXIT_FIELD).unwrap(),
        lua_events::SENSOR_EXIT
    );
}

#[test]
fn engine_module_register_when_loaded_exposes_collision_event_constants() {
    let (lua, _game_instance, _player) = setup_engine_lua();
    let globals = lua.globals();
    let engine: mlua::Table = globals.get(lua_engine::ENGINE).unwrap();
    let events: mlua::Table = engine.get(lua_events::EVENTS).unwrap();

    assert_eq!(
        events.get::<String>(lua_events::COLLISION_ENTER_FIELD).unwrap(),
        lua_events::COLLISION_ENTER
    );
    assert_eq!(
        events.get::<String>(lua_events::COLLISION_EXIT_FIELD).unwrap(),
        lua_events::COLLISION_EXIT
    );
    assert_eq!(
        events.get::<String>(lua_events::COLLISION_SQUEEZE_FIELD).unwrap(),
        lua_events::COLLISION_SQUEEZE
    );
}

const SCRIPT_METHOD: &str = "toggle";
const SWITCH_SCRIPT: &str = "kinematic_switch.lua";
const TEST_ENTITY_NAME: &str = "Moving Platform";

fn named_script(
    lua: &Lua,
    game_instance: &Rc<RefCell<GameInstance>>,
    room: RoomId,
    name: &str,
    global: bool,
) -> mlua::Result<Table> {
    let script_id = ScriptId(41);
    let instance: Table = lua
        .load(format!(
            "return {{ calls = 0, {} = function(self, amount) self.calls = self.calls + amount end }}",
            SCRIPT_METHOD,
        ))
        .eval()?;
    let mut game_instance = game_instance.borrow_mut();
    let game = &mut game_instance.game;
    let entity = game
        .ecs
        .create_entity()
        .with(Name(name.to_string()))
        .with(Script {
            script_id,
            ..Default::default()
        })
        .with_current_room(room)
        .finish();
    if global {
        game.ecs.add_component_to_entity(entity, Global::default());
    }
    game.script_manager
        .instances
        .insert((entity, script_id), instance.clone());
    Ok(instance)
}

fn setup_engine_lua() -> (Lua, Rc<RefCell<GameInstance>>, Entity) {
    let lua = Lua::new();
    lua.globals()
        .set(lua_engine::ENGINE, lua.create_table().unwrap())
        .unwrap();

    let mut game = Game::default();
    let mut overworld = World::new(WorldId(1), "Overworld".to_string(), 16.0);
    overworld.current_room_id = Some(RoomId(1));
    let mut underworld = World::new(WorldId(2), "Underworld".to_string(), 16.0);
    underworld.current_room_id = Some(RoomId(2));
    game.add_world(overworld);
    game.add_world(underworld);
    game.select_world(WorldId(1));

    let player = game
        .ecs
        .create_entity()
        .with(Transform::default())
        .with(Player)
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

    EngineModule.register(&lua).unwrap();

    (lua, game_instance, player)
}
