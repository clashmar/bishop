use engine_core::assets::{AssetKind, AssetRegistry};
use engine_core::constants::{extensions, paths};
use engine_core::scripting::lua_constants::lua_dirs;
use std::path::Path;

pub const HIDDEN_DIRS: &[&str] = &[
    paths::WINDOWS_FOLDER,
    paths::MAC_OS_FOLDER,
    paths::MENUS_FOLDER,
    paths::WORLDS_FOLDER,
    paths::PAYLOADS_FOLDER,
    lua_dirs::ENGINE,
];

pub const HIDDEN_FILENAMES: &[&str] = &[
    paths::GAME_RON,
    paths::STARTUP_RON,
    paths::LANGUAGE_MANIFEST,
    paths::ASSET_REGISTRY_RON,
    paths::ECS_RON,
    paths::TILE_REGISTRY_RON,
];

pub const HIDDEN_EXTENSIONS: &[&str] = &[extensions::ASEPRITE, extensions::ASE, extensions::JSON];

/// Determines which filesystem entries should be visible in the Resources browser.
pub struct PathFilter;

impl PathFilter {
    /// Returns true if a directory name should be shown.
    pub fn dir_visible(name: &str) -> bool {
        !HIDDEN_DIRS.contains(&name)
    }

    /// Returns true if a file name (with extension) should be shown.
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

    /// Returns true when a file is visible in the current resource directory.
    pub fn file_visible_in_resource_dir(
        name: &str,
        current_dir: &Path,
        resources_root: &Path,
    ) -> bool {
        if !Self::file_visible(name) {
            return false;
        }

        if AssetRegistry::kind_for_resource_path(current_dir, resources_root)
            == Some(AssetKind::Font)
        {
            return AssetKind::from_path(Path::new(name)) == Some(AssetKind::Font);
        }

        true
    }
}
