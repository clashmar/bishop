use super::{AssetKey, AssetKind, AssetRecord};
use crate::assets::AssetManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Error, ErrorKind};
use std::path::{Path, PathBuf};

/// Project-scoped registry of authored assets keyed by stable typed ids.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AssetRegistry {
    #[serde(
        serialize_with = "crate::storage::ordered_map::serialize",
        deserialize_with = "crate::storage::ordered_map::deserialize"
    )]
    /// Persisted authored assets keyed by their stable typed ids.
    pub records: HashMap<AssetKey, AssetRecord>,
    #[serde(skip)]
    path_to_key: HashMap<PathBuf, AssetKey>,
}

impl AssetRegistry {
    /// Rebuilds derived lookup metadata after deserialize or staged merge.
    pub fn init_editor_metadata(&mut self) {
        self.path_to_key.clear();
        for (&key, record) in &self.records {
            self.path_to_key.insert(record.path.clone(), key);
        }
    }

    /// Returns the persisted record for a key.
    pub fn record(&self, key: AssetKey) -> Option<&AssetRecord> {
        self.records.get(&key)
    }

    /// Returns the asset key registered for a project-relative path.
    pub fn key_for_path<P: AsRef<Path>>(&self, path: P) -> Option<AssetKey> {
        self.path_to_key.get(path.as_ref()).copied()
    }

    /// Inserts one record, rejecting conflicting keys and conflicting paths.
    pub fn insert(&mut self, key: AssetKey, record: AssetRecord) -> io::Result<()> {
        let expected_kind = Self::kind_for_key(key);
        if record.kind != expected_kind {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Asset key '{key:?}' requires kind '{expected_kind:?}', got '{:?}'",
                    record.kind
                ),
            ));
        }

        if let Some(existing) = self.records.get(&key)
            && existing != &record
        {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("Asset key '{key:?}' maps to multiple records"),
            ));
        }

        if let Some(existing_key) = self.existing_key_for_path(&record.path)
            && existing_key != key
        {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Asset path '{}' maps to both '{existing_key:?}' and '{key:?}'",
                    record.path.display()
                ),
            ));
        }

        self.path_to_key.insert(record.path.clone(), key);
        self.records.insert(key, record);
        Ok(())
    }

    fn existing_key_for_path(&self, path: &Path) -> Option<AssetKey> {
        self.path_to_key.get(path).copied().or_else(|| {
            self.records.iter().find_map(|(&key, record)| {
                if record.path == path {
                    Some(key)
                } else {
                    None
                }
            })
        })
    }

    fn kind_for_key(key: AssetKey) -> AssetKind {
        match key {
            AssetKey::Sprite(_) => AssetKind::Sprite,
            AssetKey::Script(_) => AssetKind::Script,
            AssetKey::Prefab(_) => AssetKind::Prefab,
        }
    }
}

impl AssetManager for AssetRegistry {
    fn editor_metadata_snapshot(&self) -> Self {
        let mut snapshot = Self {
            records: self.records.clone(),
            ..Default::default()
        };
        snapshot.init_editor_metadata();
        snapshot
    }

    fn merge_editor_metadata_from(&mut self, source: &Self) -> io::Result<()> {
        let mut merged = <Self as AssetManager>::editor_metadata_snapshot(self);
        for (&key, record) in &source.records {
            merged.insert(key, record.clone())?;
        }
        *self = merged;
        Ok(())
    }
}
