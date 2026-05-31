use std::io;

use crate::save_system::SavedSection;

/// A typed identifier for a save provider, wrapping a unique string key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SaveProviderId(String);

impl SaveProviderId {
    /// Creates a new `SaveProviderId` from any string-like value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the underlying string as a borrowed `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Phase at which a save provider's data is restored during runtime boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RestorePhase {
    /// Restored before Lua runtime is initialized.
    PreRuntime,
    /// Restored after Lua runtime is initialized.
    PostRuntime,
}

/// Captures and applies a section of save data for a [`SaveProviderId`] at a given [`RestorePhase`].
pub trait SaveProvider {
    /// Unique identifier for this provider.
    fn id(&self) -> SaveProviderId;

    /// The restore phase at which this provider's data should be applied.
    fn restore_phase(&self) -> RestorePhase;

    /// Captures current state into a [`SavedSection`].
    fn capture(&mut self) -> io::Result<SavedSection>;

    /// Applies a previously captured [`SavedSection`] to restore state.
    fn apply(&mut self, section: &SavedSection) -> io::Result<()>;
}
