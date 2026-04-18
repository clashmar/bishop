use super::{AssetKey, AssetKind, AssetRecord};
use crate::assets::AssetManager;
use crate::constants::paths::{ASSETS_FOLDER, PREFABS_FOLDER, SCRIPTS_FOLDER};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Error, ErrorKind};
use std::path::{Component, Path, PathBuf};

/// Project-scoped registry of authored assets keyed by stable typed ids.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AssetRegistry {
    #[serde(
        serialize_with = "crate::storage::ordered_map::serialize",
        deserialize_with = "crate::storage::ordered_map::deserialize"
    )]
    /// Persisted authored assets keyed by their stable typed ids.
    records: HashMap<AssetKey, AssetRecord>,
    #[serde(skip)]
    path_to_key: HashMap<PathBuf, AssetKey>,
}

impl AssetRegistry {
    /// Rebuilds derived lookup metadata after deserialize or staged merge.
    pub fn init_editor_metadata(&mut self) {
        self.try_init_editor_metadata()
            .unwrap_or_else(|error| panic!("Invalid asset registry metadata: {error}"));
    }

    /// Rebuilds derived lookup metadata after deserialize or staged merge.
    pub fn try_init_editor_metadata(&mut self) -> io::Result<()> {
        self.path_to_key = self.rebuild_path_lookup()?;
        Ok(())
    }

    fn rebuild_path_lookup(&self) -> io::Result<HashMap<PathBuf, AssetKey>> {
        let mut path_to_key = HashMap::with_capacity(self.records.len());
        for (&key, record) in &self.records {
            let expected_kind = Self::kind_for_key(key);
            if record.kind != expected_kind {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "Asset registry record '{key:?}' requires kind '{expected_kind:?}', got '{:?}'",
                        record.kind
                    ),
                ));
            }

            Self::validate_asset_path(expected_kind, &record.path).map_err(|error| {
                Error::new(
                    error.kind(),
                    format!("Invalid asset registry record '{key:?}': {error}"),
                )
            })?;

            if let Some(existing_key) = path_to_key.insert(record.path.clone(), key) {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "Asset registry path '{}' maps to both '{existing_key:?}' and '{key:?}'",
                        record.path.display()
                    ),
                ));
            }
        }

        Ok(path_to_key)
    }

    /// Returns the persisted record for a key.
    pub fn record(&self, key: AssetKey) -> Option<&AssetRecord> {
        self.records.get(&key)
    }

    /// Returns all persisted asset records.
    pub fn records(&self) -> &HashMap<AssetKey, AssetRecord> {
        &self.records
    }

    #[cfg(test)]
    pub(crate) fn records_mut_for_test(&mut self) -> &mut HashMap<AssetKey, AssetRecord> {
        &mut self.records
    }

    /// Returns the asset key registered for a project-relative path.
    pub fn key_for_path<P: AsRef<Path>>(&self, path: P) -> Option<AssetKey> {
        self.path_to_key.get(path.as_ref()).copied()
    }

    /// Registers an asset path relative to the folder for `key`.
    pub fn register_asset_relative_path<K: Into<AssetKey>, P: AsRef<Path>>(
        &mut self,
        key: K,
        path: P,
    ) -> io::Result<()> {
        let key = key.into();
        let kind = Self::kind_for_key(key);
        let path = Self::canonical_asset_path(kind, path.as_ref())?;

        if let Some(existing_key) = self.existing_key_for_path(&path) {
            return match existing_key {
                existing if existing == key => Ok(()),
                _ => Err(Self::path_conflict_error(&path, existing_key, key)),
            };
        }

        self.insert(key, AssetRecord::new(kind, path))
    }

    /// Returns the asset path relative to the folder for `key`.
    pub fn relative_path<K: Into<AssetKey>>(&self, key: K) -> Option<PathBuf> {
        let key = key.into();
        let folder = Self::asset_folder(Self::kind_for_key(key));
        self.record(key)
            .and_then(|record| record.path.strip_prefix(&folder).ok())
            .map(Path::to_path_buf)
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

        Self::validate_asset_path(expected_kind, &record.path)?;

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

    fn path_conflict_error(path: &Path, existing_key: AssetKey, key: AssetKey) -> Error {
        Error::new(
            ErrorKind::InvalidData,
            format!(
                "Asset path '{}' maps to both '{existing_key:?}' and '{key:?}'",
                path.display()
            ),
        )
    }

    fn asset_folder(kind: AssetKind) -> PathBuf {
        PathBuf::from(match kind {
            AssetKind::Sprite => ASSETS_FOLDER,
            AssetKind::Script => SCRIPTS_FOLDER,
            AssetKind::Prefab => PREFABS_FOLDER,
        })
    }

    fn canonical_asset_path(kind: AssetKind, relative_path: &Path) -> io::Result<PathBuf> {
        let normalized = Self::normalize_relative_path(kind, relative_path)?;
        if normalized != relative_path {
            let folder = Self::asset_folder(kind);
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "{kind:?} paths must use canonical spelling under '{}': '{}'",
                    folder.display(),
                    relative_path.display()
                ),
            ));
        }

        Ok(Self::asset_folder(kind).join(normalized))
    }

    fn validate_asset_path(kind: AssetKind, path: &Path) -> io::Result<()> {
        let folder = Self::asset_folder(kind);
        let relative = path.strip_prefix(&folder).map_err(|_| {
            Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "{kind:?} paths must live under '{}': '{}'",
                    folder.display(),
                    path.display()
                ),
            )
        })?;

        let normalized = Self::normalize_relative_path(kind, relative)?;
        if normalized != relative {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "{kind:?} paths must use canonical spelling under '{}': '{}'",
                    folder.display(),
                    path.display()
                ),
            ));
        }

        Ok(())
    }

    fn normalize_relative_path(kind: AssetKind, path: &Path) -> io::Result<PathBuf> {
        if path.as_os_str().is_empty() || path.is_absolute() {
            let folder = Self::asset_folder(kind);
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "{kind:?} paths must be clean relative paths under '{}': '{}'",
                    folder.display(),
                    path.display()
                ),
            ));
        }

        let mut normalized = PathBuf::new();
        for component in path.components() {
            let Component::Normal(segment) = component else {
                let folder = Self::asset_folder(kind);
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "{kind:?} paths must be clean relative paths under '{}': '{}'",
                        folder.display(),
                        path.display()
                    ),
                ));
            };

            normalized.push(segment);
        }

        Ok(normalized)
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
