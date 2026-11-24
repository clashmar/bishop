use super::{EDITOR_CONFIG, config_path, save_config_to_path};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;

/// Preferences controlling room preview overlays.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoomPreviewSettings {
    /// Whether interior zone overlays are visible.
    #[serde(default = "default_interior_zones_visible")]
    pub interior_zones_visible: bool,
}

impl Default for RoomPreviewSettings {
    fn default() -> Self {
        Self {
            interior_zones_visible: default_interior_zones_visible(),
        }
    }
}

/// Returns the room preview preferences.
pub fn get_room_preview() -> RoomPreviewSettings {
    match EDITOR_CONFIG.read() {
        Ok(config) => config.room_preview.clone(),
        Err(poison) => {
            crate::omni_error!("Editor config lock poisoned: {poison}");
            RoomPreviewSettings::default()
        }
    }
}

/// Updates and persists the room preview preferences.
pub fn set_room_preview(settings: RoomPreviewSettings) {
    let (mut snapshot, path) = match EDITOR_CONFIG.write() {
        Ok(mut config) => {
            config.room_preview = settings.clone();
            (config.clone(), config_path())
        }
        Err(poison) => {
            crate::omni_error!("Editor config lock poisoned: {poison}");
            return;
        }
    };

    if let Err(error) = save_room_preview_to_path(&mut snapshot, settings, &path) {
        crate::omni_error!("Error saving room preview settings: {error}");
    }
}

pub(super) fn save_room_preview_to_path(
    config: &mut super::EditorConfig,
    settings: RoomPreviewSettings,
    path: &Path,
) -> Result<(), Box<dyn Error>> {
    config.room_preview = settings;
    save_config_to_path(config, path)
}

fn default_interior_zones_visible() -> bool {
    true
}
