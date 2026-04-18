use engine_core::scripting::lua_constants::lua_ownership;
use std::collections::HashSet;

/// Generates `prefabs.lua` with sorted, sanitized prefab identifiers.
pub fn generate_prefabs_lua(prefab_names: &[String]) -> String {
    let mut names = prefab_names.to_vec();
    names.sort();
    names.dedup();
    let mut used_keys = HashSet::new();

    let mut lua = format!(
        "-- Auto-generated. Do not edit.\n\
        {}\n\
        ---@meta\n\n\
        ---@enum PrefabId\n\
        local PrefabId = {{\n",
        lua_ownership::LUA_OWNER_GAME_GENERATED,
    );

    for name in names {
        let key = unique_lua_identifier(&name, "Prefab", &mut used_keys);
        lua.push_str(&format!("    {} = {},\n", key, lua_string_literal(&name)));
    }

    lua.push_str("}\n\nreturn PrefabId\n");
    lua
}

fn sanitize_lua_identifier_with_prefix(s: &str, prefix: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;

    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            if capitalize {
                out.push(ch.to_ascii_uppercase());
                capitalize = false;
            } else {
                out.push(ch);
            }
        } else {
            capitalize = true;
        }
    }

    if out.is_empty() || out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!(
            "{}_{}",
            prefix,
            s.replace(|c: char| !c.is_ascii_alphanumeric(), "_")
        )
    } else {
        out
    }
}

fn unique_lua_identifier(s: &str, prefix: &str, used_keys: &mut HashSet<String>) -> String {
    let base = sanitize_lua_identifier_with_prefix(s, prefix);
    if used_keys.insert(base.clone()) {
        return base;
    }

    let mut suffix = 2;
    loop {
        let candidate = format!("{}_{}", base, suffix);
        if used_keys.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
}

fn lua_string_literal(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_prefabs_lua_marks_file_as_game_generated() {
        let lua = generate_prefabs_lua(&[]);

        assert!(lua.contains(lua_ownership::LUA_OWNER_GAME_GENERATED));
    }

    #[test]
    fn generate_prefabs_lua_sorts_sanitizes_and_dedupes_names() {
        let lua = generate_prefabs_lua(&[
            "Crate".to_string(),
            "Boss Attack".to_string(),
            "Boss-Attack".to_string(),
            "Crate".to_string(),
            "1 Small Crate".to_string(),
        ]);

        assert!(lua.contains("BossAttack = \"Boss Attack\""));
        assert!(lua.contains("BossAttack_2 = \"Boss-Attack\""));
        assert!(lua.contains("Prefab_1_Small_Crate = \"1 Small Crate\""));
        assert_eq!(lua.matches("\"Crate\"").count(), 1);
    }

    #[test]
    fn generate_prefabs_lua_escapes_string_literals() {
        let lua = generate_prefabs_lua(&["Boss \"Alpha\"\\Beta".to_string()]);

        assert!(lua.contains("BossAlphaBeta = \"Boss \\\"Alpha\\\"\\\\Beta\""));
    }
}
