use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::io;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

/// Shared contract for editor-managed asset services that can be staged and merged.
pub trait AssetManager: Sized {
    /// Returns a stage-safe snapshot containing only persistent editor metadata.
    fn editor_metadata_snapshot(&self) -> Self;

    /// Merges staged editor metadata from `source` into `self`.
    fn merge_editor_metadata_from(&mut self, source: &Self) -> io::Result<()>;
}

/// Helper contract for asset managers backed by stable `id <-> path` registries.
pub trait IdPathAssetManager: AssetManager {
    /// Stable asset identifier stored in prefab or game metadata.
    type AssetId: Copy + Eq + Hash + Debug;

    /// Human-readable asset kind used in merge errors.
    fn asset_kind() -> &'static str;

    /// Immutable access to the persistent `id -> path` registry.
    fn id_to_path(&self) -> &HashMap<Self::AssetId, PathBuf>;

    /// Mutable access to the persistent `id -> path` registry.
    fn id_to_path_mut(&mut self) -> &mut HashMap<Self::AssetId, PathBuf>;

    /// Immutable access to the derived `path -> id` registry.
    fn path_to_id(&self) -> &HashMap<PathBuf, Self::AssetId>;

    /// Mutable access to the derived `path -> id` registry.
    fn path_to_id_mut(&mut self) -> &mut HashMap<PathBuf, Self::AssetId>;

    /// Rebuilds any derived editor metadata after registry changes.
    fn rebuild_editor_metadata(&mut self);

    /// Validates that `source` can be merged into `self` without conflicting ids or paths.
    fn validate_id_path_registry_merge(&self, source: &Self) -> io::Result<()> {
        validate_id_path_registry_merge(
            Self::asset_kind(),
            source.id_to_path(),
            self.id_to_path(),
            self.path_to_id(),
        )
    }

    /// Merges the persistent `id <-> path` registry from `source` into `self`.
    fn merge_id_path_registry_from(&mut self, source: &Self) -> io::Result<()> {
        self.validate_id_path_registry_merge(source)?;

        let entries: Vec<_> = source
            .id_to_path()
            .iter()
            .map(|(&asset_id, path)| (asset_id, path.clone()))
            .collect();

        for (asset_id, path) in entries {
            self.id_to_path_mut().insert(asset_id, path.clone());
            self.path_to_id_mut().insert(path, asset_id);
        }

        self.rebuild_editor_metadata();
        Ok(())
    }
}

fn validate_id_path_registry_merge<AssetId>(
    asset_kind: &str,
    source_id_to_path: &HashMap<AssetId, PathBuf>,
    destination_id_to_path: &HashMap<AssetId, PathBuf>,
    destination_path_to_id: &HashMap<PathBuf, AssetId>,
) -> io::Result<()>
where
    AssetId: Copy + Eq + Hash + Debug,
{
    for (&asset_id, path) in source_id_to_path {
        if let Some(existing_path) = destination_id_to_path.get(&asset_id)
            && existing_path != path
        {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "{asset_kind} id '{asset_id:?}' maps to both '{}' and '{}'",
                    existing_path.display(),
                    path.display()
                ),
            ));
        }

        if let Some(existing_id) = destination_path_to_id.get(path)
            && *existing_id != asset_id
        {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "{asset_kind} path '{}' maps to both '{existing_id:?}' and '{asset_id:?}'",
                    path.display()
                ),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    struct FakeAssetId(usize);

    #[derive(Clone, Debug, Default, PartialEq)]
    struct FakeAssetManager {
        id_to_path: HashMap<FakeAssetId, PathBuf>,
        path_to_id: HashMap<PathBuf, FakeAssetId>,
        rebuild_calls: usize,
    }

    impl AssetManager for FakeAssetManager {
        fn editor_metadata_snapshot(&self) -> Self {
            self.clone()
        }

        fn merge_editor_metadata_from(&mut self, source: &Self) -> io::Result<()> {
            self.merge_id_path_registry_from(source)
        }
    }

    impl IdPathAssetManager for FakeAssetManager {
        type AssetId = FakeAssetId;

        fn asset_kind() -> &'static str {
            "FakeAsset"
        }

        fn id_to_path(&self) -> &HashMap<Self::AssetId, PathBuf> {
            &self.id_to_path
        }

        fn id_to_path_mut(&mut self) -> &mut HashMap<Self::AssetId, PathBuf> {
            &mut self.id_to_path
        }

        fn path_to_id(&self) -> &HashMap<PathBuf, Self::AssetId> {
            &self.path_to_id
        }

        fn path_to_id_mut(&mut self) -> &mut HashMap<PathBuf, Self::AssetId> {
            &mut self.path_to_id
        }

        fn rebuild_editor_metadata(&mut self) {
            self.rebuild_calls += 1;
        }
    }

    fn fake_manager(entries: &[(usize, &str)]) -> FakeAssetManager {
        let mut manager = FakeAssetManager::default();
        for (asset_id, path) in entries {
            let asset_id = FakeAssetId(*asset_id);
            let path = PathBuf::from(path);
            manager.id_to_path.insert(asset_id, path.clone());
            manager.path_to_id.insert(path, asset_id);
        }
        manager
    }

    #[test]
    fn merge_editor_metadata_from_accepts_matching_registry_entries() {
        let mut destination = fake_manager(&[(1, "sprites/player.png")]);
        let source = fake_manager(&[(1, "sprites/player.png"), (2, "sprites/tree.png")]);

        destination
            .merge_editor_metadata_from(&source)
            .expect("merge should succeed");

        assert_eq!(
            destination.id_to_path.get(&FakeAssetId(2)),
            Some(&PathBuf::from("sprites/tree.png"))
        );
        assert_eq!(
            destination.path_to_id.get(&PathBuf::from("sprites/tree.png")),
            Some(&FakeAssetId(2))
        );
        assert_eq!(destination.rebuild_calls, 1);
    }

    #[test]
    fn merge_editor_metadata_from_rejects_conflicting_paths_for_same_id() {
        let mut destination = fake_manager(&[(1, "sprites/player.png")]);
        let source = fake_manager(&[(1, "sprites/enemy.png")]);
        let before = destination.clone();

        let error = destination
            .merge_editor_metadata_from(&source)
            .expect_err("merge should fail");

        assert_eq!(error.kind(), ErrorKind::InvalidData);
        assert_eq!(destination, before);
    }

    #[test]
    fn merge_editor_metadata_from_rejects_conflicting_ids_for_same_path() {
        let mut destination = fake_manager(&[(1, "sprites/player.png")]);
        let source = fake_manager(&[(2, "sprites/player.png")]);
        let before = destination.clone();

        let error = destination
            .merge_editor_metadata_from(&source)
            .expect_err("merge should fail");

        assert_eq!(error.kind(), ErrorKind::InvalidData);
        assert_eq!(destination, before);
    }
}
