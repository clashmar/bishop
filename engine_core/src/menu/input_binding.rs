use bishop::prelude::*;
use serde::{Deserialize, Serialize};

/// Platform-aware input configuration for menu actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputBinding {
    pub keyboard: Option<KeyCode>,
    pub keyboard_alt: Option<KeyCode>,
    pub gamepad: Option<GamepadButton>,
}

impl InputBinding {
    /// Creates a new input binding with primary keyboard key.
    pub fn keyboard(key: KeyCode) -> Self {
        Self {
            keyboard: Some(key),
            keyboard_alt: None,
            gamepad: None,
        }
    }

    /// Creates a new input binding with primary and alternate keyboard keys.
    pub fn keyboard_with_alt(key: KeyCode, alt: KeyCode) -> Self {
        Self {
            keyboard: Some(key),
            keyboard_alt: Some(alt),
            gamepad: None,
        }
    }

    /// Checks if this binding is currently pressed.
    pub fn is_pressed<C: BishopContext>(&self, ctx: &C) -> bool {
        if let Some(key) = self.keyboard
            && ctx.is_key_pressed(key)
        {
            return true;
        }

        if let Some(alt) = self.keyboard_alt
            && ctx.is_key_pressed(alt)
        {
            return true;
        }
        false
    }

    /// Checks if this binding is currently down.
    pub fn is_down<C: BishopContext>(&self, ctx: &C) -> bool {
        if let Some(key) = self.keyboard
            && ctx.is_key_down(key)
        {
            return true;
        }

        if let Some(alt) = self.keyboard_alt
            && ctx.is_key_down(alt)
        {
            return true;
        }
        false
    }
}
