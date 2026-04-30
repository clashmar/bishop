use super::component_sync::{excluded_from_prefab_asset, translate_transform_snapshot};
use super::{PrefabInstanceNode, PrefabInstanceRoot};
use crate::ecs::capture::{capture_subtree, ComponentSnapshot};
use crate::ecs::entity::{get_parent, Entity};
use crate::ecs::{Ecs, Transform};
use crate::prefab::{PrefabAsset, PrefabId, PrefabNode};
use bishop::prelude::*;
use std::collections::{HashMap, HashSet};

pub fn capture_prefab(
    ecs: &mut Ecs,
    root: Entity,
    prefab_id: PrefabId,
    name: String,
) -> PrefabAsset {
    capture_prefab_with_existing(ecs, root, prefab_id, name, None)
}

pub fn capture_prefab_with_existing(
    ecs: &mut Ecs,
    root: Entity,
    prefab_id: PrefabId,
    name: String,
    existing: Option<&PrefabAsset>,
) -> PrefabAsset {
    let root_position = ecs
        .get::<Transform>(root)
        .map(|transform| transform.position)
        .unwrap_or_default();
    let snapshots = capture_subtree(ecs, root);
    let prefab_id = existing
        .map(|prefab| prefab.id)
        .or_else(|| {
            ecs.get::<PrefabInstanceRoot>(root)
                .map(|metadata| metadata.prefab_id)
        })
        .unwrap_or(prefab_id);
    let mut node_ids = HashMap::new();
    let mut used_node_ids = HashSet::new();
    let mut next_node_id = existing.map(|prefab| prefab.next_node_id).unwrap_or(1);

    for snapshot in &snapshots {
        let Some(metadata) = ecs.get::<PrefabInstanceNode>(snapshot.entity) else {
            continue;
        };

        if metadata.prefab_id != prefab_id || !used_node_ids.insert(metadata.node_id) {
            continue;
        }

        node_ids.insert(snapshot.entity, metadata.node_id);
        next_node_id = next_node_id.max(metadata.node_id + 1);
    }

    for snapshot in &snapshots {
        if node_ids.contains_key(&snapshot.entity) {
            continue;
        }

        while used_node_ids.contains(&next_node_id) {
            next_node_id += 1;
        }

        node_ids.insert(snapshot.entity, next_node_id);
        used_node_ids.insert(next_node_id);
        next_node_id += 1;
    }

    let mut nodes = Vec::with_capacity(snapshots.len());
    for snapshot in snapshots {
        let node_id = node_ids.get(&snapshot.entity).copied().unwrap_or_default();
        let parent_node_id =
            get_parent(ecs, snapshot.entity).and_then(|parent| node_ids.get(&parent).copied());
        let components = prefab_components_from_snapshot(snapshot.components, root_position);

        nodes.push(PrefabNode {
            node_id,
            parent_node_id,
            components,
        });
    }

    PrefabAsset {
        id: prefab_id,
        name,
        next_node_id,
        root_node_id: node_ids.get(&root).copied().unwrap_or(1),
        nodes,
    }
}

fn prefab_components_from_snapshot(
    components: Vec<ComponentSnapshot>,
    root_position: Vec2,
) -> Vec<ComponentSnapshot> {
    components
        .into_iter()
        .filter(|component| !excluded_from_prefab_asset(&component.type_name))
        .map(|component| translate_transform_snapshot(&component, -root_position))
        .collect()
}
