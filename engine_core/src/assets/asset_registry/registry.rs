use super::{registry_errors, AssetKey, AssetKind, AssetRecord};
use crate::assets::AssetManager;
use crate::constants::paths::{ASSETS_FOLDER, AUDIO_FOLDER, PREFABS_FOLDER, SCRIPTS_FOLDER, TEXT_FOLDER};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self};
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
                return Err(registry_errors::record_kind_mismatch(
                    key,
                    expected_kind,
                    record.kind,
                ));
            }

            Self::validate_asset_path(expected_kind, &record.path)
                .map_err(|error| registry_errors::invalid_record(key, error))?;

            if let Some(existing_key) = path_to_key.insert(record.path.clone(), key) {
                return Err(registry_errors::conflicting_registry_path(
                    &record.path,
                    existing_key,
                    key,
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
                _ => Err(registry_errors::conflicting_path(&path, existing_key, key)),
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
        self.validate_record_for_key(key, &record)?;

        if let Some(existing) = self.records.get(&key)
            && existing != &record
        {
            return Err(registry_errors::key_maps_to_multiple_records(key));
        }

        self.path_to_key.insert(record.path.clone(), key);
        self.records.insert(key, record);
        Ok(())
    }

    /// Replaces the record for a key, keeping path lookup metadata in sync.
    pub fn replace_record(&mut self, key: AssetKey, record: AssetRecord) -> io::Result<()> {
        self.validate_record_for_key(key, &record)?;

        if let Some(previous) = self.records.insert(key, record.clone()) {
            self.path_to_key.remove(&previous.path);
        }

        self.path_to_key.insert(record.path.clone(), key);
        Ok(())
    }

    /// Removes the record for a key and its derived path lookup entry.
    pub fn remove_record(&mut self, key: AssetKey) -> Option<AssetRecord> {
        let record = self.records.remove(&key)?;
        self.path_to_key.remove(&record.path);
        Some(record)
    }

    fn validate_record_for_key(&self, key: AssetKey, record: &AssetRecord) -> io::Result<()> {
        let expected_kind = Self::kind_for_key(key);
        if record.kind != expected_kind {
            return Err(registry_errors::key_kind_mismatch(
                key,
                expected_kind,
                record.kind,
            ));
        }

        Self::validate_asset_path(expected_kind, &record.path)?;

        if let Some(existing_key) = self.existing_key_for_path(&record.path)
            && existing_key != key
        {
            return Err(registry_errors::conflicting_path(
                &record.path,
                existing_key,
                key,
            ));
        }

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

    fn asset_folder(kind: AssetKind) -> PathBuf {
        PathBuf::from(match kind {
            AssetKind::Sprite => ASSETS_FOLDER,
            AssetKind::Script => SCRIPTS_FOLDER,
            AssetKind::Prefab => PREFABS_FOLDER,
            AssetKind::Sound => AUDIO_FOLDER,
            AssetKind::Toml => TEXT_FOLDER,
        })
    }

    fn canonical_asset_path(kind: AssetKind, relative_path: &Path) -> io::Result<PathBuf> {
        let folder = Self::asset_folder(kind);
        let normalized = Self::normalize_relative_path(kind, &folder, relative_path)?;
        kind.validate_extension(&normalized, &folder)?;
        if normalized != relative_path {
            return Err(registry_errors::canonical_spelling(
                kind,
                &folder,
                relative_path,
            ));
        }

        Ok(folder.join(normalized))
    }

    fn validate_asset_path(kind: AssetKind, path: &Path) -> io::Result<()> {
        let folder = Self::asset_folder(kind);
        let relative = path
            .strip_prefix(&folder)
            .map_err(|_| registry_errors::rooted_path(kind, &folder, path))?;

        let normalized = Self::normalize_relative_path(kind, &folder, relative)?;
        kind.validate_extension(&normalized, &folder)?;
        if normalized != relative {
            return Err(registry_errors::canonical_spelling(kind, &folder, path));
        }

        Ok(())
    }

    fn normalize_relative_path(kind: AssetKind, folder: &Path, path: &Path) -> io::Result<PathBuf> {
        if path.as_os_str().is_empty() || path.is_absolute() {
            return Err(registry_errors::clean_relative_path(kind, folder, path));
        }

        let mut normalized = PathBuf::new();
        for component in path.components() {
            let Component::Normal(segment) = component else {
                return Err(registry_errors::clean_relative_path(kind, folder, path));
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
            AssetKey::Sound(_) => AssetKind::Sound,
            AssetKey::Toml(_) => AssetKind::Toml,
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
