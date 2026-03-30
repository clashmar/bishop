// editor/src/storage/editor_config.rs
use crate::*;
#[cfg(feature = "editor")]
use crate::game::StartupMode;
use directories_next::ProjectDirs;
use once_cell::sync::Lazy;
use ron::from_str;
use ron::ser::{PrettyConfig, to_string_pretty};
use serde::{Deserialize, Serialize};
#[cfg(feature = "editor")]
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::RwLock;

pub static EDITOR_CONFIG: Lazy<RwLock<EditorConfig>> = Lazy::new(|| RwLock::new(load_config()));

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EditorConfig {
    pub save_root: Option<PathBuf>,
    #[cfg(feature = "editor")]
    #[serde(default = "default_startup_mode")]
    pub playtest_startup_mode: StartupMode,
    #[cfg(feature = "editor")]
    #[serde(default)]
    pub inspector_module_expanded: BTreeMap<String, bool>,
    #[cfg(feature = "editor")]
    #[serde(default)]
    pub panel_positions: BTreeMap<String, PanelPosition>,
}

#[cfg(feature = "editor")]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct PanelPosition {
    pub x: f32,
    pub y: f32,
}

#[cfg(feature = "editor")]
fn default_startup_mode() -> StartupMode {
    StartupMode::Skip
}

/// Saves the editor config .ron file from the in memory config.
pub fn save_config() -> Result<(), Box<dyn Error>> {
    let config = EDITOR_CONFIG.read()?;
    save_config_to_path(&config, &config_path())
}

/// Gets the config save root. Returns `None` if the lock is poisoned
/// or if the field itself is `None`.
pub fn get_save_root() -> Option<PathBuf> {
    if let Err(e) = EDITOR_CONFIG.read() {
        onscreen_error!("Could not read config: {e}.");
        None
    } else {
        // Safe unwrap
        EDITOR_CONFIG.read().unwrap().save_root.clone()
    }
}

#[cfg(feature = "editor")]
pub fn get_startup_mode() -> StartupMode {
    match EDITOR_CONFIG.read() {
        Ok(cfg) => cfg.playtest_startup_mode,
        Err(poison) => {
            onscreen_error!("Editor config lock poisoned: {poison}");
            default_startup_mode()
        }
    }
}

#[cfg(feature = "editor")]
pub fn set_startup_mode(startup_mode: StartupMode) {
    let (snapshot, path) = match EDITOR_CONFIG.write() {
        Ok(mut cfg) => {
            cfg.playtest_startup_mode = startup_mode;
            (cfg.clone(), config_path())
        }
        Err(poison) => {
            onscreen_error!("Editor config lock poisoned: {poison}");
            return;
        }
    };

    if let Err(e) = save_config_to_path(&snapshot, &path) {
        onscreen_error!("Error saving playtest launch preference: {e}");
    }
}

#[cfg(feature = "editor")]
pub fn get_inspector_module_expanded(title: &str) -> Option<bool> {
    match EDITOR_CONFIG.read() {
        Ok(cfg) => cfg.inspector_module_expanded.get(title).copied(),
        Err(poison) => {
            onscreen_error!("Editor config lock poisoned: {poison}");
            None
        }
    }
}

#[cfg(feature = "editor")]
pub fn set_inspector_module_expanded(title: &str, expanded: bool) {
    let (snapshot, path) = match EDITOR_CONFIG.write() {
        Ok(mut cfg) => {
            cfg.inspector_module_expanded
                .insert(title.to_string(), expanded);
            (cfg.clone(), config_path())
        }
        Err(poison) => {
            onscreen_error!("Editor config lock poisoned: {poison}");
            return;
        }
    };

    if let Err(e) = save_config_to_path(&snapshot, &path) {
        onscreen_error!("Error saving inspector module state: {e}");
    }
}

#[cfg(feature = "editor")]
pub fn get_panel_position(id: &str) -> Option<PanelPosition> {
    match EDITOR_CONFIG.read() {
        Ok(cfg) => cfg.panel_positions.get(id).copied(),
        Err(poison) => {
            onscreen_error!("Editor config lock poisoned: {poison}");
            None
        }
    }
}

#[cfg(feature = "editor")]
pub fn set_panel_position(id: &str, position: PanelPosition) {
    let (snapshot, path) = match EDITOR_CONFIG.write() {
        Ok(mut cfg) => {
            cfg.panel_positions.insert(id.to_string(), position);
            (cfg.clone(), config_path())
        }
        Err(poison) => {
            onscreen_error!("Editor config lock poisoned: {poison}");
            return;
        }
    };

    if let Err(e) = save_config_to_path(&snapshot, &path) {
        onscreen_error!("Error saving panel position state: {e}");
    }
}

/// Returns the app_dir for the program.
pub fn app_dir() -> PathBuf {
    // TODO: Insert 'company' name
    if let Some(project_dir) = ProjectDirs::from("com", "bishop", "engine") {
        project_dir.config_dir().to_path_buf()
    } else {
        onscreen_error!("Could not resolve app directory.");
        panic!("Could not resolve app directory.");
    }
}

fn config_path() -> PathBuf {
    app_dir().join("editor_config.ron")
}

fn save_config_to_path(config: &EditorConfig, path: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let ron = to_string_pretty(config, PrettyConfig::default())?;
    fs::write(path, ron)?;
    Ok(())
}

fn load_config() -> EditorConfig {
    let path = config_path();

    match fs::read_to_string(&path) {
        Ok(txt) => from_str(&txt).unwrap_or_default(),
        Err(e) => {
            onscreen_error!("Error loading config: {e}.");
            EditorConfig::default()
        }
    }
}

#[cfg(all(test, feature = "editor"))]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn defaults_have_empty_inspector_map() {
        let config = EditorConfig::default();
        assert_eq!(config.playtest_startup_mode, StartupMode::Skip);
        assert!(config.inspector_module_expanded.is_empty());
        assert!(config.panel_positions.is_empty());
    }

    #[test]
    fn inspector_map_deserializes_if_present() {
        let ron = r#"(inspector_module_expanded: { "Transform": true, "Audio Source": false })"#;
        let config: EditorConfig = from_str(ron).unwrap();

        assert_eq!(
            config.inspector_module_expanded.get("Transform"),
            Some(&true)
        );
        assert_eq!(
            config.inspector_module_expanded.get("Audio Source"),
            Some(&false)
        );
    }

    #[test]
    fn panel_positions_deserialize_if_present() {
        let ron = r#"(panel_positions: { "Console": (x: 120.5, y: 64.0) })"#;
        let config: EditorConfig = from_str(ron).unwrap();

        assert_eq!(
            config.panel_positions.get("Console"),
            Some(&PanelPosition { x: 120.5, y: 64.0 })
        );
    }

    #[test]
    fn playtest_startup_mode_defaults_to_skip_when_missing() {
        let config: EditorConfig = from_str(r#"(save_root: None)"#).unwrap();

        assert_eq!(config.playtest_startup_mode, StartupMode::Skip);
    }

    #[test]
    fn save_config_to_path_writes_inspector_map_without_global_lock() {
        let mut config = EditorConfig {
            playtest_startup_mode: StartupMode::Full,
            ..EditorConfig::default()
        };
        config
            .inspector_module_expanded
            .insert("Transform".to_string(), false);
        config
            .panel_positions
            .insert("Console".to_string(), PanelPosition { x: 42.0, y: 88.0 });

        let path =
            std::env::temp_dir().join(format!("bishop-editor-config-{}.ron", Uuid::new_v4()));

        save_config_to_path(&config, &path).unwrap();

        let saved = fs::read_to_string(&path).unwrap();
        let loaded: EditorConfig = from_str(&saved).unwrap();
        assert_eq!(loaded.playtest_startup_mode, StartupMode::Full);
        assert_eq!(
            loaded.inspector_module_expanded.get("Transform"),
            Some(&false)
        );
        assert_eq!(
            loaded.panel_positions.get("Console"),
            Some(&PanelPosition { x: 42.0, y: 88.0 })
        );

        let _ = fs::remove_file(path);
    }
}
