use crate::engine::GameInstance;
use crate::game_global::drain_commands;
use crate::save_system::SaveProviderRegistry;
use crate::scripting::lua_ctx::{LuaGameCtx, LuaSaveCtx, register_save_lua_context};
use crate::scripting::modules::save_module::SaveModule;
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::{lua_engine, lua_fields, lua_save};
use engine_core::scripting::modules::lua_module::LuaApiWriter;
use mlua::Lua;
use std::cell::{Cell, RefCell};
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
        pending_quit_to_title: Rc::new(Cell::new(false)),
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
        format!(
            r#"
        engine.{}.{}()
        engine.{}.{}()
        engine.{}.{}()
        engine.{}.{}()
        "#,
            lua_save::SAVE, lua_save::MANUAL,
            lua_save::SAVE, lua_save::AUTO,
            lua_save::SAVE, lua_save::CHECKPOINT,
            lua_save::SAVE, lua_save::LOAD_LATEST,
        ),
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
            format!(
                r#"
            engine.{}.{}{{
              ["{}"] = "game.flags",
              ["{}"] = 1,
              ["{}"] = function(_) end,
            }}
            "#,
                lua_save::SAVE, lua_save::REGISTER_PROVIDER,
                lua_fields::ID,
                lua_save::PROVIDER_VERSION,
                lua_save::PROVIDER_APPLY,
            ),
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
            format!(
                r#"
            engine.{}.{}{{
              ["{}"] = "game.flags",
              ["{}"] = function() return "{{}}" end,
              ["{}"] = function(_) end,
            }}
            "#,
                lua_save::SAVE, lua_save::REGISTER_PROVIDER,
                lua_fields::ID,
                lua_save::PROVIDER_CAPTURE,
                lua_save::PROVIDER_APPLY,
            ),
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
            format!(
                r#"
            engine.{}.{}{{
              ["{}"] = "game.flags",
              ["{}"] = 1,
              ["{}"] = function() return "{{}}" end,
            }}
            "#,
                lua_save::SAVE, lua_save::REGISTER_PROVIDER,
                lua_fields::ID,
                lua_save::PROVIDER_VERSION,
                lua_save::PROVIDER_CAPTURE,
            ),
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
        format!(
            r#"
        engine.{}.{}{{
          ["{}"] = "game.flags",
          ["{}"] = 1,
          ["{}"] = function() return "{{}}" end,
          ["{}"] = function(_) end,
        }}
        "#,
            lua_save::SAVE, lua_save::REGISTER_PROVIDER,
            lua_fields::ID,
            lua_save::PROVIDER_VERSION,
            lua_save::PROVIDER_CAPTURE,
            lua_save::PROVIDER_APPLY,
        ),
    )
    .exec()
    .unwrap();

    // Second registration with the same id fails.
    let err = lua
        .load(
            format!(
                r#"
            engine.{}.{}{{
              ["{}"] = "game.flags",
              ["{}"] = 2,
              ["{}"] = function() return "{{}}" end,
              ["{}"] = function(_) end,
            }}
            "#,
                lua_save::SAVE, lua_save::REGISTER_PROVIDER,
                lua_fields::ID,
                lua_save::PROVIDER_VERSION,
                lua_save::PROVIDER_CAPTURE,
                lua_save::PROVIDER_APPLY,
            ),
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
    SaveModule.emit_api(&mut out);

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
fn save_provider_registration_only_requires_save_context() {
    // This test proves that registering a save provider from Lua only needs the
    // save context (LuaSaveCtx), not the full game runtime context.
    let lua = Lua::new();
    lua.globals()
        .set(lua_engine::ENGINE, lua.create_table().unwrap())
        .unwrap();

    let save_providers = Rc::new(RefCell::new(SaveProviderRegistry::new()));
    register_save_lua_context(&lua, save_providers.clone(), Rc::new(Cell::new(false))).unwrap();

    SaveModule.register(&lua).unwrap();

    lua.load(
        format!(
            r#"
        engine.{}.{}{{
          ["{}"] = "game.test_only_save_ctx",
          ["{}"] = 1,
          ["{}"] = function() return "{{}}" end,
          ["{}"] = function(_) end,
        }}
        "#,
            lua_save::SAVE, lua_save::REGISTER_PROVIDER,
            lua_fields::ID,
            lua_save::PROVIDER_VERSION,
            lua_save::PROVIDER_CAPTURE,
            lua_save::PROVIDER_APPLY,
        ),
    )
    .exec()
    .unwrap();

    assert_eq!(save_providers.borrow().iter().count(), 1);
}

#[test]
fn script_can_register_provider_and_invoke_manual_and_load_latest() {
    let (lua, _game_instance) = setup_save_lua();
    SaveModule.register(&lua).unwrap();

    lua.load(
        format!(
            r#"
        engine.{}.{}{{
          ["{}"] = "game.test",
          ["{}"] = 1,
          ["{}"] = function() return "{{}}" end,
          ["{}"] = function(_) end,
        }}
        engine.{}.{}()
        engine.{}.{}()
        "#,
            lua_save::SAVE, lua_save::REGISTER_PROVIDER,
            lua_fields::ID,
            lua_save::PROVIDER_VERSION,
            lua_save::PROVIDER_CAPTURE,
            lua_save::PROVIDER_APPLY,
            lua_save::SAVE, lua_save::MANUAL,
            lua_save::SAVE, lua_save::LOAD_LATEST,
        ),
    )
    .exec()
    .unwrap();

    assert_eq!(drain_commands().count(), 2);
}

