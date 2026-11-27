use super::input_binding::InputBinding;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(test)]
mod tests;

/// The identifier used by game code to query an input action.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InputActionId(String);

impl InputActionId {
    /// Creates an action identifier from its game-defined name.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the game-defined action name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The game-authored default bindings for named input actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InputActionMap {
    /// Bindings keyed by game-defined action identifier.
    pub actions: HashMap<InputActionId, Vec<InputBinding>>,
}
