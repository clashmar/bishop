use crate::input::InputActionMap;
use crate::scripting::helpers::sanitize_lua_identifier;
use crate::scripting::lua_constants::{lua_globals, lua_ownership};
use mlua::prelude::{LuaResult, Lua};
use mlua::Table;
use std::collections::HashSet;

#[cfg(test)]
mod tests;

/// Returns Lua-safe keys paired with their authored action identifiers.
pub fn input_action_lua_entries(action_map: &InputActionMap) -> Vec<(String, String)> {
    let mut action_ids = action_map
        .actions
        .keys()
        .map(|action_id| action_id.as_str().to_string())
        .collect::<Vec<_>>();
    action_ids.sort();

    let mut used_keys = HashSet::new();
    action_ids
        .into_iter()
        .map(|action_id| {
            let key = unique_action_key(&action_id, &mut used_keys);
            (key, action_id)
        })
        .collect()
}

/// Generates the Lua table for a game's authored action identifiers.
pub fn generate_input_actions_lua(action_map: &InputActionMap) -> String {
    let input_action = lua_globals::INPUT_ACTION;
    let mut lua = format!(
        "-- Auto-generated. Do not edit.\n{}\n---@meta\n\n---@enum {input_action}\n{input_action} = {{\n",
        lua_ownership::LUA_OWNER_GAME_GENERATED,
    );

    for (key, value) in input_action_lua_entries(action_map) {
        lua.push_str(&format!("    {key} = {},\n", lua_string_literal(&value)));
    }

    lua.push_str("}\n");
    lua
}

/// Creates the current game's action identifier table for Lua scripts.
pub fn create_input_actions_lua_table(lua: &Lua, action_map: &InputActionMap) -> LuaResult<Table> {
    let table = lua.create_table()?;
    for (key, value) in input_action_lua_entries(action_map) {
        table.set(key, value)?;
    }
    Ok(table)
}

fn lua_string_literal(value: &str) -> String {
    let mut escaped = String::from("\"");
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\0'..='\u{1f}' | '\u{7f}'..='\u{ff}' => {
                escaped.push_str(&format!("\\x{:02x}", character as u32));
            }
            _ => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}

fn unique_action_key(action_id: &str, used_keys: &mut HashSet<String>) -> String {
    let base = sanitize_lua_identifier(action_id, "Action");
    if used_keys.insert(base.clone()) {
        return base;
    }

    let mut suffix = 2;
    loop {
        let candidate = format!("{base}_{suffix}");
        if used_keys.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
}
