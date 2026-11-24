//! Gamepad button mapping for controller support.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Gamepad button mapping for controller support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GamepadButton {
    South,
    East,
    North,
    West,
    LeftShoulder,
    RightShoulder,
    LeftTrigger,
    RightTrigger,
    Select,
    Start,
    LeftStick,
    RightStick,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

/// Gamepad axis whose values range from -1.0 to 1.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GamepadAxis {
    /// Horizontal axis of the left stick.
    LeftX,
    /// Vertical axis of the left stick.
    LeftY,
    /// Horizontal axis of the right stick.
    RightX,
    /// Vertical axis of the right stick.
    RightY,
    /// Left trigger axis.
    LeftTrigger,
    /// Right trigger axis.
    RightTrigger,
}

/// Axis values within this absolute magnitude are treated as neutral.
pub const GAMEPAD_AXIS_DEADZONE: f32 = 0.15;

/// Current gamepad button and axis state, including button edge transitions.
#[derive(Debug, Clone, Default)]
pub struct GamepadState {
    buttons_down: HashSet<GamepadButton>,
    buttons_pressed: HashSet<GamepadButton>,
    buttons_released: HashSet<GamepadButton>,
    axes: HashMap<GamepadAxis, f32>,
}

impl GamepadState {
    /// Clears per-frame button press and release edges.
    pub fn begin_frame(&mut self) {
        self.buttons_pressed.clear();
        self.buttons_released.clear();
    }

    /// Sets the current state of a gamepad button.
    pub fn set_button(&mut self, button: GamepadButton, down: bool) {
        let was_down = self.buttons_down.contains(&button);
        if was_down == down {
            return;
        }

        if down {
            self.buttons_down.insert(button);
            self.buttons_pressed.insert(button);
        } else {
            self.buttons_down.remove(&button);
            self.buttons_released.insert(button);
        }
    }

    /// Sets a normalized gamepad axis value, applying the shared deadzone.
    pub fn set_axis_value(&mut self, axis: GamepadAxis, value: f32) {
        let value = if value.is_finite() {
            value.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        let value = if value.abs() <= GAMEPAD_AXIS_DEADZONE {
            0.0
        } else {
            value
        };
        self.axes.insert(axis, value);
    }

    /// Returns whether a gamepad button is currently held.
    pub fn button_down(&self, button: GamepadButton) -> bool {
        self.buttons_down.contains(&button)
    }

    /// Returns whether a gamepad button was pressed this frame.
    pub fn button_pressed(&self, button: GamepadButton) -> bool {
        self.buttons_pressed.contains(&button)
    }

    /// Returns whether a gamepad button was released this frame.
    pub fn button_released(&self, button: GamepadButton) -> bool {
        self.buttons_released.contains(&button)
    }

    /// Returns the current normalized value of a gamepad axis.
    pub fn axis_value(&self, axis: GamepadAxis) -> f32 {
        self.axes.get(&axis).copied().unwrap_or(0.0)
    }

    /// Clears all held controls and reports releases for held buttons.
    pub fn disconnect(&mut self) {
        self.buttons_released.extend(self.buttons_down.iter().copied());
        self.buttons_down.clear();
        self.buttons_pressed.clear();
        self.axes.clear();
    }
}

#[cfg(test)]
mod tests;
