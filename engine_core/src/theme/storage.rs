#[cfg(feature = "editor")]
use crate::storage::editor_config::{config_path, save_config_to_path, EDITOR_CONFIG};
#[cfg(feature = "editor")]
use crate::theme::preset::find_preset_by_name;
#[cfg(feature = "editor")]
use widgets::theme::set_theme;

#[cfg(feature = "editor")]
pub fn save_editor_preset(preset_name: &str) {
    let (snapshot, path) = match EDITOR_CONFIG.write() {
        Ok(mut cfg) => {
            cfg.theme_preset = Some(preset_name.to_string());
            (cfg.clone(), config_path())
        }
        Err(poison) => {
            crate::logging::onscreen_error!("Editor config lock poisoned: {poison}");
            return;
        }
    };

    if let Some(preset) = find_preset_by_name(preset_name) {
        set_theme((preset.build)());
    }

    if let Err(e) = save_config_to_path(&snapshot, &path) {
        crate::logging::onscreen_error!("Error saving theme: {e}");
    }
}
