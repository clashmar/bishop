use crate::engine::GameInstance;
use crate::game_global::take_pending_world_transition;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::transitions::world_transitions::DestinationSelector;
use crate::scripting::modules::engine_module::EngineModule;
use engine_core::ecs::{Entity, Player, Transform};
use engine_core::game::Game;
use engine_core::scripting::lua_constants::{lua_engine, lua_fields};
use engine_core::scripting::modules::lua_module::{LuaApi, LuaApiWriter};
use engine_core::scripting::LuaModule;
use engine_core::worlds::{RoomId, World, WorldId, WorldTransitionMode};
use mlua::Lua;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[test]
fn overlay_entry_when_called_records_entry_target_request() {
    let (lua, _game_instance, _player) = setup_engine_lua();

    let result = lua
        .load(format!(
            r#"
            engine.{}({{
              WorldId = 2,
              RoomId = 9,
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
            x,
            y,
        } if world_id == WorldId(2)
            && room_id == RoomId(9)
            && x == 3.5
            && y == -4.5
    ));
}

#[test]
fn emit_api_when_called_documents_restore_location_payload() {
    let mut out = LuaApiWriter::default();
    EngineModule.emit_api(&mut out);

    assert!(out.buf.contains(&format!("function engine.{}(entry) end", lua_engine::OVERLAY_ENTRY)));
    assert!(out.buf.contains("---@class RestoreLocation"));
    assert!(out.buf.contains("---@param location RestoreLocation"));
    assert!(out.buf.contains(&format!(
        "function engine.{}(location) end",
        lua_engine::RESTORE_LOCATION
    )));
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
