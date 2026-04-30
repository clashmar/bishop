use crate::constants::paths;
use std::ops::Deref;
use std::path::{Path, PathBuf};

/// An engine-managed folder that must never be deleted, renamed, or moved.
/// Exposes read operations only; does not implement `Into<UserPath>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtectedPath(PathBuf);

impl ProtectedPath {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn into_inner(self) -> PathBuf {
        self.0
    }
}

impl Deref for ProtectedPath {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for ProtectedPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

/// A user-created file or folder path that may be freely deleted, renamed, or moved.
/// The only path type accepted by destructive editor commands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserPath(PathBuf);

impl UserPath {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn into_inner(self) -> PathBuf {
        self.0
    }
}

impl From<PathBuf> for UserPath {
    fn from(p: PathBuf) -> Self {
        Self(p)
    }
}

impl Deref for UserPath {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for UserPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

/// Top-level folder names (immediate children of Resources/) that are engine-managed.
pub const SYSTEM_FOLDER_ROOTS: &[&str] = &[
    paths::ASSETS_FOLDER,
    paths::SCRIPTS_FOLDER,
    paths::TEXT_FOLDER,
    paths::PREFABS_FOLDER,
    paths::AUDIO_FOLDER,
    paths::MENUS_FOLDER,
];

/// Engine-managed subfolder paths, expressed as arrays of existing constants.
/// Each entry is the sequence of path components from Resources/ downward.
///
/// When a new engine-managed subfolder is added (e.g. in
/// `editor_storage::create_game_folders`), add its component sequence here.
const ENGINE_SUBFOLDER_PATHS: &[&[&str]] = &[
    &[paths::AUDIO_FOLDER, paths::SFX_FOLDER],
    &[paths::AUDIO_FOLDER, paths::MUSIC_FOLDER],
    // Text locale subfolders created by create_default_text_files:
    &[
        paths::TEXT_FOLDER,
        paths::TEXT_LANGUAGE_FOLDER,
        paths::DIALOGUE_FOLDER,
    ],
    &[
        paths::TEXT_FOLDER,
        paths::TEXT_LANGUAGE_FOLDER,
        paths::UI_TEXT_FOLDER,
    ],
];

/// Returns `true` when `path` exactly matches an engine-managed folder
/// (relative to `resources_root`), protecting it from deletion/rename/move.
/// User-created subdirectories inside system folders are not protected.
pub fn is_protected_path(path: &Path, resources_root: &Path) -> bool {
    let relative = match path.strip_prefix(resources_root) {
        Ok(r) => r,
        Err(_) => return false,
    };
    SYSTEM_FOLDER_ROOTS
        .iter()
        .any(|&root| path_matches_segments(relative, &[root]))
        || ENGINE_SUBFOLDER_PATHS
            .iter()
            .any(|segments| path_matches_segments(relative, segments))
}

fn path_matches_segments(path: &Path, segments: &[&str]) -> bool {
    let mut components = path.components().filter_map(|c| c.as_os_str().to_str());
    for &expected in segments {
        match components.next() {
            Some(actual) if actual == expected => continue,
            _ => return false,
        }
    }
    components.next().is_none()
}
