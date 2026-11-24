use crate::input::{InputActionId, InputActionMap, InputBinding};
use super::{InputSettings, load_input_settings_from_path, save_input_settings_to_path};
use bishop::KeyCode;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use tempfile::TempDir;
use uuid::Uuid;

const ACTION_ID: &str = "jump";

fn binding(key: KeyCode) -> Vec<InputBinding> {
    vec![InputBinding::keyboard(key)]
}

#[test]
fn input_settings_default_to_no_overrides() {
    let settings = InputSettings::default();

    assert!(settings.global_overrides.is_empty());
    assert!(settings.per_game_overrides.is_empty());
}

#[test]
fn global_input_override_round_trips_through_a_file() -> Result<(), Box<dyn Error>> {
    let root = TempDir::new()?;
    let path = root.path().join("input_settings.ron");
    let settings = InputSettings {
        global_overrides: HashMap::from([(
            InputActionId::new(ACTION_ID),
            binding(KeyCode::Enter),
        )]),
        ..InputSettings::default()
    };

    save_input_settings_to_path(&settings, &path)?;
    let loaded = load_input_settings_from_path(&path);

    assert_eq!(loaded, settings);
    Ok(())
}

#[test]
fn per_game_input_override_round_trips_through_a_file() -> Result<(), Box<dyn Error>> {
    let root = TempDir::new()?;
    let path = root.path().join("input_settings.ron");
    let game_id = Uuid::new_v4();
    let settings = InputSettings {
        per_game_overrides: HashMap::from([(
            game_id.to_string(),
            HashMap::from([(InputActionId::new(ACTION_ID), binding(KeyCode::Enter))]),
        )]),
        ..InputSettings::default()
    };

    save_input_settings_to_path(&settings, &path)?;
    let loaded = load_input_settings_from_path(&path);

    assert_eq!(loaded, settings);
    Ok(())
}

#[test]
fn input_binding_resolution_prefers_per_game_then_global_then_authored_defaults() {
    let game_id = Uuid::new_v4();
    let action_id = InputActionId::new(ACTION_ID);
    let authored = InputActionMap {
        actions: HashMap::from([(action_id.clone(), binding(KeyCode::Space))]),
    };
    let mut settings = InputSettings {
        global_overrides: HashMap::from([(action_id.clone(), binding(KeyCode::Enter))]),
        ..InputSettings::default()
    };

    assert_eq!(
        settings.bindings_for(game_id, &action_id, &authored),
        binding(KeyCode::Enter),
    );

    settings
        .per_game_overrides
        .entry(game_id.to_string())
        .or_default()
        .insert(action_id.clone(), binding(KeyCode::Escape));
    assert_eq!(
        settings.bindings_for(game_id, &action_id, &authored),
        binding(KeyCode::Escape),
    );

    settings
        .per_game_overrides
        .entry(game_id.to_string())
        .or_default()
        .insert(action_id.clone(), Vec::new());
    assert!(settings.bindings_for(game_id, &action_id, &authored).is_empty());

    settings.per_game_overrides.clear();
    settings.global_overrides.clear();
    assert_eq!(
        settings.bindings_for(game_id, &action_id, &authored),
        binding(KeyCode::Space),
    );
}

#[test]
fn malformed_input_settings_fall_back_to_empty_settings() -> Result<(), Box<dyn Error>> {
    let root = TempDir::new()?;
    let path = root.path().join("input_settings.ron");
    fs::write(&path, "not valid RON")?;

    let loaded = load_input_settings_from_path(&path);

    assert_eq!(loaded, InputSettings::default());
    Ok(())
}
