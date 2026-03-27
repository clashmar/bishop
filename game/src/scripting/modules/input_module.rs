// game/src/scripting/modules/input_module.rs
use crate::game_global::*;
use crate::input::input_snapshot::InputSnapshot;
use crate::scripting::lua_ctx::LuaBishopCtx;
use engine_core::scripting::lua_constants::*;
use engine_core::scripting::modules::lua_module::*;
use engine_core::*;
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

        let take_control_fn = lua.create_function(|_, (name, priority): (String, u8)| {
            take_input_control(&name, priority);
            Ok(())
        })?;

        let release_control_fn = lua.create_function(|_, name: String| {
            release_input_control(&name);
            Ok(())
        })?;

        let in_control_fn = lua.create_function(|_, name: String| Ok(in_input_control(&name)))?;

        // Assemble the `engine.input` table
        let engine_tbl: Table = lua.globals().get(ENGINE)?;
        let input_tbl = lua.create_table()?;
        input_tbl.set(INPUT_IS_DOWN, is_down_fn)?;
        input_tbl.set(INPUT_PRESSED, pressed_fn)?;
        input_tbl.set(INPUT_RELEASED, released_fn)?;
        input_tbl.set(INPUT_TAKE_CONTROL, take_control_fn)?;
        input_tbl.set(INPUT_RELEASE_CONTROL, release_control_fn)?;
        input_tbl.set(INPUT_IN_CONTROL, in_control_fn)?;

        // Attach the sub‑module to the already existing global `engine` table
        engine_tbl.set(INPUT, input_tbl)?;

        Ok(())
    }
}

/// Build a Lua function that queries a current `InputSnapshot`.
pub fn make_snapshot_query_fn<Sel>(lua: &Lua, map_selector: Sel) -> LuaResult<Function>
where
    Sel: Fn(&InputSnapshot) -> &HashMap<&'static str, bool> + Copy + Send + 'static,
{
    lua.create_function(move |lua, key: String| {
        let bishop_ctx = LuaBishopCtx::borrow_ctx(lua)?;
        let mut snapshot = get_input_snapshot();
        snapshot.capture_input_state(&bishop_ctx.ctx);

        let value = map_selector(&snapshot)
            .get(key.as_str())
            .copied()
            .unwrap_or(false);

        Ok(value)
    })
}

register_lua_api!(InputModule, ENGINE_FILE);

impl LuaApi for InputModule {
    fn emit_api(&self, out: &mut LuaApiWriter) {
        // input.is_down()
        out.line("---@param input string");
        out.line(&format!(
            "function {}.{}.{}(input) end",
            ENGINE, INPUT, INPUT_IS_DOWN
        ));
        out.line("");

        // input.is_pressed()
        out.line("---@param input string");
        out.line(&format!(
            "function {}.{}.{}(input) end",
            ENGINE, INPUT, INPUT_PRESSED
        ));
        out.line("");

        // input.is_released()
        out.line("---@param input string");
        out.line(&format!(
            "function {}.{}.{}(input) end",
            ENGINE, INPUT, INPUT_RELEASED
        ));
        out.line("");

        // input.take_control()
        out.line("---@param name string");
        out.line("---@param priority number");
        out.line(&format!(
            "function {}.{}.{}(name, priority) end",
            ENGINE, INPUT, INPUT_TAKE_CONTROL
        ));
        out.line("");

        // input.release_control()
        out.line("---@param name string");
        out.line(&format!(
            "function {}.{}.{}(name) end",
            ENGINE, INPUT, INPUT_RELEASE_CONTROL
        ));
        out.line("");

        // input.in_control()
        out.line("---@param name string");
        out.line("---@return boolean");
        out.line(&format!(
            "function {}.{}.{}(name) end",
            ENGINE, INPUT, INPUT_IN_CONTROL
        ));
        out.line("");
    }
}
