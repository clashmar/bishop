use crate::editor_assets::prefabs_lua::generate_prefabs_lua;
use crate::editor_assets::sounds_lua::generate_sounds_lua;
use engine_core::animation::{generate_animations_lua, ClipId};
use engine_core::ecs::{Animation, Ecs};
use engine_core::menu::MenuTemplate;
use engine_core::prefab::PrefabManager;
use engine_core::scripting::event_tags::event_tag::{generate_event_tags_lua, EventTag};
use engine_core::scripting::generate_menus_lua;
use engine_core::scripting::lua_constants::{lua_dirs, lua_files};
use engine_core::scripting::lua_project::engine_relative_path;
use engine_core::scripting::menus_lua::generate_menus_lua_from_dir;
use engine_core::scripting::{
    collect_entry_handles, collect_world_handles, generate_entries_lua, generate_worlds_lua,
    EntryHandle, WorldHandle,
};
use engine_core::storage::scripts_folder;
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::io;
use std::io::{Error, ErrorKind};
use std::path::Path;

/// Writes initial per-game generated Lua tables so `_engine/globals.lua`
/// can require them immediately in a brand-new project.
pub fn write_initial_generated_lua_files(scripts_folder: &Path) -> io::Result<()> {
    write_animations_lua(scripts_folder, &[])?;
    write_prefabs_lua(scripts_folder, &[])?;
    write_sounds_lua(scripts_folder, &[])?;
    write_menus_lua(scripts_folder, &[])?;
    write_worlds_lua(scripts_folder, &[])?;
    write_entries_lua(scripts_folder, &[])?;
    write_event_tags_lua(scripts_folder, &[])?;
    Ok(())
}

/// Writes the per-game `animations.lua` file with built-in and custom clips.
pub fn write_animations_lua(scripts_folder: &Path, custom_clips: &[String]) -> io::Result<()> {
    let engine_folder = scripts_folder.join(lua_dirs::ENGINE);
    let path = engine_folder.join(engine_relative_path(lua_files::ANIMATIONS));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, generate_animations_lua(custom_clips))
}

/// Writes the per-game `sounds.lua` file with the supplied group names.
pub fn write_sounds_lua(scripts_folder: &Path, group_names: &[String]) -> io::Result<()> {
    let engine_folder = scripts_folder.join(lua_dirs::ENGINE);
    let path = engine_folder.join(engine_relative_path(lua_files::SOUNDS));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, generate_sounds_lua(group_names))
}

/// Writes the per-game `prefabs.lua` file with the supplied prefab names.
pub fn write_prefabs_lua(scripts_folder: &Path, prefab_names: &[String]) -> io::Result<()> {
    let engine_folder = scripts_folder.join(lua_dirs::ENGINE);
    let path = engine_folder.join(engine_relative_path(lua_files::PREFABS));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, generate_prefabs_lua(prefab_names))
}

/// Writes the per-game `menus.lua` file for the supplied menu templates.
pub fn write_menus_lua(scripts_folder: &Path, templates: &[MenuTemplate]) -> io::Result<()> {
    let engine_folder = scripts_folder.join(lua_dirs::ENGINE);
    let path = engine_folder.join(engine_relative_path(lua_files::MENUS));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, generate_menus_lua(templates))
}

/// Regenerates `menus.lua` from the current menus directory on disk.
pub fn write_menus_lua_from_dir(scripts_folder: &Path, menus_dir: &Path) -> io::Result<()> {
    let engine_folder = scripts_folder.join(lua_dirs::ENGINE);
    let path = engine_folder.join(engine_relative_path(lua_files::MENUS));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = generate_menus_lua_from_dir(menus_dir).map_err(io::Error::other)?;
    fs::write(path, content)
}

/// Writes the per-game `worlds.lua` file with the supplied world handles.
pub fn write_worlds_lua(scripts_folder: &Path, worlds: &[WorldHandle]) -> io::Result<()> {
    let engine_folder = scripts_folder.join(lua_dirs::ENGINE);
    let path = engine_folder.join(engine_relative_path(lua_files::WORLDS));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, generate_worlds_lua(worlds))
}

/// Writes the per-game `entries.lua` file with the supplied entry handles.
pub fn write_entries_lua(scripts_folder: &Path, entries: &[EntryHandle]) -> io::Result<()> {
    let engine_folder = scripts_folder.join(lua_dirs::ENGINE);
    let path = engine_folder.join(engine_relative_path(lua_files::ENTRIES));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, generate_entries_lua(entries))
}

/// Writes the per-game `event_tags.lua` file with built-in and custom tags.
pub fn write_event_tags_lua(scripts_folder: &Path, custom_tags: &[String]) -> io::Result<()> {
    let engine_folder = scripts_folder.join(lua_dirs::ENGINE);
    let path = engine_folder.join(engine_relative_path(lua_files::EVENT_TAGS));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, generate_event_tags_lua(custom_tags))
}

/// Collects all custom clip names from the ECS.
pub fn collect_custom_clip_names(ecs: &Ecs) -> Vec<String> {
    let mut names = HashSet::new();

    for animation in ecs.get_store::<Animation>().data.values() {
        for clip_id in animation.clips.keys() {
            if let ClipId::Custom(name) = clip_id {
                names.insert(name.clone());
            }
        }
    }

    names.into_iter().collect()
}

/// Collects all unique prefab names from the prefab manager.
pub fn collect_prefab_names(prefab_manager: &PrefabManager) -> io::Result<Vec<String>> {
    let mut names = HashSet::new();

    for prefab in prefab_manager.prefabs.values() {
        if !names.insert(prefab.name.clone()) {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("duplicate prefab name: {}", prefab.name),
            ));
        }
    }

    let mut names = names.into_iter().collect::<Vec<_>>();
    names.sort();
    Ok(names)
}

/// Collects custom event tags used in the game.
pub fn collect_custom_event_tags(game: &engine_core::game::Game) -> Vec<String> {
    let mut tags = BTreeSet::new();

    for world in game.worlds() {
        for room in world.rooms() {
            for tag in &room.tags {
                if let EventTag::Custom(name) = tag {
                    tags.insert(name.clone());
                }
            }
        }
    }

    tags.into_iter().collect()
}

/// Regenerates the global `event_tags.lua` file from the live game state.
pub fn refresh_event_tags_lua(game: &engine_core::game::Game) -> io::Result<()> {
    let custom_tags = collect_custom_event_tags(game);
    write_event_tags_lua(&scripts_folder(), &custom_tags)
}

/// Regenerates `worlds.lua` and `entries.lua` from the live game state.
pub fn refresh_world_navigation_lua(game: &engine_core::game::Game) -> io::Result<()> {
    let worlds = collect_world_handles(game);
    let entries = collect_entry_handles(game);
    write_worlds_lua(&scripts_folder(), &worlds)?;
    write_entries_lua(&scripts_folder(), &entries)
}
