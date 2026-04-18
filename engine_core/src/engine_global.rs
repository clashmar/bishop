// engine_core/src/engine_global.rs
use crate::constants::window;
use once_cell::sync::Lazy;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineMode {
    Editor,
    Game,
    /// Uses editor paths but runs release code.
    Playtest,
}

static GAME_NAME: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));

/// Set the current game name globally.
pub fn set_game_name(name: impl Into<String>) {
    *GAME_NAME.lock().unwrap() = name.into();
}

/// Get a clone of the current game name.
pub fn game_name() -> String {
    GAME_NAME.lock().unwrap().clone()
}

pub static ENGINE_MODE: Mutex<EngineMode> = Mutex::new(EngineMode::Editor);

/// Switch the global engine mode.
pub fn set_engine_mode(mode: EngineMode) {
    *ENGINE_MODE.lock().unwrap() = mode;
}

/// Retrieve the current engine mode.
pub fn get_engine_mode() -> EngineMode {
    *ENGINE_MODE.lock().unwrap()
}

static CAM_TILE_DIMS: Mutex<(f32, f32)> =
    Mutex::new((window::DEFAULT_CAM_TILES_X, window::DEFAULT_CAM_TILES_Y));

/// Returns the width and height of the game virtual screen in terms of grid tiles,
/// or the default if not initialized.
pub fn cam_tile_dims() -> (f32, f32) {
    *CAM_TILE_DIMS.lock().unwrap()
}

/// Sets the global camera tile dimensions. Call when creating/loading a game.
pub fn set_global_cam_tile_dims(dims: (f32, f32)) {
    let mut guard = CAM_TILE_DIMS.lock().unwrap();
    let x = dims.0.max(1.0);
    let y = dims.1.max(1.0);
    *guard = (x, y);
}
