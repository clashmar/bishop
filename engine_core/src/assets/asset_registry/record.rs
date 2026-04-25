use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Persisted metadata for a single authored asset.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRecord {
    /// Canonical project-relative path for the asset.
    pub path: PathBuf,
}

impl AssetRecord {
    /// Creates a persisted asset record.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}
