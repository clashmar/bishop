use super::{InputSnapshot, PhysicalInputState};
use bishop::input::{GamepadAxis, GamepadButton, GamepadState, KeyCode};
use engine_core::input::{AxisDirection, InputActionId, InputActionMap, InputBinding, InputSource};
use engine_core::storage::InputSettings;
use std::collections::HashMap;
use uuid::Uuid;

const JUMP_ACTION_ID: &str = "jump";

fn binding(source: InputSource) -> InputBinding {
    InputBinding { source }
}

fn action_map(bindings: Vec<InputBinding>) -> InputActionMap {
    InputActionMap {
        actions: HashMap::from([(InputActionId::new(JUMP_ACTION_ID), bindings)]),
    }
}

#[test]
fn keyboard_action_tracks_down_pressed_and_released() {
    let action_id = InputActionId::new(JUMP_ACTION_ID);
    let actions = action_map(vec![binding(InputSource::Key(KeyCode::Space))]);
    let settings = InputSettings::default();
    let game_id = Uuid::new_v4();
    let mut physical = PhysicalInputState::default();
    let mut snapshot = InputSnapshot::default();

    physical.set_key(KeyCode::Space, true, true, false);
    snapshot.resolve_actions(&actions, &settings, game_id, &physical);
    assert!(snapshot.is_action_down(&action_id));
    assert!(snapshot.is_action_pressed(&action_id));
    assert!(!snapshot.is_action_released(&action_id));

    physical.set_key(KeyCode::Space, true, false, false);
    snapshot.resolve_actions(&actions, &settings, game_id, &physical);
    assert!(snapshot.is_action_down(&action_id));
    assert!(!snapshot.is_action_pressed(&action_id));

    physical.set_key(KeyCode::Space, false, false, true);
    snapshot.resolve_actions(&actions, &settings, game_id, &physical);
    assert!(!snapshot.is_action_down(&action_id));
    assert!(snapshot.is_action_released(&action_id));
}

#[test]
fn gamepad_button_and_positive_axis_activate_actions() {
    let jump = InputActionId::new(JUMP_ACTION_ID);
    let axis = InputActionId::new("move_right");
    let actions = InputActionMap {
        actions: HashMap::from([
            (
                jump.clone(),
                vec![binding(InputSource::GamepadButton(GamepadButton::South))],
            ),
            (
                axis.clone(),
                vec![binding(InputSource::GamepadAxis {
                    axis: GamepadAxis::LeftX,
                    direction: AxisDirection::Positive,
                    threshold: 0.5,
                })],
            ),
        ]),
    };
    let settings = InputSettings::default();
    let game_id = Uuid::new_v4();
    let mut physical = PhysicalInputState::default();
    let mut gamepad = GamepadState::default();
    gamepad.set_button(GamepadButton::South, true);
    gamepad.set_axis_value(GamepadAxis::LeftX, 0.75);
    physical.set_gamepad(gamepad.clone());

    let mut snapshot = InputSnapshot::default();
    snapshot.resolve_actions(&actions, &settings, game_id, &physical);

    assert!(snapshot.is_action_down(&jump));
    assert!(snapshot.is_action_pressed(&jump));
    assert!(snapshot.is_action_down(&axis));
    assert!(snapshot.is_action_pressed(&axis));

    gamepad.begin_frame();
    gamepad.set_button(GamepadButton::South, false);
    physical.set_gamepad(gamepad);
    snapshot.resolve_actions(&actions, &settings, game_id, &physical);
    assert!(!snapshot.is_action_down(&jump));
    assert!(snapshot.is_action_released(&jump));
}

#[test]
fn axis_action_does_not_activate_inside_the_shared_deadzone() {
    let action_id = InputActionId::new("move_right");
    let actions = InputActionMap {
        actions: HashMap::from([(
            action_id.clone(),
            vec![binding(InputSource::GamepadAxis {
                axis: GamepadAxis::LeftX,
                direction: AxisDirection::Positive,
                threshold: 0.0,
            })],
        )]),
    };
    let settings = InputSettings::default();
    let game_id = Uuid::new_v4();
    let mut gamepad = GamepadState::default();
    gamepad.set_axis_value(GamepadAxis::LeftX, 0.1);
    let mut physical = PhysicalInputState::default();
    physical.set_gamepad(gamepad);

    let mut snapshot = InputSnapshot::default();
    snapshot.resolve_actions(&actions, &settings, game_id, &physical);

    assert!(!snapshot.is_action_down(&action_id));
    assert!(!snapshot.is_action_pressed(&action_id));
}

#[test]
fn negative_axis_activates_negative_direction_and_releases_below_threshold() {
    let action_id = InputActionId::new("move_left");
    let actions = InputActionMap {
        actions: HashMap::from([(
            action_id.clone(),
            vec![binding(InputSource::GamepadAxis {
                axis: GamepadAxis::LeftX,
                direction: AxisDirection::Negative,
                threshold: 0.5,
            })],
        )]),
    };
    let settings = InputSettings::default();
    let game_id = Uuid::new_v4();
    let mut physical = PhysicalInputState::default();
    let mut gamepad = GamepadState::default();
    gamepad.set_axis_value(GamepadAxis::LeftX, -0.8);
    physical.set_gamepad(gamepad.clone());
    let mut snapshot = InputSnapshot::default();

    snapshot.resolve_actions(&actions, &settings, game_id, &physical);
    assert!(snapshot.is_action_down(&action_id));
    assert!(snapshot.is_action_pressed(&action_id));

    gamepad.set_axis_value(GamepadAxis::LeftX, -0.2);
    physical.set_gamepad(gamepad);
    snapshot.resolve_actions(&actions, &settings, game_id, &physical);
    assert!(!snapshot.is_action_down(&action_id));
    assert!(snapshot.is_action_released(&action_id));
}

#[test]
fn missing_actions_are_neutral_and_settings_precedence_is_respected() {
    let overridden = InputActionId::new(JUMP_ACTION_ID);
    let absent = InputActionId::new("not_defined");
    let actions = action_map(vec![binding(InputSource::Key(KeyCode::Space))]);
    let game_id = Uuid::new_v4();
    let settings = InputSettings {
        global_overrides: HashMap::from([(
            overridden.clone(),
            vec![binding(InputSource::Key(KeyCode::Enter))],
        )]),
        per_game_overrides: HashMap::from([(
            game_id.to_string(),
            HashMap::from([(overridden.clone(), Vec::new())]),
        )]),
    };
    let mut physical = PhysicalInputState::default();
    physical.set_key(KeyCode::Space, true, true, false);
    physical.set_key(KeyCode::Enter, true, true, false);

    let mut snapshot = InputSnapshot::default();
    snapshot.resolve_actions(&actions, &settings, game_id, &physical);
    assert!(!snapshot.is_action_down(&overridden));

    snapshot.resolve_actions(&actions, &settings, Uuid::new_v4(), &physical);
    assert!(snapshot.is_action_down(&overridden));

    snapshot.resolve_actions(&actions, &InputSettings::default(), game_id, &physical);
    assert!(snapshot.is_action_down(&overridden));
    assert!(!snapshot.is_action_down(&absent));
}
