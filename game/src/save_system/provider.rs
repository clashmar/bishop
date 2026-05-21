use std::io;

use crate::save_system::SavedSection;

/// A typed identifier for a save provider, wrapping a unique `&'static str` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SaveProviderId(&'static str);

impl SaveProviderId {
    /// Creates a new `SaveProviderId` from a static string.
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    /// Returns the underlying static string.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

/// Phase at which a save provider's captured data is restored during runtime boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RestorePhase {
    /// Restored before Lua runtime is initialized (e.g. engine state like room resume).
    PreRuntime,
    /// Restored after Lua runtime is initialized (e.g. game state like player data).
    PostRuntime,
}

/// A provider that captures and applies a section of save data, identified by a unique
/// [`SaveProviderId`] and assigned to a [`RestorePhase`].
pub trait SaveProvider {
    /// Unique identifier for this provider.
    fn id(&self) -> SaveProviderId;

    /// The restore phase at which this provider's data should be applied.
    fn restore_phase(&self) -> RestorePhase;

    /// Capture the current state into a [`SavedSection`] for persistence.
    fn capture(&mut self) -> io::Result<SavedSection>;

    /// Apply a previously captured [`SavedSection`] to restore state.
    fn apply(&mut self, section: &SavedSection) -> io::Result<()>;
}
