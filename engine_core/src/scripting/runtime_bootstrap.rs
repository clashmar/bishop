use crate::ecs::components::text::TomlId;
use crate::scripting::color_bootstrap::register_color_helpers;
use crate::scripting::event_bus::EventBus;
use crate::scripting::lua_constants::{lua_engine, lua_globals, lua_save};
use crate::scripting::theme_bootstrap::register_theme_helpers;
use mlua::prelude::LuaResult;
use mlua::{Lua, Table, Value, Variadic};

/// Registers the shared runtime Lua globals used by both editor and game.
pub fn register_runtime_modules(lua: &Lua, event_bus: &EventBus) -> LuaResult<()> {
    register_engine_module(lua)?;
    register_engine_asset_helpers(lua)?;
    register_engine_event_helpers(lua)?;
    register_engine_save_helpers(lua)?;
    register_color_helpers(lua)?;
    register_theme_helpers(lua)?;
    lua.globals()
        .set(lua_globals::LUA_EVENT_BUS, event_bus.clone())?;
    Ok(())
}

/// Creates the global `engine` module table.
pub fn register_engine_module(lua: &Lua) -> LuaResult<()> {
    let engine_mod = lua.create_table()?;
    lua.globals().set(lua_engine::ENGINE, engine_mod.clone())?;
    lua.register_module(lua_engine::ENGINE, &engine_mod)?;
    Ok(())
}

fn register_engine_asset_helpers(lua: &Lua) -> LuaResult<()> {
    let engine_tbl: Table = lua.globals().get(lua_engine::ENGINE)?;
    let asset_tbl = match engine_tbl.get::<Option<Table>>(lua_engine::ASSET)? {
        Some(table) => table,
        None => {
            let table = lua.create_table()?;
            engine_tbl.set(lua_engine::ASSET, table.clone())?;
            table
        }
    };

    let toml_fn = lua.create_function(|_lua, args: Variadic<Value>| {
        if !args.is_empty() {
            return Err(mlua::Error::RuntimeError(format!(
                "wrong number of arguments: expected 0, got {}",
                args.len()
            )));
        }
        Ok(TomlId(0))
    })?;
    asset_tbl.set(lua_engine::TOML, toml_fn)?;
    Ok(())
}

fn register_engine_event_helpers(lua: &Lua) -> LuaResult<()> {
    let engine_tbl: Table = lua.globals().get(lua_engine::ENGINE)?;

    let on_fn = lua.create_function(|lua, (event, handler): (String, mlua::Function)| {
        let ud: mlua::AnyUserData = lua.globals().get(lua_globals::LUA_EVENT_BUS)?;
        let bus = ud.borrow::<EventBus>()?;
        bus.on(event, handler);
        Ok(())
    })?;
    engine_tbl.set(lua_engine::ON, on_fn)?;

    let emit_fn = lua.create_function(|lua, (event, args): (String, Variadic<Value>)| {
        let ud: mlua::AnyUserData = lua.globals().get(lua_globals::LUA_EVENT_BUS)?;
        let bus = ud.borrow::<EventBus>()?;
        bus.emit(event, args);
        Ok(())
    })?;
    engine_tbl.set(lua_engine::EMIT, emit_fn)?;
    Ok(())
}

fn register_engine_save_helpers(lua: &Lua) -> LuaResult<()> {
    let engine_tbl: Table = lua.globals().get(lua_engine::ENGINE)?;
    let save_tbl = lua.create_table()?;
    let triggers_tbl = lua.create_table()?;

    triggers_tbl.set(lua_save::MANUAL, lua_save::MANUAL)?;
    triggers_tbl.set(lua_save::AUTO, lua_save::AUTO)?;
    triggers_tbl.set(lua_save::CHECKPOINT, lua_save::CHECKPOINT)?;
    save_tbl.set(lua_save::TRIGGERS, triggers_tbl)?;

    save_tbl.set(
        lua_save::REQUEST,
        lua.create_function(|_, _def: Table| Ok(()))?,
    )?;
    save_tbl.set(lua_save::MANUAL, lua.create_function(|_, ()| Ok(()))?)?;
    save_tbl.set(lua_save::AUTO, lua.create_function(|_, ()| Ok(()))?)?;
    save_tbl.set(
        lua_save::CHECKPOINT,
        lua.create_function(|_, ()| Ok(()))?,
    )?;
    save_tbl.set(
        lua_save::LOAD_LATEST,
        lua.create_function(|_, ()| Ok(()))?,
    )?;
    save_tbl.set(
        lua_save::REGISTER_PROVIDER,
        lua.create_function(|_, _def: Table| Ok(()))?,
    )?;
    save_tbl.set(
        lua_save::TO_STRING,
        lua.create_function(|_, _value: Value| Ok(String::new()))?,
    )?;
    save_tbl.set(
        lua_save::FROM_STRING,
        lua.create_function(|_, _json: String| Ok(Value::Nil))?,
    )?;
    save_tbl.set(
        lua_save::HAS_LATEST,
        lua.create_function(|_, ()| Ok(false))?,
    )?;

    engine_tbl.set(lua_save::SAVE, save_tbl)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::lua_constants::lua_save;

    #[test]
    fn register_runtime_modules_sets_shared_engine_helpers() {
        let lua = Lua::new();
        let event_bus = EventBus::default();

        register_runtime_modules(&lua, &event_bus).unwrap();

        let globals = lua.globals();
        let engine_tbl: Table = globals.get(lua_engine::ENGINE).unwrap();
        let asset_tbl: Table = engine_tbl.get(lua_engine::ASSET).unwrap();

        asset_tbl
            .get::<mlua::Function>(lua_engine::TOML)
            .unwrap()
            .call::<TomlId>(())
            .unwrap();
        engine_tbl.get::<mlua::Function>(lua_engine::ON).unwrap();
        engine_tbl.get::<mlua::Function>(lua_engine::EMIT).unwrap();
        globals
            .get::<mlua::AnyUserData>(lua_globals::LUA_EVENT_BUS)
            .unwrap();
    }

    #[test]
    fn register_runtime_modules_when_editor_script_reads_save_api_loads_script() {
        let lua = Lua::new();
        let event_bus = EventBus::default();

        register_runtime_modules(&lua, &event_bus).unwrap();

        let script = format!(
            r#"
            local checkpoint = engine.{}.{}.{}

            engine.{}.{}()
            engine.{}.{}()
            engine.{}.{}()
            engine.{}.{}({{
                id = "editor.test",
                version = 1,
                capture = function() return "{{}}" end,
                apply = function(_) end,
            }})

            return {{
                trigger = checkpoint,
            }}
        "#,
            lua_save::SAVE,
            lua_save::TRIGGERS,
            lua_save::CHECKPOINT,
            lua_save::SAVE,
            lua_save::MANUAL,
            lua_save::SAVE,
            lua_save::AUTO,
            lua_save::SAVE,
            lua_save::CHECKPOINT,
            lua_save::SAVE,
            lua_save::REGISTER_PROVIDER,
        );

        let script_table: Table = lua.load(script).eval().unwrap();

        assert_eq!(
            script_table.get::<String>(lua_save::TRIGGER).unwrap(),
            lua_save::CHECKPOINT
        );
    }

    #[test]
    fn register_runtime_modules_supports_editor_script_load_patterns() {
        let lua = Lua::new();
        let event_bus = EventBus::default();

        register_runtime_modules(&lua, &event_bus).unwrap();

        let script = r#"
            engine.on("editor:test", function(value)
                engine_value = value
            end)

            return {
                public = {
                    dialogue = engine.asset.toml(),
                }
            }
        "#;

        let script_table: Table = lua.load(script).eval().unwrap();
        let public: Table = script_table.get("public").unwrap();
        assert_eq!(public.get::<TomlId>("dialogue").unwrap(), TomlId(0));

        let engine_tbl: Table = lua.globals().get(lua_engine::ENGINE).unwrap();
        engine_tbl
            .get::<mlua::Function>(lua_engine::EMIT)
            .unwrap()
            .call::<()>((
                "editor:test".to_string(),
                mlua::MultiValue::from_vec(vec![Value::Integer(7)]),
            ))
            .unwrap();

        assert_eq!(lua.globals().get::<i64>("engine_value").unwrap(), 7);
    }
}
