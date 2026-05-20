mod constants;
mod document;
mod paths;

pub use constants::*;
pub use document::*;
pub use paths::*;

use engine_core::storage::sanitise_name;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaveLane {
    Manual,
    Autosave,
}

impl SaveLane {
    pub fn file_stem(self) -> &'static str {
        match self {
            Self::Manual => lane_stems::MANUAL,
            Self::Autosave => lane_stems::AUTOSAVE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SaveSlotKey {
    #[default]
    Default,
    Named(String),
}

impl SaveSlotKey {
    pub fn folder_name(&self) -> String {
        match self {
            Self::Default => DEFAULT_RUNTIME_SAVE_SLOT.to_string(),
            Self::Named(name) => sanitise_name(name),
        }
    }
}

#[cfg(test)]
mod tests;
