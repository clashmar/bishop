use super::{GAMEPAD_AXIS_DEADZONE, GamepadAxis, GamepadButton, GamepadState};

#[test]
fn gamepad_button_tracks_down_pressed_and_released_edges() {
    let mut state = GamepadState::default();

    state.set_button(GamepadButton::South, true);

    assert!(state.button_down(GamepadButton::South));
    assert!(state.button_pressed(GamepadButton::South));
    assert!(!state.button_released(GamepadButton::South));

    state.begin_frame();
    assert!(state.button_down(GamepadButton::South));
    assert!(!state.button_pressed(GamepadButton::South));

    state.set_button(GamepadButton::South, false);
    assert!(!state.button_down(GamepadButton::South));
    assert!(state.button_released(GamepadButton::South));
}

#[test]
fn gamepad_axis_preserves_signed_normalized_values() {
    let mut state = GamepadState::default();

    state.set_axis_value(GamepadAxis::LeftX, 0.75);
    state.set_axis_value(GamepadAxis::LeftY, -0.5);

    assert_eq!(state.axis_value(GamepadAxis::LeftX), 0.75);
    assert_eq!(state.axis_value(GamepadAxis::LeftY), -0.5);
}

#[test]
fn gamepad_axis_values_inside_deadzone_become_neutral() {
    let mut state = GamepadState::default();

    state.set_axis_value(GamepadAxis::LeftX, GAMEPAD_AXIS_DEADZONE / 2.0);
    state.set_axis_value(GamepadAxis::LeftY, -GAMEPAD_AXIS_DEADZONE / 2.0);

    assert_eq!(state.axis_value(GamepadAxis::LeftX), 0.0);
    assert_eq!(state.axis_value(GamepadAxis::LeftY), 0.0);
}

#[test]
fn gamepad_disconnect_clears_down_buttons_and_axes() {
    let mut state = GamepadState::default();
    state.set_button(GamepadButton::South, true);
    state.set_axis_value(GamepadAxis::LeftX, 0.8);

    state.disconnect();

    assert!(!state.button_down(GamepadButton::South));
    assert!(state.button_released(GamepadButton::South));
    assert_eq!(state.axis_value(GamepadAxis::LeftX), 0.0);
}
