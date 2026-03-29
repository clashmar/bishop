// game/src/scripting/modules/menu_module.rs
use crate::game_global::{is_menu_active, push_command};
use crate::scripting::commands::menu_commands::{CloseMenuCmd, OpenMenuCmd};
use engine_core::register_lua_api;
use engine_core::register_lua_module;
use engine_core::scripting::lua_constants::*;
use engine_core::scripting::modules::lua_module::*;
use mlua::prelude::LuaResult;
use mlua::Lua;
use mlua::Table;

/// Lua module that exposes the menu system API.
#[derive(Default)]
pub struct MenuModule;
register_lua_module!(MenuModule);

impl LuaModule for MenuModule {
    fn register(&self, lua: &Lua) -> LuaResult<()> {
        let engine_tbl: Table = lua.globals().get(ENGINE)?;
        let menu_tbl = lua.create_table()?;

        let open_fn = lua.create_function(|_lua, menu_id: String| {
            push_command(Box::new(OpenMenuCmd { menu_id }));
            Ok(())
        })?;
        menu_tbl.set(OPEN_MENU, open_fn)?;

        let close_fn = lua.create_function(|_lua, ()| {
            push_command(Box::new(CloseMenuCmd));
            Ok(())
        })?;
        menu_tbl.set(CLOSE_MENU, close_fn)?;

        let is_open_fn = lua.create_function(|_lua, ()| Ok(is_menu_active()))?;
        menu_tbl.set(IS_MENU_OPEN, is_open_fn)?;

        engine_tbl.set(LUA_MENU, menu_tbl)?;
        Ok(())
    }
}

register_lua_api!(MenuModule, MENU_FILE);

impl LuaApi for MenuModule {
    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Menu system module");
        out.line("---@class MenuApi");
        out.line("engine.menu = {}");
        out.line("");

        out.line("--- Opens a menu by id.");
        out.line("---@param menu_id string The menu template id");
        out.line("function engine.menu.open(menu_id) end");
        out.line("");

        out.line("--- Closes the current menu.");
        out.line("function engine.menu.close() end");
        out.line("");

        out.line("--- Returns true if any menu is currently active.");
        out.line("---@return boolean");
        out.line("function engine.menu.is_open() end");
        out.line("");
    }
}
