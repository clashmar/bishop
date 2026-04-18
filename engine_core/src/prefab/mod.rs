mod capture;
mod component_sync;
mod instance;

use crate::ecs::capture::ComponentSnapshot;
use crate::onscreen_error;
use crate::storage::path_utils::{resources_folder, sanitise_name};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs;
use std::io;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

pub use crate::ecs::components::prefab_instance::{
    PrefabInstanceNode, PrefabInstanceRoot, PrefabOverrides,
};
pub use capture::{capture_prefab, capture_prefab_with_existing};
pub use instance::{instantiate_prefab, refresh_prefab_instance};

const PREFABS_FOLDER_NAME: &str = "prefabs";

/// Opaque handle for a persisted prefab asset.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
pub struct PrefabId(pub usize);

impl Display for PrefabId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.0.fmt(f)
    }
}

/// Project-wide prefab library persisted as individual prefab files.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PrefabLibrary {
    /// Prefabs keyed by their stable asset id.
    pub prefabs: HashMap<PrefabId, PrefabAsset>,
    /// Prefabs keyed by their runtime Lua name.
    #[serde(skip)]
    pub prefab_ids_by_name: HashMap<String, PrefabId>,
    /// Next available prefab id for this game.
    pub next_prefab_id: usize,
}

impl Default for PrefabLibrary {
    fn default() -> Self {
        Self {
            prefabs: HashMap::new(),
            prefab_ids_by_name: HashMap::new(),
            next_prefab_id: 1,
        }
    }
}

impl PrefabLibrary {
    /// Allocates the next project-scoped prefab id.
    pub fn allocate_prefab_id(&mut self) -> PrefabId {
        let id = PrefabId(self.next_prefab_id.max(1));
        self.next_prefab_id = id.0 + 1;
        id
    }

    fn restore_next_prefab_id(&mut self) {
        self.next_prefab_id = self
            .prefabs
            .keys()
            .map(|id| id.0)
            .max()
            .map(|max_id| max_id + 1)
            .unwrap_or(1);
    }

    fn rebuild_name_lookup(&mut self) -> io::Result<()> {
        self.prefab_ids_by_name.clear();

        for prefab in self.prefabs.values() {
            if let Some(existing_id) = self.prefab_ids_by_name.insert(prefab.name.clone(), prefab.id)
            {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "Duplicate prefab name '{}' for ids '{}' and '{}'",
                        prefab.name, existing_id, prefab.id
                    ),
                ));
            }
        }

        Ok(())
    }

    /// Returns a prefab by its runtime name.
    pub fn prefab_named(&self, name: &str) -> Option<&PrefabAsset> {
        self.prefab_ids_by_name
            .get(name)
            .and_then(|id| self.prefabs.get(id))
    }
}

/// Serializable prefab asset with stable node identifiers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrefabAsset {
    /// Stable identifier for the prefab asset file.
    pub id: PrefabId,
    /// Human-readable display name.
    pub name: String,
    /// Next available stable node identifier.
    pub next_node_id: usize,
    /// Root node identifier for the prefab hierarchy.
    pub root_node_id: usize,
    /// Flat list of prefab nodes in the hierarchy.
    pub nodes: Vec<PrefabNode>,
}

/// Serializable prefab node with parent linkage by stable node id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrefabNode {
    /// Stable identifier for this node within the prefab.
    pub node_id: usize,
    /// Stable identifier for the parent node when present.
    pub parent_node_id: Option<usize>,
    /// Serialized component snapshots owned by this node.
    pub components: Vec<ComponentSnapshot>,
}

/// Creates a new empty prefab asset with a stable root node.
pub fn create_prefab(prefab_id: PrefabId, name: String) -> PrefabAsset {
    PrefabAsset {
        id: prefab_id,
        name,
        next_node_id: 2,
        root_node_id: 1,
        nodes: vec![PrefabNode {
            node_id: 1,
            parent_node_id: None,
            components: Vec::new(),
        }],
    }
}

/// Loads every prefab file for the supplied game into a single library.
pub fn load_prefab_library(game_name: &str) -> io::Result<PrefabLibrary> {
    let paths = prefab_paths_for_game(game_name)?;
    let mut prefabs = HashMap::new();
    for path in paths {
        match load_prefab_from_path(&path) {
            Ok(prefab) => {
                if prefabs.contains_key(&prefab.id) {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "Duplicate prefab id '{}' encountered while loading '{}'",
                            prefab.id,
                            path.display()
                        ),
                    ));
                }

                prefabs.insert(prefab.id, prefab);
            }
            Err(error) => {
                onscreen_error!("Failed to load prefab '{}': {error}", path.display());
            }
        }
    }

    let mut library = PrefabLibrary {
        prefabs,
        ..Default::default()
    };
    library.restore_next_prefab_id();
    library.rebuild_name_lookup()?;
    Ok(library)
}

/// Lists every prefab asset for the supplied game.
pub fn list_prefabs(game_name: &str) -> io::Result<Vec<PrefabAsset>> {
    let mut prefabs: Vec<_> = load_prefab_library(game_name)?
        .prefabs
        .into_values()
        .collect();
    prefabs.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(prefabs)
}

/// Loads a single prefab asset by id.
pub fn load_prefab(game_name: &str, prefab_id: PrefabId) -> io::Result<PrefabAsset> {
    let path = find_prefab_path(game_name, prefab_id)?.ok_or_else(|| {
        Error::new(
            ErrorKind::NotFound,
            format!("Prefab '{prefab_id}' not found"),
        )
    })?;
    load_prefab_from_path(&path)
}

/// Saves a single prefab asset to disk.
pub fn save_prefab(game_name: &str, prefab: &PrefabAsset) -> io::Result<()> {
    let prefab = canonical_prefab_asset(prefab);
    validate_prefab(&prefab)?;
    let existing_path = find_prefab_path(game_name, prefab.id)?;
    ensure_unique_prefab_name(game_name, &prefab)?;
    let path = resolve_prefab_save_path(game_name, &prefab, existing_path.as_deref())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let ron =
        ron::ser::to_string_pretty(&prefab, ron::ser::PrettyConfig::new()).map_err(Error::other)?;

    fs::write(&path, ron)?;

    if let Some(existing_path) = existing_path
        && existing_path != path
        && existing_path.exists()
    {
        fs::remove_file(existing_path)?;
    }

    Ok(())
}

/// Returns the prefab asset in canonical save order.
pub fn canonical_prefab_asset(prefab: &PrefabAsset) -> PrefabAsset {
    let mut canonical = prefab.clone();
    for node in &mut canonical.nodes {
        node.components = canonical_component_snapshots(&node.components);
    }
    canonical.nodes.sort_by_key(|node| node.node_id);
    canonical
}

fn canonical_component_snapshots(components: &[ComponentSnapshot]) -> Vec<ComponentSnapshot> {
    let mut sorted = components.to_vec();
    sorted.sort_by(|left, right| left.type_name.cmp(&right.type_name));
    sorted
}

/// Deletes a single prefab asset file when it exists.
pub fn delete_prefab(game_name: &str, prefab_id: PrefabId) -> io::Result<bool> {
    let Some(path) = find_prefab_path(game_name, prefab_id)? else {
        return Ok(false);
    };

    fs::remove_file(path)?;
    Ok(true)
}

/// Validates prefab graph integrity before runtime/editor use.
pub fn validate_prefab(prefab: &PrefabAsset) -> io::Result<()> {
    if prefab.id.0 == 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Prefab '{}' cannot use id 0", prefab.name),
        ));
    }

    let mut node_ids = HashSet::new();
    let all_node_ids = prefab
        .nodes
        .iter()
        .map(|node| node.node_id)
        .collect::<HashSet<_>>();
    let root_node = prefab
        .nodes
        .iter()
        .find(|node| node.node_id == prefab.root_node_id);

    if root_node.is_none() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Prefab '{}' is missing its root node", prefab.name),
        ));
    }

    if root_node.and_then(|node| node.parent_node_id).is_some() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Prefab '{}' root node cannot have a parent", prefab.name),
        ));
    }

    for node in &prefab.nodes {
        if !node_ids.insert(node.node_id) {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Prefab '{}' contains duplicate node id {}",
                    prefab.name, node.node_id
                ),
            ));
        }

        if let Some(parent_node_id) = node.parent_node_id
            && (parent_node_id == node.node_id || !all_node_ids.contains(&parent_node_id))
        {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Prefab '{}' contains an invalid parent reference for node {}",
                    prefab.name, node.node_id
                ),
            ));
        }
    }

    let mut children_by_parent: HashMap<usize, Vec<usize>> = HashMap::new();
    for node in &prefab.nodes {
        if let Some(parent_node_id) = node.parent_node_id {
            children_by_parent
                .entry(parent_node_id)
                .or_default()
                .push(node.node_id);
        }
    }

    let mut visited = HashSet::new();
    let mut visiting = HashSet::new();
    validate_prefab_subtree(
        prefab.root_node_id,
        &children_by_parent,
        &mut visited,
        &mut visiting,
    )?;

    if visited.len() != prefab.nodes.len() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Prefab '{}' contains disconnected nodes", prefab.name),
        ));
    }

    Ok(())
}

fn validate_prefab_subtree(
    node_id: usize,
    children_by_parent: &HashMap<usize, Vec<usize>>,
    visited: &mut HashSet<usize>,
    visiting: &mut HashSet<usize>,
) -> io::Result<()> {
    if !visiting.insert(node_id) {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Prefab contains a cycle at node {node_id}"),
        ));
    }

    if let Some(children) = children_by_parent.get(&node_id) {
        for child_node_id in children {
            validate_prefab_subtree(*child_node_id, children_by_parent, visited, visiting)?;
        }
    }

    visiting.remove(&node_id);
    visited.insert(node_id);
    Ok(())
}

fn prefab_folder_for_game(game_name: &str) -> PathBuf {
    resources_folder(game_name).join(PREFABS_FOLDER_NAME)
}

fn prefab_path(game_name: &str, prefab_id: PrefabId) -> PathBuf {
    prefab_folder_for_game(game_name).join(format!("{}.ron", prefab_id.0))
}

fn prefab_name_stem(name: &str) -> String {
    let stem = sanitise_name(name);
    if stem.is_empty() {
        "Prefab".to_string()
    } else {
        stem
    }
}

fn prefab_paths_for_game(game_name: &str) -> io::Result<Vec<PathBuf>> {
    let folder = prefab_folder_for_game(game_name);
    if !folder.exists() {
        return Ok(Vec::new());
    }

    let mut paths = fs::read_dir(folder)?
        .filter_map(|entry| entry.ok().map(|value| value.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "ron"))
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

fn find_prefab_path(game_name: &str, prefab_id: PrefabId) -> io::Result<Option<PathBuf>> {
    let numeric_path = prefab_path(game_name, prefab_id);
    if numeric_path.exists() {
        return Ok(Some(numeric_path));
    }

    for path in prefab_paths_for_game(game_name)? {
        let Ok(prefab) = load_prefab_from_path(&path) else {
            continue;
        };
        if prefab.id == prefab_id {
            return Ok(Some(path));
        }
    }

    Ok(None)
}

fn resolve_prefab_save_path(
    game_name: &str,
    prefab: &PrefabAsset,
    existing_path: Option<&Path>,
) -> io::Result<PathBuf> {
    let folder = prefab_folder_for_game(game_name);
    if !folder.exists() {
        fs::create_dir_all(&folder)?;
    }

    let stem = prefab_name_stem(&prefab.name);
    let preferred = folder.join(format!("{stem}.ron"));
    if !preferred.exists() || existing_path == Some(preferred.as_path()) {
        return Ok(preferred);
    }

    let mut index = 2usize;
    loop {
        let candidate = folder.join(format!("{stem} {index}.ron"));
        if !candidate.exists() || existing_path == Some(candidate.as_path()) {
            return Ok(candidate);
        }
        index += 1;
    }
}

fn ensure_unique_prefab_name(game_name: &str, prefab: &PrefabAsset) -> io::Result<()> {
    for path in prefab_paths_for_game(game_name)? {
        let Ok(existing_prefab) = load_prefab_from_path(&path) else {
            continue;
        };

        if existing_prefab.id != prefab.id && existing_prefab.name == prefab.name {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                format!(
                    "Prefab name '{}' is already used by prefab '{}'",
                    prefab.name, existing_prefab.id
                ),
            ));
        }
    }

    Ok(())
}

fn load_prefab_from_path(path: &Path) -> io::Result<PrefabAsset> {
    let ron = fs::read_to_string(path)?;
    let prefab = ron::from_str(&ron).map_err(|error| {
        Error::new(
            ErrorKind::InvalidData,
            format!("Could not parse prefab '{}': {error}", path.display()),
        )
    })?;
    validate_prefab(&prefab)?;
    Ok(prefab)
}

#[cfg(test)]
#[path = "tests/prefab_module_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod runtime_tests;
