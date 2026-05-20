/// Root folder for engine-managed runtime save files (outside Resources/).
pub const RUNTIME_SAVES_ROOT: &str = "_runtime_saves";

/// Folder containing slot directories for one game's runtime saves.
pub const RUNTIME_SAVE_SLOTS_FOLDER: &str = "slots";

/// Reserved slot name used by the MVP until player-facing slots exist.
pub const DEFAULT_RUNTIME_SAVE_SLOT: &str = "default";

/// Latest-save manifest file stored at the game runtime-save root.
pub const LATEST_RUNTIME_SAVE_MANIFEST: &str = "latest.ron";

/// Save lane file-stems.
pub mod lane_stems {
    pub const MANUAL: &str = "manual";
    pub const AUTOSAVE: &str = "autosave";
}
