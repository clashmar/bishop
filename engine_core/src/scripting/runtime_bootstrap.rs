use crate::ecs::components::text::TomlId;
use crate::scripting::event_bus::EventBus;
use crate::scripting::lua_constants::{lua_engine, lua_globals};
use mlua::prelude::LuaResult;
use mlua::{Lua, Table, Value, Variadic};

/// Registers the shared runtime Lua globals used by both editor and game.
pub fn register_runtime_modules(lua: &Lua, event_bus: &EventBus) -> LuaResult<()> {
    register_engine_module(lua)?;
    register_engine_asset_helpers(lua)?;
    register_engine_event_helpers(lua)?;
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

#[cfg(test)]
mod tests {
    use super::*;

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
            .call::<()>(("editor:test".to_string(), mlua::MultiValue::from_vec(vec![Value::Integer(7)])))
            .unwrap();

        assert_eq!(lua.globals().get::<i64>("engine_value").unwrap(), 7);
    }
}
