use super::AssetKind;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Persisted metadata for a single authored asset.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRecord {
    /// Persisted asset kind for the record.
    pub kind: AssetKind,
    /// Canonical project-relative path for the asset.
    pub path: PathBuf,
}

impl AssetRecord {
    /// Creates a persisted asset record.
    pub fn new(kind: AssetKind, path: PathBuf) -> Self {
        Self { kind, path }
    }
}
