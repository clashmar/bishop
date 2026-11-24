use bishop::{GamepadButton, KeyCode};
pub use bishop::GamepadAxis;
use serde::{Deserialize, Serialize};

/// A physical source that can activate an input action.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum InputSource {
    /// A keyboard key.
    Key(KeyCode),
    /// A gamepad button.
    GamepadButton(GamepadButton),
    /// A gamepad axis direction and activation threshold.
    GamepadAxis {
        /// The physical gamepad axis.
        axis: GamepadAxis,
        /// The direction that activates the binding.
        direction: AxisDirection,
        /// The minimum absolute axis value that activates the binding.
        threshold: f32,
    },
}

/// The direction in which a gamepad axis activates an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AxisDirection {
    /// The axis value is above zero.
    Positive,
    /// The axis value is below zero.
    Negative,
}

/// One physical input binding.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InputBinding {
    /// The physical input assigned to the action.
    pub source: InputSource,
}

impl InputBinding {
    /// Creates a keyboard binding.
    pub fn keyboard(key: KeyCode) -> Self {
        Self {
            source: InputSource::Key(key),
        }
    }
}
