use crate::engine::GameInstance;
use crate::game_global::drain_commands;
use crate::save_system::SaveProviderRegistry;
use crate::scripting::lua_ctx::{LuaGameCtx, LuaSaveCtx};
use crate::scripting::modules::save_module::SaveModule;
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::{lua_engine, lua_save};
use engine_core::scripting::modules::lua_module::LuaApiWriter;
use mlua::Lua;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn setup_save_lua() -> (Lua, Rc<RefCell<GameInstance>>) {
    let _ = drain_commands().count();
    let lua = Lua::new();
    lua.globals()
        .set(lua_engine::ENGINE, lua.create_table().unwrap())
        .unwrap();

    let save_providers = Rc::new(RefCell::new(SaveProviderRegistry::new()));
    let mut game = Game::default();
    game.add_world(World::default());
    let game_instance = Rc::new(RefCell::new(GameInstance {
        game,
        prev_positions: HashMap::new(),
    }));

    LuaGameCtx {
        game_instance: game_instance.clone(),
    }
    .set_lua_ctx(&lua)
    .unwrap();
    LuaSaveCtx {
        save_providers,
    }
    .set_lua_ctx(&lua)
    .unwrap();

    (lua, game_instance)
}

#[test]
fn save_helpers_enqueue_four_commands() {
    let (lua, _game_instance) = setup_save_lua();
    SaveModule.register(&lua).unwrap();

    lua.load(
        r#"
        engine.save.manual()
        engine.save.auto()
        engine.save.checkpoint()
        engine.save.load_latest()
        "#,
    )
    .exec()
    .unwrap();

    assert_eq!(drain_commands().count(), 4);
}

#[test]
fn register_provider_rejects_missing_capture_function() {
    let (lua, _game_instance) = setup_save_lua();
    SaveModule.register(&lua).unwrap();

    let err = lua
        .load(
            r#"
            engine.save.register_provider({
              id = "game.flags",
              version = 1,
              apply = function(_) end,
            })
            "#,
        )
        .exec()
        .unwrap_err();

    assert!(
        err.to_string().contains("capture"),
        "expected error to mention 'capture', got: {}",
        err
    );
}

#[test]
fn register_provider_rejects_missing_version_field() {
    let (lua, _game_instance) = setup_save_lua();
    SaveModule.register(&lua).unwrap();

    let err = lua
        .load(
            r#"
            engine.save.register_provider({
              id = "game.flags",
              capture = function() return "{}" end,
              apply = function(_) end,
            })
            "#,
        )
        .exec()
        .unwrap_err();

    assert!(
        err.to_string().contains("version"),
        "expected error to mention 'version', got: {}",
        err
    );
}

#[test]
fn register_provider_rejects_missing_apply_function() {
    let (lua, _game_instance) = setup_save_lua();
    SaveModule.register(&lua).unwrap();

    let err = lua
        .load(
            r#"
            engine.save.register_provider({
              id = "game.flags",
              version = 1,
              capture = function() return "{}" end,
            })
            "#,
        )
        .exec()
        .unwrap_err();

    assert!(
        err.to_string().contains("apply"),
        "expected error to mention 'apply', got: {}",
        err
    );
}

#[test]
fn register_provider_rejects_duplicate_id() {
    let (lua, _game_instance) = setup_save_lua();
    SaveModule.register(&lua).unwrap();

    // First registration succeeds.
    lua.load(
        r#"
        engine.save.register_provider({
          id = "game.flags",
          version = 1,
          capture = function() return "{}" end,
          apply = function(_) end,
        })
        "#,
    )
    .exec()
    .unwrap();

    // Second registration with the same id fails.
    let err = lua
        .load(
            r#"
            engine.save.register_provider({
              id = "game.flags",
              version = 2,
              capture = function() return "{}" end,
              apply = function(_) end,
            })
            "#,
        )
        .exec()
        .unwrap_err();

    assert!(
        err.to_string().contains("game.flags"),
        "expected error to mention provider id 'game.flags', got: {}",
        err
    );
}

#[test]
fn save_module_api_emits_all_public_helpers() {
    let mut out = LuaApiWriter::default();
    SaveModule::default().emit_api(&mut out);

    let engine_save = format!("engine.{} = {{}}", lua_save::SAVE);
    let manual = format!("function engine.{}.{}() end", lua_save::SAVE, lua_save::MANUAL);
    let auto = format!("function engine.{}.{}() end", lua_save::SAVE, lua_save::AUTO);
    let checkpoint = format!("function engine.{}.{}() end", lua_save::SAVE, lua_save::CHECKPOINT);
    let load_latest = format!("function engine.{}.{}() end", lua_save::SAVE, lua_save::LOAD_LATEST);
    let register = format!("function engine.{}.{}(def) end", lua_save::SAVE, lua_save::REGISTER_PROVIDER);

    assert!(out.buf.contains(&engine_save), "missing: {}", engine_save);
    assert!(out.buf.contains(&manual), "missing: {}", manual);
    assert!(out.buf.contains(&auto), "missing: {}", auto);
    assert!(out.buf.contains(&checkpoint), "missing: {}", checkpoint);
    assert!(out.buf.contains(&load_latest), "missing: {}", load_latest);
    assert!(out.buf.contains(&register), "missing: {}", register);
}

#[test]
fn script_can_register_provider_and_invoke_manual_and_load_latest() {
    let (lua, _game_instance) = setup_save_lua();
    SaveModule.register(&lua).unwrap();

    lua.load(
        r#"
        engine.save.register_provider({
          id = "game.test",
          version = 1,
          capture = function() return "{}" end,
          apply = function(_) end,
        })
        engine.save.manual()
        engine.save.load_latest()
        "#,
    )
    .exec()
    .unwrap();

    assert_eq!(drain_commands().count(), 2);
}
