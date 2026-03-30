use engine_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const STARTUP_FILE: &str = "startup.ron";

/// Runtime-authored startup flow configuration.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct StartupAsset {
    /// Loading-phase screens shown before entering the runtime session flow.
    pub loading: LoadingConfig,
    /// Menu id to open when entering the front-end start menu.
    pub start_menu_id: String,
}

impl Default for StartupAsset {
    fn default() -> Self {
        Self {
            loading: LoadingConfig::default(),
            start_menu_id: "start".to_string(),
        }
    }
}

/// Startup screens shown before the runtime session is ready.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LoadingConfig {
    /// Non-skippable splash cards shown before the loading buffer.
    pub splash_screens: Vec<StartupScreenSpec>,
    /// Screen displayed while finalization finishes.
    pub fallback_screen: StartupScreenSpec,
}

/// One startup screen definition.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct StartupScreenSpec {
    /// Minimum time the screen should remain visible.
    pub min_duration_secs: f32,
    /// Full-screen background color.
    pub background_color: [f32; 4],
    /// Screen content.
    pub content: StartupScreenContent,
}

impl Default for StartupScreenSpec {
    fn default() -> Self {
        Self {
            min_duration_secs: 0.0,
            background_color: [0.0, 0.0, 0.0, 1.0],
            content: StartupScreenContent::Text {
                text: "Loading".to_string(),
                font_size: 48.0,
                color: [1.0, 1.0, 1.0, 1.0],
            },
        }
    }
}

/// Minimal startup screen content types for v1.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum StartupScreenContent {
    /// Centered text.
    Text {
        text: String,
        font_size: f32,
        color: [f32; 4],
    },
}

impl Default for StartupScreenContent {
    fn default() -> Self {
        StartupScreenContent::Text {
            text: "Loading".to_string(),
            font_size: 48.0,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// Loads the startup from the given resources folder, falling back to defaults.
pub fn load_startup_from_resources(resources_dir: &Path) -> StartupAsset {
    let path = resources_dir.join(STARTUP_FILE);
    let Ok(ron_str) = fs::read_to_string(&path) else {
        return StartupAsset::default();
    };

    ron::from_str(&ron_str).unwrap_or_else(|err| {
        onscreen_error!(
            "Failed to parse startup ron '{}': {}",
            path.display(),
            err
        );
        StartupAsset::default()
    })
}

/// Loads the startup for a game by name using the current engine-mode resource paths.
pub fn load_startup_for_game_name(game_name: &str) -> StartupAsset {
    let resources_dir = resources_folder(game_name);
    load_startup_from_resources(&resources_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("startup_test_{unique}"))
    }

    #[test]
    fn default_startup_matches_documented_defaults() {
        let asset = StartupAsset::default();

        assert!(asset.loading.splash_screens.is_empty());
        assert_eq!(asset.start_menu_id, "start");
        assert_eq!(
            asset.loading.fallback_screen.content,
            StartupScreenContent::Text {
                text: "Loading".to_string(),
                font_size: 48.0,
                color: [1.0, 1.0, 1.0, 1.0],
            }
        );
    }

    #[test]
    fn missing_startup_file_uses_defaults() {
        let dir = temp_dir();
        fs::create_dir_all(&dir).unwrap();

        let asset = load_startup_from_resources(&dir);

        assert_eq!(asset, StartupAsset::default());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn demo_startup_asset_parses() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../games/Demo/Resources/startup.ron");
        let ron_str = fs::read_to_string(path).unwrap();

        let asset = ron::from_str::<StartupAsset>(&ron_str).unwrap();

        assert_eq!(asset.loading.splash_screens.len(), 1);
    }

}
