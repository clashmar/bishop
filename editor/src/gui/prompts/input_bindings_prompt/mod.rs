mod prompt;
mod state;

#[cfg(test)]
mod tests;

/// Authored action and player-setting changes staged by the input prompt.
pub struct InputBindingsChanges {
    /// Updated game-authored action map, if it changed.
    pub input_actions: Option<engine_core::input::InputActionMap>,
    /// Updated player input settings, if they changed.
    pub input_settings: Option<engine_core::storage::InputSettings>,
}

/// Result returned when the input bindings prompt is confirmed or cancelled.
pub enum InputBindingsPromptResult {
    /// Save the staged action and player-setting changes.
    Save(InputBindingsChanges),
    /// Discard the staged changes.
    Cancel,
}

pub(crate) use prompt::InputBindingsPrompt;

#[cfg(test)]
pub(crate) use state::{BindingScope, InputBindingsPromptState};

#[cfg(test)]
pub(crate) use prompt::{
    axis_capture_binding, gamepad_button_capture_binding, key_capture_binding,
};
