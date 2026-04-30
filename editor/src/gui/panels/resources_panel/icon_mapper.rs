use engine_core::constants::extensions;
use engine_core::storage::is_protected_path;
use std::path::Path;

/// Icon types used in the Resources browser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconType {
    Folder,
    SystemFolder,
    LuaScript,
    Image,
    Audio,
    Text,
    Prefab,
    File,
}

pub const FILE_ICON_MAP: &[(&str, IconType)] = &[
    (extensions::PNG, IconType::Image),
    (extensions::LUA, IconType::LuaScript),
    (extensions::WAV, IconType::Audio),
    (extensions::TOML, IconType::Text),
    (extensions::PREFAB, IconType::Prefab),
];

/// Maps file extensions and directory names to icon types.
pub struct IconMapper;

impl IconMapper {
    /// Returns Folder for all non-system directories.
    pub fn dir_icon() -> IconType {
        IconType::Folder
    }

    /// Returns Folder for all non-system directories.
    pub fn sys_dir_icon() -> IconType {
        IconType::SystemFolder
    }

    /// Returns the correct icon for a directory based on whether it is
    /// engine-managed (system) or user-created.
    pub fn dir_icon_for(path: &Path, resources_root: &Path) -> IconType {
        if is_protected_path(path, resources_root) {
            IconType::SystemFolder
        } else {
            IconType::Folder
        }
    }

    /// Returns the icon type for a file based on its extension.
    /// Maps known extensions to specific icons; unknown extensions get File.
    pub fn file_icon(name: &str) -> IconType {
        Path::new(name)
            .extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| {
                FILE_ICON_MAP
                    .iter()
                    .find(|(file_ext, _)| file_ext == &ext)
                    .map(|&(_, icon)| icon)
            })
            .unwrap_or(IconType::File)
    }
}
