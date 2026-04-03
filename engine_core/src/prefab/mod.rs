use crate::ecs::capture::ComponentSnapshot;
use crate::onscreen_error;
use crate::storage::path_utils::resources_folder;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs;
use std::io;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

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
    /// Next available prefab id for this game.
    pub next_prefab_id: usize,
}

impl Default for PrefabLibrary {
    fn default() -> Self {
        Self {
            prefabs: HashMap::new(),
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
    let folder = prefab_folder_for_game(game_name);
    if !folder.exists() {
        return Ok(PrefabLibrary::default());
    }

    let mut paths = fs::read_dir(folder)?
        .filter_map(|entry| entry.ok().map(|value| value.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "ron"))
        .collect::<Vec<_>>();
    paths.sort();
    let mut prefabs = HashMap::new();
    for path in paths {
        match load_prefab_from_path(&path) {
            Ok(prefab) => {
                if prefabs.contains_key(&prefab.id) {
                    onscreen_error!(
                        "Skipping duplicate prefab id '{}' from '{}'",
                        prefab.id,
                        path.display()
                    );
                    continue;
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
    Ok(library)
}

/// Lists every prefab asset for the supplied game.
pub fn list_prefabs(game_name: &str) -> io::Result<Vec<PrefabAsset>> {
    let mut prefabs: Vec<_> = load_prefab_library(game_name)?.prefabs.into_values().collect();
    prefabs.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(prefabs)
}

/// Loads a single prefab asset by id.
pub fn load_prefab(game_name: &str, prefab_id: PrefabId) -> io::Result<PrefabAsset> {
    load_prefab_from_path(&prefab_path(game_name, prefab_id))
}

/// Saves a single prefab asset to disk.
pub fn save_prefab(game_name: &str, prefab: &PrefabAsset) -> io::Result<()> {
    let path = prefab_path(game_name, prefab.id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let ron =
        ron::ser::to_string_pretty(prefab, ron::ser::PrettyConfig::new()).map_err(Error::other)?;

    fs::write(path, ron)
}

/// Deletes a single prefab asset file when it exists.
pub fn delete_prefab(game_name: &str, prefab_id: PrefabId) -> io::Result<bool> {
    let path = prefab_path(game_name, prefab_id);
    if !path.exists() {
        return Ok(false);
    }

    fs::remove_file(path)?;
    Ok(true)
}

/// Validates prefab graph integrity before runtime/editor use.
pub fn validate_prefab(prefab: &PrefabAsset) -> io::Result<()> {
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
                    prefab.name,
                    node.node_id
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
                    prefab.name,
                    node.node_id
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
mod tests {
    use super::*;
    use crate::storage::test_utils::{TestGameFolder, game_fs_test_lock};

    #[test]
    fn load_prefab_library_skips_invalid_prefab_files() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let test_folder = TestGameFolder::new("prefab_partial_load");
        let valid = create_prefab(PrefabId(1), "Valid".to_string());

        save_prefab(test_folder.name(), &valid).unwrap();
        fs::write(
            prefab_folder_for_game(test_folder.name()).join("broken.ron"),
            "not valid ron",
        )
        .unwrap();

        let library = load_prefab_library(test_folder.name()).unwrap();

        assert_eq!(library.prefabs.len(), 1);
        assert_eq!(library.prefabs.get(&valid.id), Some(&valid));
    }

    #[test]
    fn load_prefab_library_skips_duplicate_prefab_ids_after_first_sorted_file() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let test_folder = TestGameFolder::new("prefab_duplicate_ids");
        let prefab_id = PrefabId(7);
        let first = PrefabAsset {
            id: prefab_id,
            name: "First".to_string(),
            next_node_id: 2,
            root_node_id: 1,
            nodes: vec![PrefabNode {
                node_id: 1,
                parent_node_id: None,
                components: vec![],
            }],
        };
        let second = PrefabAsset {
            name: "Second".to_string(),
            ..first.clone()
        };
        let folder = prefab_folder_for_game(test_folder.name());
        fs::create_dir_all(&folder).unwrap();

        fs::write(
            folder.join("a_first.ron"),
            ron::to_string(&first).unwrap(),
        )
        .unwrap();
        fs::write(
            folder.join("z_second.ron"),
            ron::to_string(&second).unwrap(),
        )
        .unwrap();

        let library = load_prefab_library(test_folder.name()).unwrap();

        assert_eq!(library.prefabs.len(), 1);
        assert_eq!(library.prefabs.get(&prefab_id), Some(&first));
    }

    #[test]
    fn load_prefab_library_skips_structurally_invalid_prefabs() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let test_folder = TestGameFolder::new("prefab_invalid_structure");
        let valid = create_prefab(PrefabId(1), "Valid".to_string());
        let invalid = PrefabAsset {
            id: PrefabId(2),
            name: "Broken".to_string(),
            next_node_id: 2,
            root_node_id: 99,
            nodes: vec![PrefabNode {
                node_id: 1,
                parent_node_id: None,
                components: vec![],
            }],
        };
        let folder = prefab_folder_for_game(test_folder.name());

        save_prefab(test_folder.name(), &valid).unwrap();
        fs::write(
            folder.join("broken_structure.ron"),
            ron::to_string(&invalid).unwrap(),
        )
        .unwrap();

        let library = load_prefab_library(test_folder.name()).unwrap();

        assert_eq!(library.prefabs.len(), 1);
        assert_eq!(library.prefabs.get(&valid.id), Some(&valid));
    }

    #[test]
    fn validate_prefab_rejects_disconnected_and_cyclic_graphs() {
        let disconnected = PrefabAsset {
            id: PrefabId(1),
            name: "Disconnected".to_string(),
            next_node_id: 3,
            root_node_id: 1,
            nodes: vec![
                PrefabNode {
                    node_id: 1,
                    parent_node_id: None,
                    components: vec![],
                },
                PrefabNode {
                    node_id: 2,
                    parent_node_id: None,
                    components: vec![],
                },
            ],
        };
        let cyclic = PrefabAsset {
            id: PrefabId(2),
            name: "Cycle".to_string(),
            next_node_id: 3,
            root_node_id: 1,
            nodes: vec![
                PrefabNode {
                    node_id: 1,
                    parent_node_id: Some(2),
                    components: vec![],
                },
                PrefabNode {
                    node_id: 2,
                    parent_node_id: Some(1),
                    components: vec![],
                },
            ],
        };

        assert!(validate_prefab(&disconnected).is_err());
        assert!(validate_prefab(&cyclic).is_err());
    }

    #[test]
    fn load_prefab_library_restores_next_prefab_id_from_loaded_assets() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let test_folder = TestGameFolder::new("prefab_next_id");
        let first = create_prefab(PrefabId(3), "First".to_string());
        let second = create_prefab(PrefabId(9), "Second".to_string());

        save_prefab(test_folder.name(), &first).unwrap();
        save_prefab(test_folder.name(), &second).unwrap();

        let mut library = load_prefab_library(test_folder.name()).unwrap();

        assert_eq!(library.next_prefab_id, 10);
        assert_eq!(library.allocate_prefab_id(), PrefabId(10));
        assert_eq!(library.allocate_prefab_id(), PrefabId(11));
    }
}
