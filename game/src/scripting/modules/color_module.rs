use engine_core::register_lua_api;
use engine_core::register_lua_module;
use engine_core::scripting::lua_constants::{lua_color, lua_files, lua_theme};
use engine_core::scripting::modules::lua_module::*;
use mlua::prelude::LuaResult;
use mlua::Lua;

/// Color module stub — actual Lua bindings are in `engine_core::scripting::color_bootstrap`.
/// This exists only for `LuaApi::emit_api` to generate `_engine/color.lua`.
#[derive(Default)]
pub struct ColorModule;
register_lua_module!(ColorModule);

impl LuaModule for ColorModule {
    fn register(&self, _lua: &Lua) -> LuaResult<()> {
        Ok(()) // registered via register_runtime_modules
    }
}

register_lua_api!(ColorModule, lua_files::COLOR);

impl LuaApi for ColorModule {
    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line(&format!("---@class {}", lua_theme::CLASS_COLOR));
        out.line(&format!("---@field {} number", lua_color::R));
        out.line(&format!("---@field {} number", lua_color::G));
        out.line(&format!("---@field {} number", lua_color::B));
        out.line(&format!("---@field {} number", lua_color::A));
        out.line(&format!(
            "---@field {} fun(hex: string, alpha?: number): {}",
            lua_color::FROM_HEX,
            lua_theme::CLASS_COLOR
        ));
        out.line(&format!(
            "---@field {} fun(r: number, g: number, b: number, a?: number): {}",
            lua_color::RGBA,
            lua_theme::CLASS_COLOR
        ));
        out.line(&format!("{} = {{}}", lua_theme::CLASS_COLOR));
        out.line("");
    }
}
