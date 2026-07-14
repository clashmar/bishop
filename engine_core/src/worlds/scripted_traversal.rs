use crate::ecs::{CurrentRoom, Script};
use crate::game::Game;
use crate::logging::omni_error;
use crate::scripting::helpers::sanitize_lua_identifier;
use crate::scripting::world_navigation_lua::collect_entry_handles;
use crate::storage::path_utils::scripts_folder;
use crate::worlds::RoomId;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;

static MOVE_TO_ENTRY_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\bmove_to_entry\s*\(\s*Entries\.([A-Za-z_][A-Za-z0-9_]*)\.([A-Za-z_][A-Za-z0-9_]*)\s*\)")
        .expect("move_to_entry regex should compile")
});

/// Concrete scripted room-to-room traversal inferred from Lua.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScriptedTraversalEdge {
    /// Room containing the scripted traversal source.
    pub from: RoomId,
    /// Room containing the destination entry.
    pub to: RoomId,
}

/// Extracts literal `Entries.World.Entry` traversals from a Lua source string.
pub fn extract_scripted_traversal_edges(
    source: &str,
    from: RoomId,
    lookup: &HashMap<(String, String), RoomId>,
) -> Vec<ScriptedTraversalEdge> {
    let mut edges = BTreeSet::new();

    for captures in MOVE_TO_ENTRY_PATTERN.captures_iter(source) {
        let Some(world_key) = captures.get(1).map(|capture| capture.as_str().to_string()) else {
            continue;
        };
        let Some(entry_key) = captures.get(2).map(|capture| capture.as_str().to_string()) else {
            continue;
        };
        let Some(&to) = lookup.get(&(world_key, entry_key)) else {
            continue;
        };
        edges.insert(ScriptedTraversalEdge { from, to });
    }

    edges.into_iter().collect()
}

/// Collects all literal scripted traversal edges authored in entity scripts.
pub fn collect_scripted_traversal_edges(game: &Game) -> Vec<ScriptedTraversalEdge> {
    let lookup = build_entry_lookup(game);
    let mut edges = BTreeSet::new();

    for (&entity, script) in &game.ecs.get_store::<Script>().data {
        if script.script_id.0 == 0 {
            continue;
        }
        let Some(CurrentRoom(from)) = game.ecs.get::<CurrentRoom>(entity).copied() else {
            continue;
        };
        let Some(relative_path) = game.script_manager.path_for_id(script.script_id) else {
            continue;
        };
        let path = scripts_folder().join(relative_path);
        let Ok(source) = fs::read_to_string(&path) else {
            omni_error!("Skipping scripted traversal extraction for {:?}: could not read {}", entity, path.display());
            continue;
        };

        for edge in extract_scripted_traversal_edges(&source, from, &lookup) {
            edges.insert(edge);
        }
    }

    edges.into_iter().collect()
}

fn build_entry_lookup(game: &Game) -> HashMap<(String, String), RoomId> {
    let mut entries = collect_entry_handles(game);
    entries.sort_by_key(|entry| {
        (
            entry.world_id,
            entry.world_name.clone(),
            entry.room_id,
            entry.entry_name.clone(),
        )
    });

    let mut used_entry_keys: HashMap<String, HashSet<String>> = HashMap::new();
    let mut lookup = HashMap::new();
    for entry in entries {
        let entry_key = unique_lua_key(
            &entry.entry_name,
            "Entry",
            used_entry_keys.entry(entry.world_key.clone()).or_default(),
        );
        lookup.insert((entry.world_key, entry_key), entry.room_id);
    }
    lookup
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::{Active, Script, WorldEntry};
    use crate::engine_global::set_game_name;
    use crate::game::Game;
    use crate::storage::path_utils::scripts_folder;
    use crate::storage::test_utils::{TestGameFolder, game_fs_test_lock};
    use crate::worlds::{Room, World, WorldId};

    #[test]
    fn scripted_traversal_extractor_when_source_uses_literal_entry_handle_emits_room_edge() {
        let lookup = HashMap::from([(
            ("SecondWorld".to_string(), WorldEntry::START_NAME.to_string()),
            RoomId(9),
        )]);

        let edges = extract_scripted_traversal_edges(
            "self.entity:move_to_entry(Entries.SecondWorld.Start)",
            RoomId(1),
            &lookup,
        );

        assert_eq!(
            edges,
            vec![ScriptedTraversalEdge {
                from: RoomId(1),
                to: RoomId(9),
            }]
        );
    }

    #[test]
    fn scripted_traversal_extractor_when_destination_is_dynamic_emits_no_room_edge() {
        let lookup = HashMap::from([(
            ("SecondWorld".to_string(), WorldEntry::START_NAME.to_string()),
            RoomId(9),
        )]);

        let edges = extract_scripted_traversal_edges(
            "local entry = Entries.SecondWorld.Start\nself.entity:move_to_entry(entry)",
            RoomId(1),
            &lookup,
        );

        assert!(edges.is_empty());
    }

    #[test]
    fn collect_scripted_traversal_edges_when_script_uses_literal_entry_handle_emits_room_edge() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let folder = TestGameFolder::new("scripted_traversal_edges");
        set_game_name(folder.name());
        fs::create_dir_all(scripts_folder()).unwrap();
        fs::write(
            scripts_folder().join("portal.lua"),
            "return { init = function(self) self.entity:move_to_entry(Entries.SecondWorld.Start) end }",
        )
        .unwrap();

        let mut game = Game::with_name(folder.name());

        let mut source_world = World::new(WorldId(1), "Main World".to_string(), 16.0);
        source_world.add_room(Room {
            id: RoomId(1),
            ..Default::default()
        });
        let mut target_world = World::new(WorldId(2), "Second World".to_string(), 16.0);
        target_world.add_room(Room {
            id: RoomId(9),
            ..Default::default()
        });
        game.add_world(source_world);
        game.add_world(target_world);

        game.ecs
            .create_entity()
            .with(WorldEntry {
                name: WorldEntry::START_NAME.to_string(),
                ..Default::default()
            })
            .with_current_room(RoomId(9))
            .finish();

        let script_id = game
            .script_manager
            .get_or_load(&mut game.asset_registry, "portal.lua")
            .expect("script should register");

        game.ecs
            .create_entity()
            .with(Active::default())
            .with(Script {
                script_id,
                ..Default::default()
            })
            .with_current_room(RoomId(1))
            .finish();

        let edges = collect_scripted_traversal_edges(&game);

        assert_eq!(
            edges,
            vec![ScriptedTraversalEdge {
                from: RoomId(1),
                to: RoomId(9),
            }]
        );
    }
}
