use crate::game_global::{is_menu_active, push_command};
use crate::scripting::commands::menu_commands::{CloseMenuCmd, OpenMenuCmd, SetElementEnabledCmd, SetElementVisibleCmd};
use engine_core::register_lua_api;
use engine_core::register_lua_module;
use engine_core::scripting::lua_constants::{lua_engine, lua_files, lua_menu};
use engine_core::scripting::modules::lua_module::*;
use mlua::prelude::LuaResult;
use mlua::{Lua, Table, Value};

/// Lua module that exposes the menu system API.
#[derive(Default)]
pub struct MenuModule;
register_lua_module!(MenuModule);

impl LuaModule for MenuModule {
    fn register(&self, lua: &Lua) -> LuaResult<()> {
        let engine_tbl: Table = lua.globals().get(lua_engine::ENGINE)?;
        let menu_tbl = lua.create_table()?;

        let open_fn = lua.create_function(|_, menu: Value| {
            push_command(Box::new(OpenMenuCmd {
                menu_id: menu_id_from_lua(menu)?,
            }));
            Ok(())
        })?;
        menu_tbl.set(lua_menu::OPEN, open_fn)?;

        let close_fn = lua.create_function(|_lua, ()| {
            push_command(Box::new(CloseMenuCmd));
            Ok(())
        })?;
        menu_tbl.set(lua_menu::CLOSE, close_fn)?;

        let is_open_fn = lua.create_function(|_lua, ()| Ok(is_menu_active()))?;
        menu_tbl.set(lua_menu::IS_OPEN, is_open_fn)?;

        let set_enabled_fn = lua.create_function(|_, (menu, name, enabled): (Value, String, bool)| {
            push_command(Box::new(SetElementEnabledCmd {
                menu_id: menu_id_from_lua(menu)?,
                element_name: name,
                enabled,
            }));
            Ok(())
        })?;
        menu_tbl.set(lua_menu::SET_ENABLED, set_enabled_fn)?;

        let set_visible_fn = lua.create_function(|_, (menu, name, visible): (Value, String, bool)| {
            push_command(Box::new(SetElementVisibleCmd {
                menu_id: menu_id_from_lua(menu)?,
                element_name: name,
                visible,
            }));
            Ok(())
        })?;
        menu_tbl.set(lua_menu::SET_VISIBLE, set_visible_fn)?;

        engine_tbl.set(lua_menu::MENU, menu_tbl)?;
        Ok(())
    }
}

register_lua_api!(MenuModule, lua_files::MENU);

impl LuaApi for MenuModule {
    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("--- Menu system module");
        out.line("---@class MenuApi");
        out.line("engine.menu = {}");
        out.line("");

        out.line("--- Opens a menu.");
        out.line(&format!(
            "---@param menu string|{} A menu id string or generated Menus table",
            lua_menu::MENUS_CLASS
        ));
        out.line("---@return nil");
        out.line("function engine.menu.open(menu) end");
        out.line("");

        out.line("--- Closes the current menu.");
        out.line("---@return nil");
        out.line("function engine.menu.close() end");
        out.line("");

        out.line("--- Returns true if any menu is currently active.");
        out.line("---@return boolean");
        out.line("function engine.menu.is_open() end");
        out.line("");

        out.line("--- Sets the enabled state of a named element in a menu template.");
        out.line(&format!("---@param menu string|{}", lua_menu::MENUS_CLASS));
        out.line("---@param element_name string");
        out.line("---@param enabled boolean");
        out.line("---@return nil");
        out.line(&format!(
            "function engine.menu.{}(menu, element_name, enabled) end",
            lua_menu::SET_ENABLED
        ));
        out.line("");

        out.line("--- Sets the visible state of a named element in a menu template.");
        out.line(&format!("---@param menu string|{}", lua_menu::MENUS_CLASS));
        out.line("---@param element_name string");
        out.line("---@param visible boolean");
        out.line("---@return nil");
        out.line(&format!(
            "function engine.menu.{}(menu, element_name, visible) end",
            lua_menu::SET_VISIBLE
        ));
        out.line("");
    }
}

fn menu_id_from_lua(value: Value) -> LuaResult<String> {
    match value {
        Value::String(id) => Ok(id.to_str()?.to_string()),
        Value::Table(table) => table.get::<String>("Id").map_err(|_| {
            mlua::Error::RuntimeError(format!(
                "menu argument must be a menu id string or {} table with an 'Id' field",
                lua_menu::MENUS_CLASS
            ))
        }),
        _ => Err(mlua::Error::RuntimeError(format!(
            "menu argument must be a menu id string or {} table with an 'Id' field",
            lua_menu::MENUS_CLASS
        ))),
    }
}
