/// Fixed-step timing values.
pub mod timing {
    /// 60Hz physics.
    pub const FIXED_DT: f32 = 1.0 / 60.0;
    /// Protects against long freezes.
    pub const MAX_ACCUM: f32 = 0.05;
}

/// Shared world and grid defaults.
pub mod world {
    use bishop::prelude::*;

    /// Default grid size that the world scales to.
    pub const DEFAULT_GRID_SIZE: f32 = 16.0;
    pub const MINIMUM_GRID_SIZE: f32 = 5.0;

    /// Base tile size for editor scaling.
    pub const BASE_GRID_SIZE: f32 = 32.0;

    pub const DEFAULT_ROOM_SIZE: Vec2 = Vec2::new(16.0, 9.0);
    pub const DEFAULT_ROOM_POSITION: Vec2 = Vec2::new(0.0, 0.0);
}

/// File extension constants for known asset types.
pub mod extensions {
    /// Asset images (sprites, animations).
    pub const PNG: &str = "png";
    /// Lua script source.
    pub const LUA: &str = "lua";
    /// WAVE audio.
    pub const WAV: &str = "wav";
    /// TOML data files (text, manifests).
    pub const TOML: &str = "toml";
    /// RON data files (menus, game state, editor config).
    pub const RON: &str = "ron";
    /// Prefab asset files backed by RON text.
    pub const PREFAB: &str = "prefab";
    /// Aseprite source files (stripped on export, not assets).
    pub const ASEPRITE: &str = "aseprite";
    /// Aseprite source files (short extension).
    pub const ASE: &str = "ase";
    /// Aseprite-exported frame data (stripped on export, not assets).
    pub const JSON: &str = "json";
}

/// Shared save and bundle paths.
pub mod paths {
    /// Name of the game .ron save file.
    pub const GAME_RON: &str = "game.ron";

    /// Name of the startup .ron file.
    pub const STARTUP_RON: &str = "startup.ron";

    /// Name of the language manifest file.
    pub const LANGUAGE_MANIFEST: &str = "_manifest.toml";

    /// Name of the root user-facing save folder for the editor.
    pub const SAVE_ROOT: &str = "Bishop";

    /// Name of the root of the save root for all games.
    pub const GAME_SAVE_ROOT: &str = "games";

    /// Name of the shipped demo game.
    pub const DEMO_GAME: &str = "Demo";

    /// Name of the `Resources` folder.
    pub const RESOURCES_FOLDER: &str = "Resources";

    /// Name of the assets folder.
    pub const ASSETS_FOLDER: &str = "assets";

    /// Name of the scripts folder.
    pub const SCRIPTS_FOLDER: &str = "scripts";

    /// Name of the text folder.
    pub const TEXT_FOLDER: &str = "text";

    /// Name of the folder that contains menu templates.
    pub const MENUS_FOLDER: &str = "menus";

    /// Name of the audio folder.
    pub const AUDIO_FOLDER: &str = "audio";

    /// Name of the sound effects subfolder inside audio.
    pub const SFX_FOLDER: &str = "sfx";

    /// Name of the music subfolder inside audio.
    pub const MUSIC_FOLDER: &str = "music";

    /// Name of the default language subfolder inside text.
    pub const TEXT_LANGUAGE_FOLDER: &str = "en";

    /// Name of the dialogue subfolder inside a language folder.
    pub const DIALOGUE_FOLDER: &str = "dialogue";

    /// Name of the UI text subfolder inside a language folder.
    pub const UI_TEXT_FOLDER: &str = "ui";

    /// Name of the prefabs folder.
    pub const PREFABS_FOLDER: &str = "prefabs";

    /// Name of the themes folder.
    pub const THEMES_FOLDER: &str = "themes";

    /// Name of the folder for windows-specific game assets.
    pub const WINDOWS_FOLDER: &str = "windows";

    /// Name of the folder for macOS-specific game assets.
    pub const MAC_OS_FOLDER: &str = "mac_os";

    /// Name of the macOS contents folder.
    pub const CONTENTS_FOLDER: &str = "Contents";

    /// Name of the bundle assets for the macOS editor.
    pub const BUNDLE_ASSETS: &str = "bundle_assets";

    /// Name of the underscore-prefixed folder that stores editor-only per-game metadata.
    pub const EDITOR_METADATA_FOLDER: &str = "_editor";
}

/// Scale to the base resolution.
pub fn editor_zoom_factor(grid_size: f32) -> f32 {
    grid_size / world::BASE_GRID_SIZE
}

/// Window sizing defaults.
pub mod window {
    use super::world::BASE_GRID_SIZE;

    pub const DEFAULT_CAM_GRID_X: f32 = 16.0;
    pub const DEFAULT_CAM_GRID_Y: f32 = 9.0;

    pub const FIXED_WINDOW_WIDTH: i32 = (DEFAULT_CAM_GRID_X * 3.0 * BASE_GRID_SIZE) as i32;
    pub const FIXED_WINDOW_HEIGHT: i32 = (DEFAULT_CAM_GRID_Y * 3.0 * BASE_GRID_SIZE) as i32;

    // Prevents the window from becoming absurdly small/large
    pub const MIN_WINDOW_WIDTH: i32 = 640;
    pub const MIN_WINDOW_HEIGHT: i32 = 360;
    pub const MAX_WINDOW_WIDTH: i32 = 2560;
    pub const MAX_WINDOW_HEIGHT: i32 = 1440;
}

/// UI layout defaults.
pub mod ui {
    /// Target design resolution width for menus and UI.
    pub const DESIGN_RESOLUTION_WIDTH: f32 = 1920.0;
    /// Target design resolution height for menus and UI.
    pub const DESIGN_RESOLUTION_HEIGHT: f32 = 1080.0;
}
