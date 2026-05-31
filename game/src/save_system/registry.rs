use crate::save_system::{SaveProvider, SaveProviderId};
use std::cmp::Ordering;

/// Error returned when registering a provider with a duplicate id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveRegistrationError {
    /// The duplicate provider id.
    pub provider_id: SaveProviderId,
}

impl std::fmt::Display for SaveRegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "duplicate save provider id: {}", self.provider_id.as_str())
    }
}

impl std::error::Error for SaveRegistrationError {}

/// A registry of [`SaveProvider`]s sorted by [`RestorePhase`] then provider id.
/// Duplicate ids are rejected on registration.
pub struct SaveProviderRegistry<'a> {
    providers: Vec<Box<dyn SaveProvider + 'a>>,
}

impl<'a> SaveProviderRegistry<'a> {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Registers a provider, returning an error if its id is already registered.
    /// Insertion maintains sorted order.
    pub fn register(
        &mut self,
        provider: Box<dyn SaveProvider + 'a>,
    ) -> Result<(), SaveRegistrationError> {
        let id = provider.id();
        if self.providers.iter().any(|p| p.id() == id) {
            return Err(SaveRegistrationError { provider_id: id });
        }

        let phase = provider.restore_phase();
        let insert_idx = self.providers.iter().position(|p| {
            let p_phase = p.restore_phase();
            let p_id = p.id();
            match p_phase.cmp(&phase) {
                Ordering::Less => false,
                Ordering::Greater => true,
                Ordering::Equal => p_id > provider.id(),
            }
        });

        match insert_idx {
            Some(idx) => self.providers.insert(idx, provider),
            None => self.providers.push(provider),
        }
        Ok(())
    }

    /// Iterates over all providers in canonical order (phase then id).
    pub fn iter(&self) -> impl Iterator<Item = &(dyn SaveProvider + 'a)> {
        self.providers.iter().map(|b| b.as_ref())
    }

    /// Mutably iterates over all providers in canonical order (phase then id).
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut (dyn SaveProvider + 'a)> {
        self.providers.iter_mut().map(|b| b.as_mut())
    }
}

impl<'a> Default for SaveProviderRegistry<'a> {
    fn default() -> Self {
        Self::new()
    }
}
