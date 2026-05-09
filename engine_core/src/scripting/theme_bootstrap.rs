use crate::scripting::lua_constants::{lua_engine, lua_theme};
use mlua::prelude::LuaResult;
use mlua::{Lua, Table};
use strum::VariantNames;
use widgets::theme::set_theme;
use widgets::theme::{StyleRule, StyleSelector, Theme, WidgetType, WidgetTheme};

const RULES_KEY: &str = lua_theme::RULES_TABLE;
const RULE_FN: &str = lua_theme::RULE;
const SELECTOR_KEY: &str = lua_theme::SELECTOR;

pub(crate) fn register_theme_helpers(lua: &Lua) -> LuaResult<()> {
    let engine_tbl: Table = lua.globals().get(lua_engine::ENGINE)?;
    let theme_tbl = lua.create_table()?;

    let new_fn = lua.create_function(|lua, ()| {
        let tbl = lua.create_table()?;
        let theme = Theme::default();
        macro_rules! field {
            ($f:ident) => {
                let ct = lua.create_table()?;
                ct.set("r", theme.$f.r)?;
                ct.set("g", theme.$f.g)?;
                ct.set("b", theme.$f.b)?;
                ct.set("a", theme.$f.a)?;
                tbl.set(stringify!($f), ct)?;
            };
        }
        widgets::each_color_field!(field);

        let rules_tbl = lua.create_table()?;
        tbl.set(RULES_KEY, rules_tbl)?;

        let rule_fn = lua.create_function(
            |lua, (tbl, selector_str, props_tbl): (Table, String, Table)| {
                let rules_tbl: Table = tbl.get(RULES_KEY)?;
                let len = rules_tbl.raw_len();
                let rule_tbl = lua.create_table()?;
                rule_tbl.set(SELECTOR_KEY, selector_str.as_str())?;
                macro_rules! prop {
                    ($f:ident) => {
                        if let Ok(c) = props_tbl.get::<Table>(stringify!($f)) {
                            let ct = lua.create_table()?;
                            ct.set("r", c.get::<f32>("r").unwrap_or(1.0))?;
                            ct.set("g", c.get::<f32>("g").unwrap_or(1.0))?;
                            ct.set("b", c.get::<f32>("b").unwrap_or(1.0))?;
                            ct.set("a", c.get::<f32>("a").unwrap_or(1.0))?;
                            rule_tbl.set(stringify!($f), ct)?;
                        }
                    };
                }
                widgets::each_color_field!(prop);
                rules_tbl.raw_set(len + 1, rule_tbl)?;
                Ok(())
            },
        )?;
        tbl.set(RULE_FN, rule_fn)?;
        Ok(tbl)
    })?;
    theme_tbl.set(lua_theme::NEW, new_fn)?;

    let activate_fn = lua.create_function(|_lua, theme_tbl: Table| {
        let theme = lua_table_to_theme(&theme_tbl)?;
        set_theme(theme);
        Ok(())
    })?;
    theme_tbl.set(lua_theme::ACTIVATE, activate_fn)?;

    engine_tbl.set(lua_theme::THEME, theme_tbl)?;

    let wt_tbl = lua.create_table()?;
    for &name in WidgetType::VARIANTS {
        wt_tbl.set(name, name)?;
    }
    lua.globals().set(lua_theme::CLASS_WIDGET_TYPE, wt_tbl)?;

    Ok(())
}

/// Converts a Lua table (from `engine.theme.new()` or equivalent) into a Rust [`Theme`].
pub fn lua_table_to_theme(tbl: &Table) -> LuaResult<Theme> {
    let mut theme = Theme::default();
    macro_rules! field {
        ($f:ident) => {
            if let Ok(ct) = tbl.get::<Table>(stringify!($f)) {
                theme.$f = bishop::Color::new(
                    ct.get("r").unwrap_or(1.0),
                    ct.get("g").unwrap_or(1.0),
                    ct.get("b").unwrap_or(1.0),
                    ct.get("a").unwrap_or(1.0),
                );
            }
        };
    }
    widgets::each_color_field!(field);

    if let Ok(rules_tbl) = tbl.get::<Table>(RULES_KEY) {
        let len = rules_tbl.raw_len();
        for i in 1..=len {
            if let Ok(rule_tbl) = rules_tbl.get::<Table>(i) {
                let selector_str: String = rule_tbl.get(SELECTOR_KEY).unwrap_or_default();
                let selector = selector_from_str(&selector_str);
                let mut props = WidgetTheme::default();
                macro_rules! prop {
                    ($f:ident) => {
                        if let Ok(ct) = rule_tbl.get::<Table>(stringify!($f)) {
                            props.$f = Some(bishop::Color::new(
                                ct.get("r").unwrap_or(1.0),
                                ct.get("g").unwrap_or(1.0),
                                ct.get("b").unwrap_or(1.0),
                                ct.get("a").unwrap_or(1.0),
                            ));
                        }
                    };
                }
                widgets::each_color_field!(prop);
                theme.rules.push(StyleRule {
                    selector,
                    properties: props,
                });
            }
        }
    }
    Ok(theme)
}

fn selector_from_str(s: &str) -> StyleSelector {
    if let Some(id) = s.strip_prefix('#') {
        StyleSelector::Id(id.to_string())
    } else if let Some(class) = s.strip_prefix('.') {
        StyleSelector::Class(class.to_string())
    } else {
        StyleSelector::Type(s.parse().unwrap_or(WidgetType::Button))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::event_bus::EventBus;
    use crate::scripting::runtime_bootstrap::register_runtime_modules;

    #[test]
    fn theme_with_rule_works() {
        let lua = Lua::new();
        let event_bus = EventBus::default();
        register_runtime_modules(&lua, &event_bus).unwrap();

        let script = r#"
            local t = engine.theme.new()
            t.primary = { r = 1.0, g = 0.0, b = 0.0, a = 1.0 }
            t.background = { r = 0.0, g = 0.0, b = 1.0, a = 1.0 }
            t:rule("Button", { background = { r = 0.0, g = 1.0, b = 0.0, a = 1.0 } })
            return t
        "#;
        let tbl: Table = lua.load(script).eval().unwrap();
        let theme = lua_table_to_theme(&tbl).unwrap();
        assert_eq!(theme.primary, bishop::Color::new(1.0, 0.0, 0.0, 1.0));
        assert_eq!(theme.rules.len(), 1);
        assert_eq!(
            theme.rules[0].properties.background,
            Some(bishop::Color::new(0.0, 1.0, 0.0, 1.0))
        );
    }
}
