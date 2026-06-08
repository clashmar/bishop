use std::collections::HashMap;
use std::io;

use crate::save_system::{
    RestorePhase, SaveProviderId, SaveProviderRegistry, RuntimeSaveDocument,
    RuntimeSaveMetadata, SavedSection,
};

/// Errors during coordinated save or restore.
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

/// Captures state from all providers into a `RuntimeSaveDocument`.
pub fn capture_document(
    registry: &mut SaveProviderRegistry<'_>,
    metadata: &RuntimeSaveMetadata,
) -> Result<RuntimeSaveDocument, SaveCoordinatorError> {
    let mut sections: HashMap<String, SavedSection> = HashMap::new();

    for provider in registry.iter_mut() {
        let id = provider.id();
        let section_key = id.as_str().to_string();
        let section = provider.capture().map_err(|source| {
            SaveCoordinatorError::Capture {
                provider_id: id,
                source,
            }
        })?;
        sections.insert(section_key, section);
    }

    Ok(RuntimeSaveDocument {
        metadata: metadata.clone(),
        sections,
    })
}

/// Applies all saved sections from a document in registry order.
pub fn apply_document(
    registry: &mut SaveProviderRegistry<'_>,
    document: &RuntimeSaveDocument,
) -> Result<ApplySaveReport, SaveCoordinatorError> {
    let unknown_section_ids = collect_unknown_sections(registry, document);

    for provider in registry.iter_mut() {
        let provider_id = provider.id();
        let section_key = provider_id.as_str();
        if let Some(section) = document.sections.get(section_key) {
            provider.apply(section).map_err(|source| {
                SaveCoordinatorError::Apply {
                    provider_id: provider.id(),
                    source,
                }
            })?;
        }
    }

    Ok(ApplySaveReport { unknown_section_ids })
}

/// Applies saved sections matching a given restore phase.
pub fn apply_document_phase(
    registry: &mut SaveProviderRegistry<'_>,
    document: &RuntimeSaveDocument,
    phase: RestorePhase,
) -> Result<ApplySaveReport, SaveCoordinatorError> {
    let unknown_section_ids = collect_unknown_sections(registry, document);

    for provider in registry.iter_mut() {
        if provider.restore_phase() != phase {
            continue;
        }
        let provider_id = provider.id();
        let section_key = provider_id.as_str();
        if let Some(section) = document.sections.get(section_key) {
            provider.apply(section).map_err(|source| {
                SaveCoordinatorError::Apply {
                    provider_id: provider.id(),
                    source,
                }
            })?;
        }
    }

    Ok(ApplySaveReport { unknown_section_ids })
}

/// Collects section keys from the document that have no matching provider.
fn collect_unknown_sections(
    registry: &SaveProviderRegistry<'_>,
    document: &RuntimeSaveDocument,
) -> Vec<String> {
    let mut unknown: Vec<String> = document
        .sections
        .keys()
        .filter(|section_key| !registry.iter().any(|p| p.id().as_str() == section_key.as_str()))
        .cloned()
        .collect();
    unknown.sort();
    unknown
}
