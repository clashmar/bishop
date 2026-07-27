use crate::ecs::{CurrentRoom, WorldEntry};
use crate::game::Game;
use crate::scripting::helpers::sanitize_lua_identifier;
use crate::scripting::lua_constants::lua_ownership;
use crate::worlds::{RoomId, WorldId};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Generated handle for a world table entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldHandle {
    /// Authored world id.
    pub id: WorldId,
    /// Authored world name.
    pub name: String,
}

/// Generated handle for a world entry table entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntryHandle {
    /// Destination world id.
    pub world_id: WorldId,
    /// Canonical generated world key.
    pub world_key: String,
    /// Destination world name.
    pub world_name: String,
    /// Destination room id.
    pub room_id: RoomId,
    /// Authored entry name.
    pub entry_name: String,
}

/// Collects authored worlds into generated Lua handles.
pub fn collect_world_handles(game: &Game) -> Vec<WorldHandle> {
    game.worlds()
        .iter()
        .map(|world| WorldHandle {
            id: world.id,
            name: world.name.clone(),
        })
        .collect()
}

/// Collects authored world entries into generated Lua handles.
pub fn collect_entry_handles(game: &Game) -> Vec<EntryHandle> {
    let room_world_map = game.room_world_map();
    let world_keys = build_world_keys(&collect_world_handles(game));

    game.ecs
        .get_store::<WorldEntry>()
        .data
        .iter()
        .filter_map(|(&entity, entry)| {
            let room_id = game.ecs.get::<CurrentRoom>(entity)?.room_id;
            let world_id = room_world_map.get(&room_id).copied()?;
            let world_name = game.get_world(world_id)?.name.clone();
            let world_key = world_keys.get(&world_id)?.clone();
            Some(EntryHandle {
                world_id,
                world_key,
                world_name,
                room_id,
                entry_name: entry.name.clone(),
            })
        })
        .collect()
}

/// Generates `worlds.lua` from authored worlds.
pub fn generate_worlds_lua(worlds: &[WorldHandle]) -> String {
    let mut sorted_worlds = worlds.iter().collect::<Vec<_>>();
    sorted_worlds.sort_by_key(|world| (world.id, world.name.as_str()));
    let world_keys = build_world_keys(worlds);

    let mut out = format!(
        "-- Auto-generated. Do not edit.\n{}\n---@meta\n\n---@class Worlds\nlocal Worlds = {{\n",
        lua_ownership::LUA_OWNER_GAME_GENERATED,
    );

    for world in sorted_worlds {
        let Some(key) = world_keys.get(&world.id) else {
            continue;
        };
        out.push_str(&format!(
            "    {} = {{ Id = {}, Name = {} }},\n",
            key,
            world.id.0,
            lua_string_literal(&world.name),
        ));
    }

    out.push_str("}\n\nreturn Worlds\n");
    out
}

/// Generates `entries.lua` from authored world entries.
pub fn generate_entries_lua(entries: &[EntryHandle]) -> String {
    let mut sorted_entries = entries.iter().collect::<Vec<_>>();
    sorted_entries.sort_by_key(|entry| {
        (
            entry.world_id,
            entry.world_name.as_str(),
            entry.room_id,
            entry.entry_name.as_str(),
        )
    });

    let mut grouped_entries: BTreeMap<WorldId, Vec<&EntryHandle>> = BTreeMap::new();
    for entry in sorted_entries {
        grouped_entries.entry(entry.world_id).or_default().push(entry);
    }

    let mut out = format!(
        "-- Auto-generated. Do not edit.\n{}\n---@meta\n\n---@class Entries\nlocal Entries = {{\n",
        lua_ownership::LUA_OWNER_GAME_GENERATED,
    );

    for (_world_id, world_entries) in grouped_entries {
        let Some(world_key) = world_entries.first().map(|entry| entry.world_key.as_str()) else {
            continue;
        };
        out.push_str(&format!("    {} = {{\n", world_key));

        let mut used_entry_keys = HashSet::new();
        for entry in world_entries {
            let entry_key = unique_lua_key(&entry.entry_name, "Entry", &mut used_entry_keys);
            out.push_str(&format!(
                "        {} = {{ WorldId = {}, RoomId = {}, EntryName = {} }},\n",
                entry_key,
                entry.world_id.0,
                entry.room_id.0,
                lua_string_literal(&entry.entry_name),
            ));
        }

        out.push_str("    },\n");
    }

    out.push_str("}\n\nreturn Entries\n");
    out
}

fn build_world_keys(worlds: &[WorldHandle]) -> HashMap<WorldId, String> {
    let mut sorted_worlds = worlds.iter().collect::<Vec<_>>();
    sorted_worlds.sort_by_key(|world| (world.id, world.name.as_str()));

    let mut used_world_keys = HashSet::new();
    let mut world_keys = HashMap::new();
    for world in sorted_worlds {
        let key = unique_lua_key(&world.name, "World", &mut used_world_keys);
        world_keys.insert(world.id, key);
    }
    world_keys
}

fn unique_lua_key(value: &str, fallback_prefix: &str, used_keys: &mut HashSet<String>) -> String {
    let base = sanitize_lua_identifier(value, fallback_prefix);
    let mut key = base.clone();
    let mut suffix = 2;

    while !used_keys.insert(key.clone()) {
        key = format!("{base}_{suffix}");
        suffix += 1;
    }

    key
}

fn lua_string_literal(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::lua_constants::lua_ownership;

    #[test]
    fn generate_worlds_lua_sorts_world_handles_and_marks_file_as_game_generated() {
        let lua = generate_worlds_lua(&[
            WorldHandle {
                id: WorldId(2),
                name: "Arcade".to_string(),
            },
            WorldHandle {
                id: WorldId(1),
                name: "Overworld".to_string(),
            },
        ]);

        assert!(lua.contains(lua_ownership::LUA_OWNER_GAME_GENERATED));
        assert!(lua.contains("---@class Worlds"));
        assert!(lua.contains("Arcade = { Id = 2, Name = \"Arcade\" }"));
        assert!(lua.contains("Overworld = { Id = 1, Name = \"Overworld\" }"));
    }

    #[test]
    fn generate_entries_lua_groups_entries_under_sanitized_world_keys() {
        let lua = generate_entries_lua(&[
            EntryHandle {
                world_id: WorldId(1),
                world_key: "Overworld".to_string(),
                world_name: "Overworld".to_string(),
                room_id: RoomId(9),
                entry_name: WorldEntry::START_NAME.to_string(),
            },
            EntryHandle {
                world_id: WorldId(2),
                world_key: "Arcade".to_string(),
                world_name: "Arcade".to_string(),
                room_id: RoomId(20),
                entry_name: "FromMain".to_string(),
            },
        ]);

        assert!(lua.contains("---@class Entries"));
        assert!(lua.contains("Overworld = {"));
        assert!(lua.contains("Start = { WorldId = 1, RoomId = 9, EntryName = \"Start\" }"));
        assert!(lua.contains("Arcade = {"));
        assert!(lua.contains("FromMain = { WorldId = 2, RoomId = 20, EntryName = \"FromMain\" }"));
    }
}
