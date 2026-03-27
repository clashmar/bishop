// editor/src/storage/editor_config.rs
use crate::*;
use directories_next::ProjectDirs;
use once_cell::sync::Lazy;
use ron::from_str;
use ron::ser::{PrettyConfig, to_string_pretty};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

pub static EDITOR_CONFIG: Lazy<RwLock<EditorConfig>> = Lazy::new(|| RwLock::new(load_config()));

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EditorConfig {
    pub save_root: Option<PathBuf>,
}

/// Saves the editor config .ron file from the in memory config.
pub fn save_config() -> Result<(), Box<dyn Error>> {
    let path = config_path();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let config = EDITOR_CONFIG.read()?;
    let ron = to_string_pretty(&*config, PrettyConfig::default())?;
    fs::write(path, ron)?;
    Ok(())
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
