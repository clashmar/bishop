use engine_core::register_lua_api;
use engine_core::register_lua_module;
use engine_core::scripting::lua_constants::{lua_files, lua_theme};
use engine_core::scripting::modules::lua_module::*;
use mlua::prelude::LuaResult;
use mlua::Lua;
use strum::VariantNames;
use widgets::theme::WidgetType;

/// Theme module stub — actual Lua bindings are in `engine_core::scripting::runtime_bootstrap`.
/// This exists only for `LuaApi::emit_api` to generate `_engine/theme.lua`.
#[derive(Default)]
pub struct ThemeModule;
register_lua_module!(ThemeModule);

impl LuaModule for ThemeModule {
    fn register(&self, _lua: &Lua) -> LuaResult<()> {
        Ok(()) // registered via register_runtime_modules
    }
}

register_lua_api!(ThemeModule, lua_files::THEME);

impl LuaApi for ThemeModule {
    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line(&format!("---@class {}", lua_theme::CLASS_THEME_API));
        out.line("engine.theme = {}");
        out.line("");

        out.line(&format!("---@enum {}", lua_theme::CLASS_WIDGET_TYPE));
        out.line(&format!("{} = {{", lua_theme::CLASS_WIDGET_TYPE));
        for &name in WidgetType::VARIANTS {
            let wt: WidgetType = name.parse().unwrap();
            if !wt.is_exposed_to_lua() {
                continue;
            }
            out.line(&format!("    {} = \"{}\",", name, name));
        }
        out.line("}");
        out.line("");

        out.line(&format!("---@class {}", lua_theme::CLASS_THEME));
        macro_rules! field_doc {
            ($f:ident, $desc:literal) => {
                out.line(&format!(
                    "---@field {} {} -- {}",
                    stringify!($f),
                    lua_theme::CLASS_COLOR,
                    $desc
                ));
            };
        }
        widgets::each_color_field_desc!(field_doc);
        out.line(&format!(
            "---@field {} fun(self: {}, {}: {}|string, {}: table)",
            lua_theme::RULE,
            lua_theme::CLASS_THEME,
            lua_theme::SELECTOR,
            lua_theme::CLASS_WIDGET_TYPE,
            lua_theme::PROPS,
        ));
        out.line("");

        out.line(&format!("---@return {}", lua_theme::CLASS_THEME));
        out.line(&format!(
            "function engine.{}.{}() end",
            lua_theme::THEME,
            lua_theme::NEW
        ));
        out.line("");

        out.line("--- Activates the given theme globally.");
        out.line(&format!(
            "---@param {} {}",
            lua_theme::THEME,
            lua_theme::CLASS_THEME
        ));
        out.line(&format!(
            "function engine.{}.{}({}) end",
            lua_theme::THEME,
            lua_theme::ACTIVATE,
            lua_theme::THEME
        ));
        out.line("");
    }
}
