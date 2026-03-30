// editor/src/storage/editor_storage.rs
#![allow(unused)]
use crate::editor_assets::assets::write_sounds_lua;
use crate::storage::sound_preset_storage::*;
use crate::tilemap::tile_palette::TilePalette;
use crate::write_animations_lua;
use crate::write_engine_scripts;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;
use uuid::Uuid;

/// Create a brand-new game with a single empty world.
pub fn create_new_game(name: String) -> Game {
    onscreen_debug!("Creating new game.");

    // Set game name globally
    set_game_name(&name);
    set_current_sound_preset_library(SoundPresetLibrary::default());

    // Ensure the folder structure exists.
    create_game_folders(&name);

    let asset_manager = AssetManager::default();
    let script_manager = ScriptManager::default();

    // Build the game first so we can allocate room IDs globally
    let mut game = Game {
        version: 1,
        id: Uuid::new_v4(),
        name,
        ecs: Ecs::default(),
        worlds: vec![],
        asset_manager,
        script_manager,
        text_manager: TextManager::default(),
        current_world_id: WorldId(Uuid::nil()),
        game_map: GameMap::default(),
        next_room_id: 0,
    };

    let world = create_new_world(&mut game);
    game.current_world_id = world.id;
    game.worlds.push(world);

    // Create the global Player entity
    game.ecs
        .create_entity()
        .with(Player)
        .with(Global {})
        .with(PhysicsBody)
        .with(Name("Player".to_string()));

    // Save the game.
    if let Err(e) = save_game(&game) {
        onscreen_error!("Could not save the new game: {e}");
    }

    if let Err(e) = save_default_front_end_menus() {
        onscreen_error!("Could not scaffold default menus: {e}");
    }

    game
}

fn create_game_folders(name: &str) {
    let folders: [(PathBuf, &str); 6] = [
        (resources_folder_current(), RESOURCES_FOLDER),
        (assets_folder(), ASSETS_FOLDER),
        (scripts_folder(), SCRIPTS_FOLDER),
        (text_folder(), TEXT_FOLDER),
        (windows_folder(), WINDOWS_FOLDER),
        (mac_os_folder(), MAC_OS_FOLDER),
    ];

    for (path, folder) in folders {
        if let Err(e) = fs::create_dir_all(&path) {
            onscreen_error!("Could not create {folder} folder '{}': {e}", path.display());
        }
    }

    // Extract embedded _engine scripts
    if let Err(e) = write_engine_scripts(&scripts_folder()) {
        onscreen_error!("Could not write _engine scripts: {e}");
    }

    // Create an empty main.lua for the user (only if it doesn't already exist)
    let main_lua = scripts_folder().join("main.lua");
    if !main_lua.exists() {
        if let Err(e) = fs::write(&main_lua, "") {
            onscreen_error!("Could not create main.lua: {e}");
        }
    }

    // Create audio subfolders
    for path in [sfx_folder(), music_folder()] {
        if let Err(e) = fs::create_dir_all(&path) {
            onscreen_error!("Could not create audio folder '{}': {e}", path.display());
        }
    }

    // Create default text structure
    create_default_text_files();
}

/// Creates the default text manifest and language folders.
fn create_default_text_files() {
    let text_root = text_folder();

    // Create _manifest.toml with default config
    let manifest_path = text_root.join("_manifest.toml");
    if !manifest_path.exists() {
        let manifest_content = r#"# Text manifest configuration
default_language = "en"
"#;
        if let Err(e) = fs::write(&manifest_path, manifest_content) {
            onscreen_error!("Could not create text manifest: {e}");
        }
    }

    // Create default language folders
    let en_dialogue = text_root.join("en").join("dialogue");
    if let Err(e) = fs::create_dir_all(&en_dialogue) {
        onscreen_error!("Could not create text/en/dialogue folder: {e}");
    }

    let en_ui = text_root.join("en").join("ui");
    if let Err(e) = fs::create_dir_all(&en_ui) {
        onscreen_error!("Could not create text/en/ui folder: {e}");
    }

    let start_ui_path = en_ui.join("start.toml");
    if !start_ui_path.exists() {
        let content = r#"Title = "NEW GAME"
Start = "Start"
Settings = "Settings"
"#;
        if let Err(e) = fs::write(&start_ui_path, content) {
            onscreen_error!("Could not create ui/start.toml: {e}");
        }
    }

    let settings_ui_path = en_ui.join("settings.toml");
    if !settings_ui_path.exists() {
        let content = r#"Settings = "Settings"
Master = "Master Volume"
Music = "Music Volume"
SFX = "SFX Volume"
Back = "Back"
"#;
        if let Err(e) = fs::write(&settings_ui_path, content) {
            onscreen_error!("Could not create ui/settings.toml: {e}");
        }
    }
}

fn default_front_end_menus() -> Vec<MenuTemplate> {
    let start_layout = LayoutConfig::vertical()
        .with_item_size(240.0, 44.0)
        .with_spacing(16.0)
        .with_padding(Padding::uniform(32.0))
        .with_alignment(Alignment::center());

    let start_menu = MenuBuilder::new("start")
        .mode(MenuMode::FrontEnd)
        .background(MenuBackground::SolidColor(Color::new(0.05, 0.06, 0.10, 1.0)))
        .layout_group(Rect::new(0.0, 0.0, 1.0, 1.0), start_layout, |group| {
            group
                .label("Title")
                .button("Start", MenuAction::CloseMenu)
                .button("Settings", MenuAction::OpenMenu("settings".to_string()))
        })
        .build();

    let settings_layout = LayoutConfig::vertical()
        .with_item_size(320.0, 44.0)
        .with_spacing(16.0)
        .with_padding(Padding::uniform(32.0))
        .with_alignment(Alignment::center());

    let settings_menu = MenuBuilder::new("settings")
        .mode(MenuMode::FrontEnd)
        .background(MenuBackground::SolidColor(Color::new(0.05, 0.06, 0.10, 1.0)))
        .layout_group(Rect::new(0.0, 0.0, 1.0, 1.0), settings_layout, |group| {
            group
                .label("Settings")
                .slider("Master", "master_volume", 0.0, 1.0, 0.05, 1.0)
                .slider("Music", "music_volume", 0.0, 1.0, 0.05, 1.0)
                .slider("SFX", "sfx_volume", 0.0, 1.0, 0.05, 1.0)
                .button("Back", MenuAction::CloseMenu)
        })
        .build();

    vec![start_menu, settings_menu]
}

fn save_default_front_end_menus() -> io::Result<()> {
    for template in default_front_end_menus() {
        save_menu(&template)?;
    }
    Ok(())
}

/// Save a `Game` and all its contents.
pub fn save_game(game: &Game) -> io::Result<()> {
    let pretty = ron::ser::PrettyConfig::new()
        .separate_tuple_members(false)
        .enumerate_arrays(true);

    let ron_string = ron::ser::to_string_pretty(game, pretty).map_err(Error::other)?;

    let resources_folder = resources_folder_current();
    let file_path = resources_folder.join(GAME_RON);

    fs::create_dir_all(&resources_folder)?;

    // Regenerate animations.lua with custom clips
    let custom_clips = collect_custom_clip_names(&game.ecs);
    if let Err(e) = write_animations_lua(&scripts_folder(), &custom_clips) {
        onscreen_error!("Could not write animations.lua: {e}");
    }

    let sound_library = current_sound_preset_library();
    save_sound_preset_library(&game.name, &sound_library)?;
    let sound_names = collect_sound_group_names(&game.ecs, &sound_library);
    write_sounds_lua(&scripts_folder(), &sound_names)?;

    onscreen_info!("Game saved to: {}", file_path.display());
    fs::write(file_path, ron_string)
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

/// Load a `Game` from the folder that matches the supplied name.
pub fn load_game_by_name(name: &str) -> io::Result<Game> {
    let path = resources_folder(name).join(GAME_RON);
    onscreen_debug!("Loading game from .ron: {}.", path.display());

    // Try to read the file
    let ron_string = match fs::read_to_string(&path) {
        Ok(s) => s,
        // File not found
        Err(ref e) if e.kind() == ErrorKind::NotFound => {
            return Ok(create_new_game(name.to_string()))
        }
        // Other I/O errors
        Err(e) => return Err(e),
    };

    // Parse the RON
    let mut game = match ron::from_str::<Game>(&ron_string) {
        Ok(game) => game,
        Err(_) => return Ok(create_new_game(name.to_string())),
    };

    set_current_sound_preset_library(load_sound_preset_library(name)?);

    Ok(game)
}

/// Return the name of the most recently modified game folder.
pub fn most_recent_game_name() -> Option<String> {
    let root = absolute_save_root();
    let mut best: Option<(String, SystemTime)> = None;

    for entry in fs::read_dir(root).ok()? {
        let entry = entry.ok()?;
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Ok(mod_time) = entry.metadata().ok()?.modified() {
            match best {
                None => best = Some((name, mod_time)),
                Some((_, t)) if mod_time > t => best = Some((name, mod_time)),
                _ => {}
            }
        }
    }
    best.map(|(name, _)| name)
}

/// Save the palette for the game.
pub fn save_palette(palette: &TilePalette, game_name: &str) -> io::Result<()> {
    let dir = game_folder(game_name);
    fs::create_dir_all(&dir)?;
    let path = dir.join("palette.ron");
    let ron = ron::ser::to_string(palette).map_err(Error::other)?;
    fs::write(path, ron)
}

/// Load the palette from the game folder.
pub fn load_palette(game_name: &str) -> io::Result<TilePalette> {
    let path = game_folder(game_name).join("palette.ron");
    if !path.exists() {
        return Ok(TilePalette::new());
    }
    let ron = fs::read_to_string(path)?;
    ron::de::from_str(&ron).map_err(Error::other)
}

/// Create a fresh world with a single default room.
pub fn create_new_world(game: &mut Game) -> World {
    let id = WorldId(Uuid::new_v4());
    let name = "new".to_string();
    let room_id = game.allocate_room_id();
    let first_room = Room::new(&mut game.ecs, room_id, DEFAULT_GRID_SIZE);
    let room_origin = first_room.position;

    let world = World {
        id,
        name: name.clone(),
        rooms: vec![first_room],
        current_room_id: None,
        starting_room_id: Some(room_id),
        starting_position: Some(room_origin),
        meta: WorldMeta::default(),
        grid_size: DEFAULT_GRID_SIZE,
    };

    let _spawn_point = game
        .ecs
        .create_entity()
        .with(PlayerProxy)
        .with(Transform {
            position: room_origin,
            ..Default::default()
        })
        .with(CurrentRoom(room_id))
        .with(Name("Player Proxy".to_string()))
        .finish();

    world
}

/// Rename a game folder and assets.
/// Returns `Ok(())` on success or an `io::Error` on failure.
pub fn rename_game(game: &mut Game, new_name: &str) -> io::Result<()> {
    let old_game_dir = game_folder(&game.name);
    let new_game_dir = game_folder(new_name);
    fs::rename(&old_game_dir, &new_game_dir)?;
    game.name = new_name.to_owned();
    set_game_name(new_name);
    Ok(())
}

/// Save a copy of the current game in a newly named folder.
/// Returns `Ok(())` on success or an `io::Error` on failure.
pub fn save_as(game: &mut Game, new_name: &str) -> io::Result<()> {
    // Determine paths
    let old_game_dir = game_folder(&game.name);
    let new_game_dir = game_folder(new_name);

    // Guard against overwriting an existing game
    if new_game_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("A game called \"{new_name}\" already exists"),
        ));
    }

    // Copy the game folder
    copy_dir_recursive(&old_game_dir, &new_game_dir)?;

    // Update the game and global
    game.name = new_name.to_owned();
    set_game_name(new_name);

    Ok(())
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
        // Set executable permissions
        onscreen_debug!("Writing binary permissions.");
        let mut permissions = fs::metadata(&path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions)?;
    }

    Ok(path)
}

/// Find all game folders in `games/`.
pub fn list_game_folders() -> io::Result<Vec<PathBuf>> {
    let root = match cfg!(debug_assertions) {
        true => absolute_save_root(),
        false => absolute_save_root().join(GAME_SAVE_ROOT),
    };

    let mut folders = Vec::new();

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.join(GAME_RON).exists() {
            folders.push(path);
        }
    }

    Ok(folders)
}

/// Return the most recently modified game folder.
pub fn most_recent_game_folder() -> Option<PathBuf> {
    let mut best: Option<(PathBuf, SystemTime)> = None;

    for path in list_game_folders().ok()? {
        if let Ok(meta) = fs::metadata(&path) {
            if let Ok(mod_time) = meta.modified() {
                match best {
                    None => best = Some((path.clone(), mod_time)),
                    Some((_, t)) if mod_time > t => best = Some((path.clone(), mod_time)),
                    _ => {}
                }
            }
        }
    }

    best.map(|(p, _)| p)
}

/// Returns a Vec of all game names in the absolute save root.
pub fn list_game_names() -> Vec<String> {
    std::fs::read_dir(absolute_save_root())
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect()
}

/// Saves a menu template to disk.
pub fn save_menu(template: &MenuTemplate) -> io::Result<()> {
    let dir = menus_folder();
    fs::create_dir_all(&dir)?;

    let path = dir.join(format!("{}.ron", template.id));
    let pretty = ron::ser::PrettyConfig::new()
        .separate_tuple_members(true)
        .enumerate_arrays(true);

    let ron = ron::ser::to_string_pretty(template, pretty).map_err(Error::other)?;

    fs::write(path, ron)
}

/// Loads all menu templates from disk.
pub fn load_menus() -> Vec<MenuTemplate> {
    let dir = menus_folder();
    if !dir.exists() {
        return Vec::new();
    }

    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };

    entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "ron"))
        .filter_map(|entry| {
            let ron = fs::read_to_string(entry.path()).ok()?;
            ron::de::from_str(&ron).ok()
        })
        .collect()
}

/// Deletes a menu template from disk.
pub fn delete_menu(id: &str) -> io::Result<()> {
    let path = menus_folder().join(format!("{}.ron", id));
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
