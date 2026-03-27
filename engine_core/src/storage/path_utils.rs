// engine_core/src/storage/path_utils.rs
use crate::constants::*;
use crate::engine_global::*;
use crate::storage::editor_config::*;
use crate::*;
use futures::executor::block_on;
use rfd::FileDialog;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::io::Error;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;

/// Path to the folder that belongs to a particular game (Editor).
pub fn game_folder(name: &str) -> PathBuf {
    absolute_save_root().join(sanitise_name(name))
}

/// Path to the resources folder for a game (Editor/Game/Playtest).
pub fn resources_folder(game_name: &str) -> PathBuf {
    match get_engine_mode() {
        EngineMode::Editor | EngineMode::Playtest => {
            // Both use the editor's save root
            game_folder(game_name).join(RESOURCES_FOLDER)
        }
        EngineMode::Game => {
            if cfg!(debug_assertions) {
                game_folder(game_name).join(RESOURCES_FOLDER)
            } else {
                // Panic is acceptable here as there is no possible fallback
                resources_dir_from_exe().unwrap()
            }
        }
    }
}

/// Path to the resources folder for the current game (Editor/Game).
pub fn resources_folder_current() -> PathBuf {
    resources_folder(&game_name())
}

/// Path to the assets folder inside the resources folder (Editor/Game).
pub fn assets_folder() -> PathBuf {
    resources_folder_current().join(ASSETS_FOLDER)
}

/// Path to the scripts folder inside the resources folder (Editor/Game).
pub fn scripts_folder() -> PathBuf {
    resources_folder_current().join(SCRIPTS_FOLDER)
}

/// Path to the text folder inside the resources folder (Editor/Game).
pub fn text_folder() -> PathBuf {
    resources_folder_current().join(TEXT_FOLDER)
}

/// Returns the path to the menus folder for the current game.
pub fn menus_folder() -> PathBuf {
    resources_folder_current().join(MENUS_FOLDER)
}

/// Path to the audio folder inside the resources folder (Editor/Game).
pub fn audio_folder() -> PathBuf {
    resources_folder_current().join(AUDIO_FOLDER)
}

/// Path to the windows folder inside the game folder (Editor).
pub fn windows_folder() -> PathBuf {
    game_folder(&game_name()).join(WINDOWS_FOLDER)
}

/// Path to the mac_os folder inside the game folder (Editor).
pub fn mac_os_folder() -> PathBuf {
    game_folder(&game_name()).join(MAC_OS_FOLDER)
}

/// Returns the absolute path to the folder that stores all games for the editor,
/// or the parent of the resources folder for games on all platforms.
pub fn absolute_save_root() -> PathBuf {
    // Game release mode uses exe_dir
    if get_engine_mode() == EngineMode::Game && !cfg!(debug_assertions) {
        let path = exe_dir().unwrap_or_else(|| {
            // If this isn't found then the game can't work
            onscreen_error!("Could not find exe_dir in game mode");
            panic!("Could not find exe_dir in game mode");
        });
        return path;
    }

    // This only works for debug editor playtest, the check is against the game build
    // Editor/Playtest dev mode - Playtest is release build but needs editor paths
    if cfg!(debug_assertions) || get_engine_mode() == EngineMode::Playtest {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            // TODO: Choose/Create a new one
            .expect("Cannot locate workspace root.");

        let path_buf = workspace_root.join(GAME_SAVE_ROOT);
        return path_buf;
    }

    // Editor release mode
    if let Some(user_path) = get_save_root() {
        // Ensure the folder still exists or recreate it
        if let Err(e) = fs::create_dir_all(&user_path) {
            onscreen_error!(
                "Could not create user save root '{}': {e}",
                user_path.display()
            );
        } else {
            return user_path;
        }

        onscreen_error!("Stored save root is no longer valid, resetting.");
        {
            match EDITOR_CONFIG.write() {
                Ok(mut cfg) => {
                    cfg.save_root = None;
                }
                Err(poison_err) => {
                    onscreen_error!("Could not lock editor config for writing: {}", poison_err);
                }
            }
        }

        // Update the .ron
        if let Err(e) = save_config() {
            onscreen_error!("Error saving config: {e}.");
        }
    } else {
        // Save root needs to be set
        if let Some(path_buf) = block_on(pick_save_root_async()) {
            return path_buf;
        }
    }

    // Fallback to the platform‑default location.
    let fallback_path = default_save_root();
    let _ = fs::create_dir_all(&fallback_path);
    onscreen_error!("Using fallback save root: {}", fallback_path.display());
    fallback_path
}

// Gets the dir that contains the current process.
pub fn exe_dir() -> Option<PathBuf> {
    // Path of the running executable
    let exe = std::env::current_exe().ok()?;

    // Platform specific layout
    #[cfg(target_os = "macos")]
    {
        // …/.app/Contents/MacOS/<app>
        exe.parent().map(|p| p.to_path_buf())
    }
    // Linux is yet to be implemented
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        // …/<app>.exe
        exe.parent().map(|p| p.to_path_buf())
    }
}

// Gets the Resources dir for the current process.
pub fn resources_dir_from_exe() -> Option<PathBuf> {
    // Path of the running executable dir
    let exe_dir = exe_dir()?;

    // Platform specific layout
    #[cfg(target_os = "macos")]
    {
        // …/Bishop.app/Contents/MacOS/
        return exe_dir
            .parent() // Contents/
            .map(|p| p.join(RESOURCES_FOLDER)); // Resources/
    }
    // Linux is yet to be implemented
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        // …/Bishop.exe  or  …/bishop
        Some(exe_dir.join(RESOURCES_FOLDER))
    }
}

/// Gets the bundle assets folder for the editor on macOS.
#[cfg(unix)]
pub fn bundle_assets_folder() -> Option<PathBuf> {
    let resources_dir = resources_dir_from_exe()?;
    Some(resources_dir.join(BUNDLE_ASSETS))
}

/// Pick the folder that will become the absolute save root.
pub async fn pick_save_root_async() -> Option<PathBuf> {
    // Let the user choose a base folder
    let base_folder = FileDialog::new()
        .set_title("Select a folder for the editor assets root directory.")
        .pick_folder()
        .unwrap_or_else(default_save_root);

    // Build the full path
    let save_root = base_folder.join(SAVE_ROOT).join(GAME_SAVE_ROOT);

    // Make sure the directory chain exists
    if let Err(e) = fs::create_dir_all(&save_root) {
        onscreen_error!("Cannot write to the selected folder: {e}");
        return None;
    }

    update_config_root(&save_root)?;

    onscreen_info!("Successfully created save root at: {:?}", save_root);
    Some(save_root)
}

/// Pick a new absolute save root and move the existing games
/// folder there if possible.
pub async fn change_save_root_async() -> Option<PathBuf> {
    // Let the user choose a new base folder
    let base_folder = rfd::FileDialog::new()
        .set_title("Select a new folder for the editor assets root directory.")
        .pick_folder()
        .unwrap_or_else(default_save_root);

    // Build the full path
    let new_root = base_folder.join(SAVE_ROOT).join(GAME_SAVE_ROOT);

    // Make sure the new folder can be created
    if let Err(e) = fs::create_dir_all(&new_root) {
        onscreen_error!("Cannot create the selected folder: {e}");
        return None;
    }

    // Move the old games folder (if it exists) to the new location
    if let Some(old_root) = get_save_root()
        && old_root != new_root
    {
        // Try a rename
        match fs::rename(&old_root, &new_root) {
            Ok(_) => {
                delete_save_root();
            }
            Err(rename_err) => {
                // If rename fails fall back to copy
                onscreen_error!("Rename failed: {rename_err}.");
                if let Err(copy_err) = copy_dir_recursive(&old_root, &new_root) {
                    // Continue even if copy fails
                    onscreen_error!("Failed to copy old games: {copy_err}.");
                } else {
                    delete_save_root();
                }
            }
        }
    }

    update_config_root(&new_root)?;

    onscreen_debug!("Save root changed to: {}", new_root.display());
    Some(new_root)
}

/// Deletes SAVE_ROOT if it is the parent of GAME_SAVE_ROOT after a
/// successful copy. DO NOT MAKE PUBLIC.
fn delete_save_root() {
    if let Some(root) = get_save_root()
        && let Some(parent) = root.parent()
        && parent.file_name() == Some(OsStr::new(SAVE_ROOT))
    {
        let _ = fs::remove_dir_all(parent);
    }
}

/// Update the in‑memory config and persist it.
fn update_config_root(root_path: &Path) -> Option<()> {
    match EDITOR_CONFIG.write() {
        Ok(mut cfg) => cfg.save_root = Some(root_path.to_path_buf()),
        Err(poison) => {
            onscreen_error!("Editor config lock poisoned: {poison}");
            return None;
        }
    }

    if let Err(e) = save_config() {
        onscreen_error!("Error saving  new save root: {e}");
        return None;
    }

    Some(())
}

/// Checks for a valid save root, or prompts the user to choose one.
/// Note: Caller should yield a frame before calling this to let the event loop start.
pub async fn ensure_save_root() -> bool {
    // Fast path
    if get_save_root().is_some() {
        return true;
    }

    // Show the async picker.
    if let Some(_path) = pick_save_root_async().await {
        return get_save_root().is_some();
    }

    // The user cancelled the picker
    false
}

/// Turns a game name into a safe folder name.
pub fn sanitise_name(name: &str) -> String {
    let trimmed = name.trim_matches(|c: char| c.is_whitespace());
    let mut out = trimmed
        .chars()
        .map(|c| {
            // keep spaces and TODO: decide other special chars
            if c.is_ascii_alphanumeric() || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();

    // Collapse consecutive underscores
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

/// Returns `Ok(())` if `candidate` is inside `absolute_save_root()`.
pub fn ensure_inside_save_root(path: &Path) -> Result<(), String> {
    let root = absolute_save_root();

    if path.starts_with(&root) {
        Ok(())
    } else {
        Err(format!(
            "Selected folder '{}' is not in the 'games' directory '{}'.",
            path.display(),
            root.display()
        ))
    }
}

/// Recursively copy the directory and all of its contents.
pub fn copy_dir_recursive(src: &PathBuf, dest: &PathBuf) -> io::Result<()> {
    copy_dir_filtered(src, dest, &[])
}

/// Recursively copy the directory, skipping files with the given extensions.
pub fn copy_dir_filtered(
    src: &PathBuf,
    dest: &PathBuf,
    skip_extensions: &[&str],
) -> io::Result<()> {
    if !src.is_dir() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("Source `{}` is not a directory.", src.display()),
        ));
    }

    // Create the target directory
    fs::create_dir_all(dest)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_filtered(&src_path, &dst_path, skip_extensions)?;
        } else {
            let should_skip = src_path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    skip_extensions
                        .iter()
                        .any(|&skip| skip.eq_ignore_ascii_case(ext))
                })
                .unwrap_or(false);

            if !should_skip {
                fs::copy(&src_path, &dst_path)?;
            }
        }
    }
    Ok(())
}

/// Platform-default location used when the user has not chosen a folder.
fn default_save_root() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").expect("HOME not set");
        Path::new(&home).join("Library/Application Support/Bishop Engine/games")
    }
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").expect("APPDATA not set");
        Path::new(&appdata).join("Bishop Engine\\games")
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let home = std::env::var("HOME").expect("HOME not set");
        Path::new(&home).join(".local/share/BishopEngine/games")
    }
}
