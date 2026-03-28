// editor/src/storage/editor_config.rs
use crate::*;
use directories_next::ProjectDirs;
use once_cell::sync::Lazy;
use ron::from_str;
use ron::ser::{PrettyConfig, to_string_pretty};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::RwLock;
#[cfg(feature = "editor")]
use std::collections::BTreeMap;

pub static EDITOR_CONFIG: Lazy<RwLock<EditorConfig>> = Lazy::new(|| RwLock::new(load_config()));

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EditorConfig {
    pub save_root: Option<PathBuf>,
    #[cfg(feature = "editor")]
    #[serde(default)]
    pub inspector_module_expanded: BTreeMap<String, bool>,
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
            cfg.inspector_module_expanded.insert(title.to_string(), expanded);
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
        assert!(config.inspector_module_expanded.is_empty());
    }

    #[test]
    fn inspector_map_deserializes_if_present() {
        let ron = r#"(inspector_module_expanded: { "Transform": true, "Audio Source": false })"#;
        let config: EditorConfig = from_str(ron).unwrap();

        assert_eq!(config.inspector_module_expanded.get("Transform"), Some(&true));
        assert_eq!(config.inspector_module_expanded.get("Audio Source"), Some(&false));
    }

    #[test]
    fn save_config_to_path_writes_inspector_map_without_global_lock() {
        let mut config = EditorConfig::default();
        config
            .inspector_module_expanded
            .insert("Transform".to_string(), false);

        let path = std::env::temp_dir()
            .join(format!("bishop-editor-config-{}.ron", Uuid::new_v4()));

        save_config_to_path(&config, &path).unwrap();

        let saved = fs::read_to_string(&path).unwrap();
        let loaded: EditorConfig = from_str(&saved).unwrap();
        assert_eq!(loaded.inspector_module_expanded.get("Transform"), Some(&false));

        let _ = fs::remove_file(path);
    }
}
