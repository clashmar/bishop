use std::collections::HashMap;
use std::io;

use crate::save_system::{SaveProviderId, SaveProviderRegistry, RuntimeSaveDocument, RuntimeSaveMetadata, SavedSection};

/// Errors that can occur during a coordinated save or restore.
#[derive(Debug)]
pub enum SaveCoordinatorError {
    /// A provider failed during the capture phase.
    Capture {
        /// The provider that failed.
        provider_id: SaveProviderId,
        /// The underlying I/O error.
        source: io::Error,
    },
    /// A provider failed during the apply phase.
    Apply {
        /// The provider that failed.
        provider_id: SaveProviderId,
        /// The underlying I/O error.
        source: io::Error,
    },
}

impl std::fmt::Display for SaveCoordinatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capture {
                provider_id,
                source,
            } => {
                write!(
                    f,
                    "capture failed for provider '{}': {}",
                    provider_id.as_str(),
                    source
                )
            }
            Self::Apply {
                provider_id,
                source,
            } => {
                write!(
                    f,
                    "apply failed for provider '{}': {}",
                    provider_id.as_str(),
                    source
                )
            }
        }
    }
}

impl std::error::Error for SaveCoordinatorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Capture { source, .. } => Some(source),
            Self::Apply { source, .. } => Some(source),
        }
    }
}

/// Report produced by an apply operation.
#[derive(Debug, Default)]
pub struct ApplySaveReport {
    /// Section ids present in the document that have no registered provider.
    /// Sorted deterministically for stable output.
    pub unknown_section_ids: Vec<String>,
}

/// Captures state from all providers into a [`RuntimeSaveDocument`].
/// On failure, the error identifies the failing provider.
pub fn capture_document(
    registry: &mut SaveProviderRegistry<'_>,
    metadata: &RuntimeSaveMetadata,
) -> Result<RuntimeSaveDocument, SaveCoordinatorError> {
    let mut sections: HashMap<String, SavedSection> = HashMap::new();

    for provider in registry.iter_mut() {
        let id = provider.id();
        let section = provider.capture().map_err(|source| {
            SaveCoordinatorError::Capture {
                provider_id: id,
                source,
            }
        })?;
        sections.insert(id.as_str().to_string(), section);
    }

    Ok(RuntimeSaveDocument {
        metadata: metadata.clone(),
        sections,
    })
}

/// Applies saved sections from a [`RuntimeSaveDocument`] in registry order.
///
/// Unknown section ids are reported in [`ApplySaveReport::unknown_section_ids`].
pub fn apply_document(
    registry: &mut SaveProviderRegistry<'_>,
    document: &RuntimeSaveDocument,
) -> Result<ApplySaveReport, SaveCoordinatorError> {
    let mut report = ApplySaveReport::default();

    // Collect unknown section ids (document keys with no registered provider)
    for section_key in document.sections.keys() {
        let has_provider = registry.iter().any(|p| p.id().as_str() == section_key);
        if !has_provider {
            report.unknown_section_ids.push(section_key.clone());
        }
    }
    report.unknown_section_ids.sort();

    // Apply providers in canonical registry order
    for provider in registry.iter_mut() {
        let section_key = provider.id().as_str();
        if let Some(section) = document.sections.get(section_key) {
            provider.apply(section).map_err(|source| {
                SaveCoordinatorError::Apply {
                    provider_id: provider.id(),
                    source,
                }
            })?;
        }
    }

    Ok(report)
}
