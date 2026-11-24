use super::preview::save_room_preview_to_path;
use super::{EditorConfig, RoomPreviewSettings};
use ron::{from_str, to_string};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

struct TempConfigPath(PathBuf);

impl TempConfigPath {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "bishop-preview-config-{}.ron",
            Uuid::new_v4()
        )))
    }
}

impl Drop for TempConfigPath {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[test]
fn room_preview_defaults_when_missing_then_zones_are_visible() -> Result<(), Box<dyn Error>> {
    let default_config = EditorConfig::default();
    let missing_group: EditorConfig = from_str("(save_root: None)")?;

    assert!(default_config.room_preview.interior_zones_visible);
    assert!(missing_group.room_preview.interior_zones_visible);
    Ok(())
}

#[test]
fn room_preview_when_serialized_and_deserialized_then_visibility_round_trips() -> Result<(), Box<dyn Error>> {
    let config = EditorConfig {
        room_preview: RoomPreviewSettings {
            interior_zones_visible: false,
        },
        ..EditorConfig::default()
    };

    let serialized = to_string(&config)?;
    let loaded: EditorConfig = from_str(&serialized)?;

    assert!(!loaded.room_preview.interior_zones_visible);
    Ok(())
}

#[test]
fn room_preview_when_hidden_at_isolated_path_then_visibility_persists() -> Result<(), Box<dyn Error>> {
    let path = TempConfigPath::new();
    let mut config = EditorConfig::default();

    save_room_preview_to_path(
        &mut config,
        RoomPreviewSettings {
            interior_zones_visible: false,
        },
        &path.0,
    )?;
    let loaded: EditorConfig = from_str(&fs::read_to_string(&path.0)?)?;

    assert!(!config.room_preview.interior_zones_visible);
    assert!(!loaded.room_preview.interior_zones_visible);
    Ok(())
}
