use crate::storage::lua_stub_gen::{
    collect_custom_clip_names, collect_prefab_names, refresh_event_tags_lua,
    refresh_world_navigation_lua, write_animations_lua, write_prefabs_lua, write_sounds_lua,
};
use crate::storage::menus::save_default_front_end_menus;
use crate::storage::scaffolding::create_game_folders;
use crate::storage::sound_presets::*;
use crate::world::world_creation::create_new_world;
use engine_core::constants::paths;
use engine_core::ecs::*;
use engine_core::game::{Game, IdAllocator};
use engine_core::logging::{omni_debug, omni_error, omni_info};
use engine_core::storage::*;
use engine_core::storage::editor_config;
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use uuid::Uuid;

pub(crate) fn editor_metadata_path(game_name: &str, file_name: &str) -> PathBuf {
    editor_metadata_folder(game_name).join(file_name)
}

/// Writes an embedded slice of bytes to the system app directory and returns the path or error.
pub fn write_to_app_dir(filename: &str, embedded: &[u8]) -> io::Result<PathBuf> {
    let mut path = app_dir();
    fs::create_dir_all(&path)?;

    path.push(filename);

    let mut file = fs::File::create(&path)?;
    file.write_all(embedded)?;

    #[cfg(target_os = "macos")]
    {
        let mut permissions = fs::metadata(&path)?.permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(0o755);
        }
        fs::set_permissions(&path, permissions)?;
    }

    Ok(path)
}

/// Create a brand-new game with a single empty world.
pub fn create_new_game(name: String) -> Game {
    omni_debug!("Creating new game.");

    engine_core::engine_global::set_game_name(&name);
    set_current_sound_preset_library(SoundPresetLibrary::default());

    create_game_folders(&name);

    let mut game = Game::default();
    game.name = name;

    let world = create_new_world(&mut game);
    game.add_world(world);

    game.ecs
        .create_entity()
        .with(Player)
        .with(Global {})
        .with(PhysicsBody)
        .with(Transform::default())
        .with(Active::default())
        .with(Name("Player".to_string()));

    if let Err(e) = save_game(&game) {
        omni_error!("Could not save the new game: {e}");
    }

    if let Err(e) = save_default_front_end_menus() {
        omni_error!("Could not scaffold default menus: {e}");
    }

    game
}

/// Save a `Game` and all its contents.
pub fn save_game(game: &Game) -> io::Result<()> {
    let resources_folder = resources_folder_current();
    fs::create_dir_all(&resources_folder)?;

    let custom_clips = collect_custom_clip_names(&game.ecs);
    if let Err(e) = write_animations_lua(&scripts_folder(), &custom_clips) {
        omni_error!("Could not write animations.lua: {e}");
    }

    let prefab_names = collect_prefab_names(&game.prefab_manager)?;
    write_prefabs_lua(&scripts_folder(), &prefab_names)?;

    let sound_library = current_sound_preset_library();
    save_sound_preset_library(&game.name, &sound_library)?;
    let sound_names = collect_sound_group_names(&game.ecs, &sound_library);
    write_sounds_lua(&scripts_folder(), &sound_names)?;

    refresh_event_tags_lua(game)?;
    refresh_world_navigation_lua(game)?;

    omni_info!("Game saved to: {}", resources_folder.display());
    save_game_to_folder(game, &resources_folder)
}

/// Load a `Game` from the folder that matches the supplied name.
pub fn load_game_by_name(name: &str) -> io::Result<Game> {
    let resources = resources_folder(name);
    omni_debug!("Loading game from: {}.", resources.display());

    if !resources.join(paths::GAME_RON).exists() {
        return Ok(create_new_game(name.to_string()));
    }

    let mut game = load_full_game_from_folder(&resources)?;
    game.rebuild_world_index();
    if let Some(world) = game.current_world_mut() {
        world.rebuild_room_grid();
    }
    game.id_allocator = IdAllocator::from_game(&game);
    game.asset_registry.try_init_editor_metadata()?;

    set_current_sound_preset_library(load_sound_preset_library(name)?);

    Ok(game)
}

/// Returns the last opened game, or the most recently modified available game.
pub fn startup_game_name() -> Option<String> {
    let last_opened = editor_config::get_last_opened_game();
    startup_game_name_from_root(&absolute_save_root(), last_opened.as_deref())
}

fn startup_game_name_from_root(root: &Path, last_opened: Option<&str>) -> Option<String> {
    let mut best: Option<(String, SystemTime)> = None;

    for path in list_game_folders_from_root(root).ok()? {
        let name = path.file_name()?.to_string_lossy().into_owned();
        if last_opened == Some(name.as_str()) {
            return Some(name);
        }
        if let Ok(mod_time) = fs::metadata(&path).ok()?.modified() {
            match best {
                None => best = Some((name, mod_time)),
                Some((_, t)) if mod_time > t => best = Some((name, mod_time)),
                _ => {}
            }
        }
    }

    best.map(|(name, _)| name)
}

/// Rename a game folder and assets.
pub fn rename_game(game: &mut Game, new_name: &str) -> io::Result<()> {
    let old_game_dir = game_folder(&game.name);
    let new_game_dir = game_folder(new_name);
    fs::rename(&old_game_dir, &new_game_dir)?;
    game.name = new_name.to_owned();
    engine_core::engine_global::set_game_name(new_name);
    Ok(())
}

/// Save a copy of the current game in a newly named folder.
pub fn save_as(game: &mut Game, new_name: &str) -> io::Result<()> {
    let old_game_dir = game_folder(&game.name);
    let new_game_dir = game_folder(new_name);

    if new_game_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("A game called \"{new_name}\" already exists"),
        ));
    }

    copy_dir_recursive(&old_game_dir, &new_game_dir)?;

    game.name = new_name.to_owned();
    game.id = Uuid::new_v4();
    engine_core::engine_global::set_game_name(new_name);

    Ok(())
}

fn list_game_folders_from_root(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut folders = Vec::new();

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if is_game_folder(&path) {
            folders.push(path);
        }
    }

    Ok(folders)
}

/// Returns a Vec of all game names in the absolute save root.
pub fn list_game_names() -> Vec<String> {
    list_game_names_from_root(&absolute_save_root())
}

fn list_game_names_from_root(root: &Path) -> Vec<String> {
    list_game_folders_from_root(root)
        .into_iter()
        .flatten()
        .filter_map(|path| path.file_name().and_then(|name| name.to_str()).map(str::to_string))
        .collect()
}

fn is_game_folder(path: &Path) -> bool {
    path.is_dir()
        && path
            .join(paths::RESOURCES_FOLDER)
            .join(paths::GAME_RON)
            .exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{game_fs_test_lock, TestGameFolder};
    use engine_core::engine_global::set_game_name;
    use engine_core::storage::{load_full_game_from_folder, save_game_to_folder};
    use engine_core::worlds::{Room, RoomId, World, WorldId};
    use std::time::Duration;

    fn game_folder_at(root: &Path, name: &str, modified: SystemTime) -> io::Result<PathBuf> {
        let folder = root.join(name);
        let resources = folder.join(paths::RESOURCES_FOLDER);
        fs::create_dir_all(&resources)?;
        fs::write(resources.join(paths::GAME_RON), b"game")?;
        fs::File::open(&folder)?.set_modified(modified)?;
        Ok(folder)
    }

    #[test]
    fn editor_game_listing_when_root_contains_demo_finds_direct_child() -> io::Result<()> {
        let root = tempfile::tempdir()?;
        let demo = root.path().join("Demo");
        fs::create_dir_all(demo.join(paths::RESOURCES_FOLDER))?;
        fs::write(demo.join(paths::RESOURCES_FOLDER).join(paths::GAME_RON), b"game")?;

        let folders = list_game_folders_from_root(root.path())?;

        assert_eq!(folders, vec![demo]);
        Ok(())
    }

    #[test]
    fn list_game_names_with_non_game_directory_excludes_it() -> io::Result<()> {
        let root = tempfile::tempdir()?;
        let game_name = "OnlyGame";
        game_folder_at(root.path(), game_name, SystemTime::UNIX_EPOCH)?;
        fs::create_dir(root.path().join("OtherDirectory"))?;

        assert_eq!(list_game_names_from_root(root.path()), vec![game_name]);
        Ok(())
    }

    #[test]
    fn startup_game_without_preference_ignores_newer_non_game_directory() -> io::Result<()> {
        let root = tempfile::tempdir()?;
        let game_name = "OnlyGame";
        game_folder_at(root.path(), game_name, SystemTime::UNIX_EPOCH)?;
        let non_game = root.path().join("OtherDirectory");
        fs::create_dir(&non_game)?;
        fs::File::open(&non_game)?
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(10))?;

        assert_eq!(
            startup_game_name_from_root(root.path(), None).as_deref(),
            Some(game_name)
        );
        Ok(())
    }

    #[test]
    fn startup_game_with_last_opened_prefers_it_over_newer_folder() -> io::Result<()> {
        let root = tempfile::tempdir()?;
        let older_name = "PreviouslyOpened";
        game_folder_at(
            root.path(), older_name, SystemTime::UNIX_EPOCH + Duration::from_secs(10),
        )?;
        game_folder_at(
            root.path(), "NewerFolder", SystemTime::UNIX_EPOCH + Duration::from_secs(20),
        )?;

        let selected = startup_game_name_from_root(root.path(), Some(older_name));

        assert_eq!(selected.as_deref(), Some(older_name));
        Ok(())
    }

    #[test]
    fn startup_game_with_missing_preference_falls_back_to_newest_valid_folder() -> io::Result<()> {
        let root = tempfile::tempdir()?;
        let older_name = "PreviouslyOpened";
        let older = game_folder_at(
            root.path(), older_name, SystemTime::UNIX_EPOCH + Duration::from_secs(10),
        )?;
        let recent_name = "NewerFolder";
        game_folder_at(
            root.path(), recent_name, SystemTime::UNIX_EPOCH + Duration::from_secs(20),
        )?;

        assert_eq!(
            startup_game_name_from_root(root.path(), None).as_deref(),
            Some(recent_name)
        );
        fs::remove_file(older.join(paths::RESOURCES_FOLDER).join(paths::GAME_RON))?;
        assert_eq!(
            startup_game_name_from_root(root.path(), Some(older_name)).as_deref(),
            Some(recent_name)
        );
        Ok(())
    }

    #[test]
    fn save_as_when_forking_assigns_new_game_id_and_rename_preserves_it() -> io::Result<()> {
        let _lock = game_fs_test_lock().lock().unwrap_or_else(|poison| poison.into_inner());
        let original = TestGameFolder::new("save_as_original");
        let fork = TestGameFolder::new("save_as_fork");
        let renamed = TestGameFolder::new("save_as_renamed");
        set_game_name(original.name());
        let mut game = Game::with_name(original.name());
        let mut world = World::new(WorldId(1), "Start".to_string(), 16.0);
        world.add_room(Room::new(&mut game.ecs, RoomId(1), 16.0));
        game.add_world(world);
        let original_id = game.id;
        save_game_to_folder(&game, &original.path().join(paths::RESOURCES_FOLDER))?;

        save_as(&mut game, fork.name())?;
        assert_ne!(game.id, original_id);
        let fork_id = game.id;
        rename_game(&mut game, renamed.name())?;
        assert_eq!(game.id, fork_id);
        save_game_to_folder(&game, &renamed.path().join(paths::RESOURCES_FOLDER))?;
        let reloaded = load_full_game_from_folder(&renamed.path().join(paths::RESOURCES_FOLDER))?;
        assert_eq!(reloaded.id, fork_id);
        Ok(())
    }
}
