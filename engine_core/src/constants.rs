// engine_core/src/constants.rs
use bishop::prelude::*;

/// 60Hz pysics.
pub const FIXED_DT: f32 = 1.0 / 60.0;
/// Protects against long freezes.
pub const MAX_ACCUM: f32 = 0.05;

/// Default tile size that the world scales to.
pub const DEFAULT_GRID_SIZE: f32 = 8.0;
pub const MINIMUM_TILE_SIZE: f32 = 5.0;

/// Base tile size for editor scaling.git
pub const BASE_GRID_SIZE: f32 = 32.0;

pub const DEFAULT_ROOM_SIZE: Vec2 = Vec2::new(16.0, 9.0);
pub const DEFAULT_ROOM_POSITION: Vec2 = Vec2::new(0.0, 0.0);

/// Name of the game .ron save file.
pub const GAME_RON: &str = "game.ron";

/// Name of the root user-facing save folder for the editor.
pub const SAVE_ROOT: &str = "Bishop";

/// Name of the root of the save root for all games.
pub const GAME_SAVE_ROOT: &str = "games";

// Name of the the 'Resources' folder.
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

/// Name of the prefabs folder.
pub const PREFABS_FOLDER: &str = "prefabs";

/// Name of the folder for windows-specific game assets.
pub const WINDOWS_FOLDER: &str = "windows";

/// Name of the folder for macOS-specific game assets.
pub const MAC_OS_FOLDER: &str = "mac_os";

/// Name of the macOS contents folder.
pub const CONTENTS_FOLDER: &str = "Contents";

/// Name of the bundle assets for the macOS editor;
pub const BUNDLE_ASSETS: &str = "bundle_assets";

/// Scale to the base resolution.
pub fn editor_zoom_factor(grid_size: f32) -> f32 {
    grid_size / BASE_GRID_SIZE
}

pub const DEFAULT_CAM_TILES_X: f32 = 16.0;
pub const DEFAULT_CAM_TILES_Y: f32 = 9.0;

pub const FIXED_WINDOW_WIDTH: i32 = (DEFAULT_CAM_TILES_X * 3. * BASE_GRID_SIZE) as i32;
pub const FIXED_WINDOW_HEIGHT: i32 = (DEFAULT_CAM_TILES_Y * 3. * BASE_GRID_SIZE) as i32;

// Prevents the window from becoming absurdly small/large
pub const MIN_WINDOW_WIDTH: i32 = 640;
pub const MIN_WINDOW_HEIGHT: i32 = 360;
pub const MAX_WINDOW_WIDTH: i32 = 2560;
pub const MAX_WINDOW_HEIGHT: i32 = 1440;

/// Target design resolution width for menus and UI.
pub const DESIGN_RESOLUTION_WIDTH: f32 = 1920.0;
/// Target design resolution height for menus and UI.
pub const DESIGN_RESOLUTION_HEIGHT: f32 = 1080.0;

/// Name of the serialized playtest payload file.
pub const PLAYTEST_PAYLOAD_RON: &str = "playtest_payload.ron";

/// Constants for persisted playtest artifact contracts.
pub mod playtest_artifacts {
    /// Default file name for session manifests.
    pub const SESSION_FILENAME: &str = "session.ron";
    /// Default file name for session snapshots.
    pub const SNAPSHOT_FILENAME: &str = "snapshot.ron";
    /// Default file name for runtime snapshot request inboxes.
    pub const REQUEST_FILENAME: &str = "request.ron";
    /// Default file name for runtime control request inboxes.
    pub const CONTROL_REQUEST_FILENAME: &str = "control-request.ron";
    /// Default file name for persisted expanded control timelines.
    pub const EXPANDED_CONTROL_FILENAME: &str = "expanded-control.ron";
    /// Suffix used for a persisted playtest session directory.
    pub const SESSION_DIR_SUFFIX: &str = "_session";
    /// Default resources subfolder for file-defined playtest control profiles.
    pub const PLAYTEST_CONTROL_PROFILES_FOLDER: &str = "playtest_control_profiles";
    /// Mode label used by playtest visibility snapshots.
    pub const PLAYTEST_MODE: &str = "playtest";
    /// Topic label used by playtest runtime observations.
    pub const PLAYTEST_RUNTIME_TOPIC: &str = "playtest.runtime";
    /// Frame label used by playtest runtime observations.
    pub const PLAYTEST_FRAME_LABEL: &str = "frame";
}
