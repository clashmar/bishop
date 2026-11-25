use crate::input::{InputActionId, InputActionMap, InputBinding};
use crate::logging::omni_error;
#[cfg(not(target_arch = "wasm32"))]
use crate::storage::app_dir;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use uuid::Uuid;

#[cfg(test)]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
const INPUT_SETTINGS_FILE: &str = "input_settings.ron";

/// Player-authored global and per-game action binding overrides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InputSettings {
    /// Global action overrides, applied when no game-specific override exists.
    pub global_overrides: HashMap<InputActionId, Vec<InputBinding>>,
    /// Game-specific overrides keyed by the serialized game identifier.
    pub per_game_overrides: HashMap<String, HashMap<InputActionId, Vec<InputBinding>>>,
}

impl InputSettings {
    /// Resolves bindings in per-game, global, then authored-default order.
    pub fn bindings_for<'a>(
        &'a self,
        game_id: Uuid,
        action_id: &InputActionId,
        authored: &'a InputActionMap,
    ) -> &'a [InputBinding] {
        self.per_game_overrides
            .get(&game_id.to_string())
            .and_then(|overrides| overrides.get(action_id))
            .or_else(|| self.global_overrides.get(action_id))
            .or_else(|| authored.actions.get(action_id))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

/// Loads persistent input overrides on native targets, or empty settings on WebAssembly.
#[cfg(target_arch = "wasm32")]
pub fn load_input_settings() -> InputSettings {
    InputSettings::default()
}

/// Loads player input overrides, using empty settings if the file is unavailable or invalid.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_input_settings() -> InputSettings {
    load_input_settings_from_path(&input_settings_path())
}

/// Saves player input overrides to the shared application settings file.
#[cfg(not(target_arch = "wasm32"))]
pub fn save_input_settings(settings: &InputSettings) -> io::Result<()> {
    save_input_settings_to_path(settings, &input_settings_path())
}

/// Reports that persistent input settings are unavailable on WebAssembly targets.
#[cfg(target_arch = "wasm32")]
pub fn save_input_settings(_settings: &InputSettings) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Persistent input settings are unavailable on WebAssembly targets",
    ))
}

#[cfg(not(target_arch = "wasm32"))]
fn input_settings_path() -> PathBuf {
    app_dir().join(INPUT_SETTINGS_FILE)
}

pub(crate) fn load_input_settings_from_path(path: &Path) -> InputSettings {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return InputSettings::default(),
        Err(error) => {
            omni_error!("Could not load input settings '{}': {error}", path.display());
            return InputSettings::default();
        }
    };

    match ron::from_str(&contents) {
        Ok(settings) => settings,
        Err(error) => {
            omni_error!("Could not parse input settings '{}': {error}", path.display());
            InputSettings::default()
        }
    }
}

pub(crate) fn save_input_settings_to_path(
    settings: &InputSettings,
    path: &Path,
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let serialized = ron::ser::to_string_pretty(settings, PrettyConfig::default())
        .map_err(io::Error::other)?;
    fs::write(path, serialized)
}
