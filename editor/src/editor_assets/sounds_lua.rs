use engine_core::scripting::lua_constants::LUA_OWNER_GAME_GENERATED;
use std::collections::HashSet;

/// Generates `sounds.lua` with sorted, sanitized sound group identifiers.
pub fn generate_sounds_lua(group_names: &[String]) -> String {
    let mut names = group_names.to_vec();
    names.sort();
    names.dedup();
    let mut used_keys = HashSet::new();

    let mut lua = format!(
        "-- Auto-generated. Do not edit.\n\
        {LUA_OWNER_GAME_GENERATED}\n\
        ---@meta\n\n\
        ---@enum SoundGroupId\n\
        local SoundGroupId = {{\n",
    );

    for name in names {
        let key = unique_lua_identifier(&name, "Sound", &mut used_keys);
        lua.push_str(&format!("    {} = \"{}\",\n", key, name));
    }

    lua.push_str("}\n\nreturn SoundGroupId\n");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_sounds_lua_marks_file_as_game_generated() {
        let lua = generate_sounds_lua(&[]);

        assert!(lua.contains(LUA_OWNER_GAME_GENERATED));
    }

    #[test]
    fn generate_sounds_lua_sorts_and_sanitizes_names() {
        let lua = generate_sounds_lua(&[
            "Talk".to_string(),
            "footsteps".to_string(),
            "1 Boss Attack".to_string(),
        ]);

        assert!(lua.contains("Footsteps = \"footsteps\""));
        assert!(lua.contains("Talk = \"Talk\""));
        assert!(lua.contains("Sound_1_Boss_Attack = \"1 Boss Attack\""));
    }

    #[test]
    fn generate_sounds_lua_disambiguates_identifier_collisions() {
        let lua = generate_sounds_lua(&["Boss Attack".to_string(), "Boss-Attack".to_string()]);

        assert!(lua.contains("BossAttack = \"Boss Attack\""));
        assert!(lua.contains("BossAttack_2 = \"Boss-Attack\""));
    }
}
