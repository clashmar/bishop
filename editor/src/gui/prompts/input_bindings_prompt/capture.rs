use bishop::input::{GamepadAxis, GamepadButton, Input, KeyCode};
use bishop::prelude::WgpuContext;
use engine_core::input::{AxisDirection, InputBinding, InputSource};

const AXIS_CAPTURE_THRESHOLD: f32 = 0.5;
const GAMEPAD_BUTTONS: [GamepadButton; 16] = [
    GamepadButton::South,
    GamepadButton::East,
    GamepadButton::North,
    GamepadButton::West,
    GamepadButton::LeftShoulder,
    GamepadButton::RightShoulder,
    GamepadButton::LeftTrigger,
    GamepadButton::RightTrigger,
    GamepadButton::Select,
    GamepadButton::Start,
    GamepadButton::LeftStick,
    GamepadButton::RightStick,
    GamepadButton::DPadUp,
    GamepadButton::DPadDown,
    GamepadButton::DPadLeft,
    GamepadButton::DPadRight,
];
const GAMEPAD_AXES: [GamepadAxis; 6] = [
    GamepadAxis::LeftX,
    GamepadAxis::LeftY,
    GamepadAxis::RightX,
    GamepadAxis::RightY,
    GamepadAxis::LeftTrigger,
    GamepadAxis::RightTrigger,
];

#[derive(Clone, Copy)]
pub(super) enum CaptureMode {
    Key,
    GamepadButton,
    GamepadAxis,
}

#[derive(Clone, Copy)]
pub(super) struct CaptureRequest {
    pub(super) mode: CaptureMode,
    pub(super) replacement_index: Option<usize>,
}

pub(super) fn binding_label(source: InputSource) -> String {
    match source {
        InputSource::Key(key) => format!("Key: {key:?}"),
        InputSource::GamepadButton(button) => format!("Button: {button:?}"),
        InputSource::GamepadAxis {
            axis, direction, ..
        } => format!(
            "Axis: {axis:?} {}",
            if direction == AxisDirection::Positive {
                "+"
            } else {
                "−"
            }
        ),
    }
}

pub(super) fn capture_mode_for(source: InputSource) -> CaptureMode {
    match source {
        InputSource::Key(_) => CaptureMode::Key,
        InputSource::GamepadButton(_) => CaptureMode::GamepadButton,
        InputSource::GamepadAxis { .. } => CaptureMode::GamepadAxis,
    }
}

pub(super) fn capture_binding(ctx: &WgpuContext, mode: CaptureMode) -> Option<InputBinding> {
    match mode {
        CaptureMode::Key => ctx.pressed_keys().into_iter().find_map(key_capture_binding),
        CaptureMode::GamepadButton => GAMEPAD_BUTTONS
            .into_iter()
            .find(|button| ctx.gamepad_state().button_pressed(*button))
            .map(gamepad_button_capture_binding),
        CaptureMode::GamepadAxis => GAMEPAD_AXES
            .into_iter()
            .filter_map(|axis| {
                let value = ctx.gamepad_state().axis_value(axis);
                axis_capture_binding(axis, value).map(|binding| (binding, value))
            })
            .max_by(|(_, left), (_, right)| left.abs().total_cmp(&right.abs()))
            .map(|(binding, _)| binding),
    }
}

pub(super) fn key_capture_binding(key: KeyCode) -> Option<InputBinding> {
    (key != KeyCode::Escape).then(|| InputBinding::keyboard(key))
}

pub(super) fn gamepad_button_capture_binding(button: GamepadButton) -> InputBinding {
    InputBinding {
        source: InputSource::GamepadButton(button),
    }
}

pub(super) fn axis_capture_binding(axis: GamepadAxis, value: f32) -> Option<InputBinding> {
    if !value.is_finite() || value.abs() < AXIS_CAPTURE_THRESHOLD {
        return None;
    }
    Some(InputBinding {
        source: InputSource::GamepadAxis {
            axis,
            direction: if value > 0.0 {
                AxisDirection::Positive
            } else {
                AxisDirection::Negative
            },
            threshold: AXIS_CAPTURE_THRESHOLD,
        },
    })
}
