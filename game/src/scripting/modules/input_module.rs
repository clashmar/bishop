use crate::game_global::*;
use crate::input::input_snapshot::InputSnapshot;
use engine_core::input::InputActionId;
use engine_core::{register_lua_api, register_lua_module};
use engine_core::scripting::lua_constants::{lua_engine, lua_files, lua_input};
use engine_core::scripting::modules::lua_module::{LuaApi, LuaApiWriter, LuaModule};
use mlua::prelude::LuaResult;
use mlua::Function;
use mlua::Lua;
use mlua::Table;
use std::collections::HashMap;

pub const INPUT_IS_DOWN: &str = "is_down";
pub const INPUT_PRESSED: &str = "pressed";
pub const INPUT_RELEASED: &str = "released";
pub const INPUT_TAKE_CONTROL: &str = "take_control";
pub const INPUT_RELEASE_CONTROL: &str = "release_control";
pub const INPUT_IN_CONTROL: &str = "in_control";

/// Lua module that exposes the current input snapshot.
#[derive(Default, Clone)]
pub struct InputModule;
register_lua_module!(InputModule);

impl LuaModule for InputModule {
    fn register(&self, lua: &Lua) -> LuaResult<()> {
        let is_down_fn = make_snapshot_query_fn(lua, |snap| &snap.down)?;
        let pressed_fn = make_snapshot_query_fn(lua, |snap| &snap.pressed)?;
        let released_fn = make_snapshot_query_fn(lua, |snap| &snap.released)?;
        let is_action_down_fn = make_action_query_fn(lua, InputSnapshot::is_action_down)?;
        let is_action_pressed_fn = make_action_query_fn(lua, InputSnapshot::is_action_pressed)?;
        let is_action_released_fn = make_action_query_fn(lua, InputSnapshot::is_action_released)?;

        let take_control_fn = lua.create_function(|_, (name, priority): (String, u8)| {
            take_input_control(&name, priority);
            Ok(())
        })?;

        let release_control_fn = lua.create_function(|_, name: String| {
            release_input_control(&name);
            Ok(())
        })?;

        let in_control_fn = lua.create_function(|_, name: String| Ok(in_input_control(&name)))?;

        let engine_tbl: Table = lua.globals().get(lua_engine::ENGINE)?;
        let input_tbl = lua.create_table()?;
        input_tbl.set(INPUT_IS_DOWN, is_down_fn)?;
        input_tbl.set(INPUT_PRESSED, pressed_fn)?;
        input_tbl.set(INPUT_RELEASED, released_fn)?;
        input_tbl.set(lua_input::IS_ACTION_DOWN, is_action_down_fn)?;
        input_tbl.set(lua_input::IS_ACTION_PRESSED, is_action_pressed_fn)?;
        input_tbl.set(lua_input::IS_ACTION_RELEASED, is_action_released_fn)?;
        input_tbl.set(INPUT_TAKE_CONTROL, take_control_fn)?;
        input_tbl.set(INPUT_RELEASE_CONTROL, release_control_fn)?;
        input_tbl.set(INPUT_IN_CONTROL, in_control_fn)?;

        engine_tbl.set(lua_engine::INPUT, input_tbl)?;

        Ok(())
    }
}

/// Builds a Lua function that queries a raw input value from the current snapshot.
pub fn make_snapshot_query_fn<Sel>(lua: &Lua, map_selector: Sel) -> LuaResult<Function>
where
    Sel: Fn(&InputSnapshot) -> &HashMap<&'static str, bool> + Copy + Send + 'static,
{
    lua.create_function(move |_, key: String| {
        let snapshot = get_input_snapshot();
        let value = map_selector(&snapshot)
            .get(key.as_str())
            .copied()
            .unwrap_or(false);
        Ok(value)
    })
}

fn make_action_query_fn<Sel>(lua: &Lua, selector: Sel) -> LuaResult<Function>
where
    Sel: Fn(&InputSnapshot, &InputActionId) -> bool + Copy + Send + 'static,
{
    lua.create_function(move |_, action: String| {
        let snapshot = get_input_snapshot();
        Ok(selector(&snapshot, &InputActionId::new(action)))
    })
}

register_lua_api!(InputModule, lua_files::ENGINE);

impl LuaApi for InputModule {
    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("---@param input string");
        out.line("---@return boolean");
        out.line(&format!(
            "function {}.{}.{}(input) end",
            lua_engine::ENGINE,
            lua_engine::INPUT,
            INPUT_IS_DOWN
        ));
        out.line("");

        out.line("---@param input string");
        out.line("---@return boolean");
        out.line(&format!(
            "function {}.{}.{}(input) end",
            lua_engine::ENGINE,
            lua_engine::INPUT,
            INPUT_PRESSED
        ));
        out.line("");

        out.line("---@param input string");
        out.line("---@return boolean");
        out.line(&format!(
            "function {}.{}.{}(input) end",
            lua_engine::ENGINE,
            lua_engine::INPUT,
            INPUT_RELEASED
        ));
        out.line("");

        for method in [
            lua_input::IS_ACTION_DOWN,
            lua_input::IS_ACTION_PRESSED,
            lua_input::IS_ACTION_RELEASED,
        ] {
            out.line("---@param action string");
            out.line("---@return boolean");
            out.line(&format!(
                "function {}.{}.{}(action) end",
                lua_engine::ENGINE,
                lua_engine::INPUT,
                method
            ));
            out.line("");
        }

        out.line("---@param name string");
        out.line("---@param priority number");
        out.line("---@return nil");
        out.line(&format!(
            "function {}.{}.{}(name, priority) end",
            lua_engine::ENGINE,
            lua_engine::INPUT,
            INPUT_TAKE_CONTROL
        ));
        out.line("");

        out.line("---@param name string");
        out.line("---@return nil");
        out.line(&format!(
            "function {}.{}.{}(name) end",
            lua_engine::ENGINE,
            lua_engine::INPUT,
            INPUT_RELEASE_CONTROL
        ));
        out.line("");

        out.line("---@param name string");
        out.line("---@return boolean");
        out.line(&format!(
            "function {}.{}.{}(name) end",
            lua_engine::ENGINE,
            lua_engine::INPUT,
            INPUT_IN_CONTROL
        ));
        out.line("");
    }
}
