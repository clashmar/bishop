use super::{save_config_to_path, EditorConfig, PanelPosition};
use ron::from_str;
use std::error::Error;
use std::fs;
use uuid::Uuid;

#[test]
fn panel_geometry_round_trips_optional_dimensions() -> Result<(), Box<dyn Error>> {
    let geometry = PanelPosition {
        x: 12.0,
        y: 34.0,
        width: Some(280.0),
        height: Some(420.0),
    };
    let mut config = EditorConfig::default();
    config
        .panel_positions
        .insert("Test panel".to_string(), geometry);
    let path = std::env::temp_dir().join(format!("bishop-panel-{}.ron", Uuid::new_v4()));

    save_config_to_path(&config, &path)?;
    let saved = fs::read_to_string(&path)?;
    let loaded: EditorConfig = from_str(&saved)?;
    fs::remove_file(path)?;

    assert_eq!(
        loaded
            .panel_positions
            .get("Test panel")
            .map(|saved| (saved.x, saved.y, saved.width, saved.height)),
        Some((12.0, 34.0, Some(280.0), Some(420.0)))
    );
    Ok(())
}

#[test]
fn panel_geometry_deserializes_without_dimensions() -> Result<(), Box<dyn Error>> {
    let config: EditorConfig =
        from_str(r#"(panel_positions: { "Test panel": (x: 12.0, y: 34.0) })"#)?;

    assert_eq!(
        config
            .panel_positions
            .get("Test panel")
            .map(|saved| (saved.x, saved.y, saved.width, saved.height)),
        Some((12.0, 34.0, None, None))
    );
    Ok(())
}
