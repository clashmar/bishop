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
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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

/// Return the name of the most recently modified game folder.
pub fn most_recent_game_name() -> Option<String> {
    let mut best: Option<(String, SystemTime)> = None;

    for path in list_game_folders().ok()? {
        let name = path.file_name()?.to_string_lossy().into_owned();
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
    engine_core::engine_global::set_game_name(new_name);

    Ok(())
}

/// Find all game folders in `games/`.
pub fn list_game_folders() -> io::Result<Vec<PathBuf>> {
    let root = match cfg!(debug_assertions) {
        true => absolute_save_root(),
        false => absolute_save_root().join(paths::GAME_SAVE_ROOT),
    };

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
    list_game_folders()
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
