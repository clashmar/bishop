use engine_core::constants::{extensions, paths};
use engine_core::scripting::lua_constants::lua_dirs;
use std::path::Path;

pub const HIDDEN_DIRS: &[&str] = &[
    paths::WINDOWS_FOLDER,
    paths::MAC_OS_FOLDER,
    paths::MENUS_FOLDER,
    lua_dirs::ENGINE,
];

pub const HIDDEN_FILENAMES: &[&str] = &[
    paths::GAME_RON,
    paths::STARTUP_RON,
    paths::LANGUAGE_MANIFEST,
];

pub const HIDDEN_EXTENSIONS: &[&str] = &[extensions::ASEPRITE, extensions::ASE, extensions::JSON];

/// Determines which filesystem entries should be visible in the Resources browser.
pub struct PathFilter;

impl PathFilter {
    /// Returns true if a directory name should be shown.
    /// Hides: `_engine`, `windows`, `mac_os`, `menus`.
    pub fn dir_visible(name: &str) -> bool {
        !HIDDEN_DIRS.contains(&name)
    }

    /// Returns true if a file name (with extension) should be shown.
    /// Hides: dotfiles, `game.ron`, `startup.ron`, any `.aseprite`/`.ase`/`.json` file.
    pub fn file_visible(name: &str) -> bool {
        if name.starts_with('.') {
            return false;
        }
        if HIDDEN_FILENAMES.contains(&name) {
            return false;
        }
        if let Some(ext) = Path::new(name).extension().and_then(|e| e.to_str()) {
            if HIDDEN_EXTENSIONS.contains(&ext) {
                return false;
            }
        }
        true
    }
}
