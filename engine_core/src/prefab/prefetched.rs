use super::{
    AssetKey, AssetRegistry, PrefabAsset, PrefabId, PrefabManager, build_prefab_manager,
    prefab_folder_for_game, validate_prefab,
};
use crate::constants::{extensions, paths};
use crate::storage::ResourceSource;
use std::collections::{HashMap, HashSet};
use std::io::{self, Error, ErrorKind};
use std::path::{Path, PathBuf};

/// Loads staged prefab definitions strictly, without scanning the native filesystem.
pub fn load_prefab_manager_from_source(
    game_name: &str,
    asset_registry: &mut AssetRegistry,
    source: &ResourceSource,
) -> io::Result<PrefabManager> {
    let prefab_root = Path::new(paths::PREFABS_FOLDER);
    let paths = source.prefetched_files_under(prefab_root)?;
    let mut loaded = Vec::new();
    let mut loaded_ids = HashSet::new();
    let mut ids_by_path = HashMap::new();
    for relative in paths.into_iter().filter(|path| {
        path.extension().is_some_and(|extension| extension == extensions::PREFAB)
    }) {
        let bytes = source.read_required(&relative)?;
        let text = std::str::from_utf8(&bytes).map_err(|error| {
            Error::new(ErrorKind::InvalidData, format!("invalid UTF-8 in {}: {error}", relative.display()))
        })?;
        let prefab: PrefabAsset = ron::from_str(text).map_err(|error| {
            Error::new(ErrorKind::InvalidData, format!("invalid prefab {}: {error}", relative.display()))
        })?;
        validate_prefab(&prefab).map_err(|error| {
            Error::new(ErrorKind::InvalidData, format!("invalid prefab {}: {error}", relative.display()))
        })?;
        if !loaded_ids.insert(prefab.id) {
            return Err(Error::new(ErrorKind::InvalidData, format!(
                "duplicate prefab id {} in {}", prefab.id, relative.display()
            )));
        }
        ids_by_path.insert(relative.clone(), prefab.id);
        let nested = relative.strip_prefix(prefab_root).map_err(|error| {
            Error::new(ErrorKind::InvalidData, format!("prefab path {}: {error}", relative.display()))
        })?;
        let virtual_path = prefab_folder_for_game(game_name).join(nested);
        loaded.push((virtual_path, prefab));
    }

    let mut registered: Vec<(PrefabId, PathBuf)> = asset_registry.records().iter()
        .filter_map(|(key, record)| match key {
            AssetKey::Prefab(id) => Some((*id, record.path.clone())),
            _ => None,
        }).collect();
    registered.sort_by(|left, right| left.1.cmp(&right.1));
    for (id, path) in registered {
        match ids_by_path.get(&path) {
            Some(loaded_id) if *loaded_id == id => {}
            Some(loaded_id) => return Err(Error::new(ErrorKind::InvalidData, format!(
                "registered prefab {} points to {} containing prefab {}", id, path.display(), loaded_id
            ))),
            None => return Err(Error::new(ErrorKind::NotFound, format!(
                "registered prefab resource missing: {}", path.display()
            ))),
        }
    }
    build_prefab_manager(game_name, asset_registry, loaded)
}
