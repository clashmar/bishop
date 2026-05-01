#[cfg(feature = "editor")]
use crate::storage::editor_config::{config_path, save_config_to_path, EDITOR_CONFIG};
#[cfg(feature = "editor")]
use widgets::theme::{set_theme, Theme};

/// Persist a new theme to the editor config and update the active theme.
#[cfg(feature = "editor")]
pub fn save_editor_theme(new_theme: Theme) {
    let (snapshot, path) = match EDITOR_CONFIG.write() {
        Ok(mut cfg) => {
            cfg.theme = new_theme;
            (cfg.clone(), config_path())
        }
        Err(poison) => {
            crate::logging::onscreen_error!("Editor config lock poisoned: {poison}");
            return;
        }
    };

    set_theme(new_theme);

    if let Err(e) = save_config_to_path(&snapshot, &path) {
        crate::logging::onscreen_error!("Error saving theme: {e}");
    }
}
