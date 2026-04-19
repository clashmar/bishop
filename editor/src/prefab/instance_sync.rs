use crate::shared::scene_ui::inspector::ScenePrefabAction;
use crate::shared::scene_ui::prefab_link::PrefabLinkSource;
use engine_core::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LinkedPrefabInstanceState {
    pub source: PrefabLinkSource,
    pub selected_entity: Entity,
    pub root_entity: Entity,
    pub prefab_id: PrefabId,
    pub prefab_name: String,
    pub label: String,
    pub has_overrides: bool,
    pub has_local_changes: bool,
    pub open_action: ScenePrefabAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LinkedPrefabReference {
    pub source: PrefabLinkSource,
    pub root_entity: Entity,
    pub prefab_id: PrefabId,
}

pub(crate) fn linked_prefab_reference(ecs: &Ecs, entity: Entity) -> Option<LinkedPrefabReference> {
    if let Some(root) = ecs.get::<PrefabInstanceRoot>(entity) {
        return Some(LinkedPrefabReference {
            source: PrefabLinkSource::Root,
            root_entity: entity,
            prefab_id: root.prefab_id,
        });
    }

    ecs.get::<PrefabInstanceNode>(entity)
        .map(|node| LinkedPrefabReference {
            source: PrefabLinkSource::Node,
            root_entity: node.root_entity,
            prefab_id: node.prefab_id,
        })
}

pub(crate) fn linked_prefab_instance_state(
    ecs: &mut Ecs,
    prefab_manager: &PrefabManager,
    entity: Entity,
) -> Option<LinkedPrefabInstanceState> {
    let reference = linked_prefab_reference(ecs, entity)?;
    let prefab = prefab_manager.prefabs.get(&reference.prefab_id)?;
    let has_local_changes = instance_prefab_differs(ecs, prefab, reference.root_entity);

    Some(LinkedPrefabInstanceState {
        source: reference.source,
        selected_entity: entity,
        root_entity: reference.root_entity,
        prefab_id: reference.prefab_id,
        prefab_name: prefab.name.clone(),
        label: format!("Prefab: {}", prefab.name),
        has_overrides: subtree_has_prefab_overrides(ecs, reference.root_entity),
        has_local_changes,
        open_action: ScenePrefabAction::OpenPrefabEditor,
    })
}

pub(crate) fn linked_prefab_instance_roots(ecs: &Ecs, prefab_id: PrefabId) -> Vec<Entity> {
    ecs.get_store::<PrefabInstanceRoot>()
        .data
        .iter()
        .filter_map(|(&entity, root)| (root.prefab_id == prefab_id).then_some(entity))
        .collect()
}

pub(crate) fn capture_linked_prefab_instance_snapshots(
    ecs: &mut Ecs,
    prefab_id: PrefabId,
) -> Vec<GroupSnapshot> {
    linked_prefab_instance_roots(ecs, prefab_id)
        .into_iter()
        .map(|root_entity| capture_subtree(ecs, root_entity))
        .collect()
}

pub(crate) fn subtree_has_prefab_overrides(ecs: &Ecs, root_entity: Entity) -> bool {
    linked_prefab_subtree_entities(ecs, root_entity)
        .into_iter()
        .any(|entity| ecs.has::<PrefabOverrides>(entity))
}

pub(crate) fn instance_prefab_differs(
    ecs: &mut Ecs,
    prefab: &PrefabAsset,
    root_entity: Entity,
) -> bool {
    capture_prefab_with_existing(
        ecs,
        root_entity,
        prefab.id,
        prefab.name.clone(),
        Some(prefab),
    ) != *prefab
}

pub(crate) fn sync_prefab_overrides_for_entity(
    ecs: &mut Ecs,
    prefab_manager: &PrefabManager,
    entity: Entity,
) {
    let Some(reference) = linked_prefab_reference(ecs, entity) else {
        return;
    };
    let Some(prefab) = prefab_manager.prefabs.get(&reference.prefab_id) else {
        return;
    };

    sync_prefab_overrides_for_root(ecs, prefab, reference.root_entity);
}

pub(crate) fn sync_prefab_overrides_for_root(
    ecs: &mut Ecs,
    prefab: &PrefabAsset,
    root_entity: Entity,
) {
    let captured = capture_prefab_with_existing(
        ecs,
        root_entity,
        prefab.id,
        prefab.name.clone(),
        Some(prefab),
    );
    let captured_nodes = captured
        .nodes
        .iter()
        .map(|node| (node.node_id, node))
        .collect::<HashMap<_, _>>();
    let prefab_nodes = prefab
        .nodes
        .iter()
        .map(|node| (node.node_id, node))
        .collect::<HashMap<_, _>>();

    for entity in linked_prefab_subtree_entities(ecs, root_entity) {
        let Some(metadata) = ecs.get::<PrefabInstanceNode>(entity).copied() else {
            continue;
        };
        let Some(captured_node) = captured_nodes.get(&metadata.node_id) else {
            continue;
        };
        let Some(prefab_node) = prefab_nodes.get(&metadata.node_id) else {
            continue;
        };

        let captured_components = component_map(&captured_node.components);
        let prefab_components = component_map(&prefab_node.components);

        let mut modified_components = prefab_components
            .iter()
            .filter_map(|(type_name, prefab_component)| {
                let captured_component = captured_components.get(type_name)?;
                (captured_component.ron != prefab_component.ron).then(|| type_name.clone())
            })
            .collect::<Vec<_>>();
        modified_components.sort();

        let mut removed_components = prefab_components
            .keys()
            .filter(|type_name| !captured_components.contains_key(*type_name))
            .cloned()
            .collect::<Vec<_>>();
        removed_components.sort();

        let mut added_components = captured_components
            .iter()
            .filter(|(type_name, _)| !prefab_components.contains_key(*type_name))
            .map(|(_, component)| component.clone())
            .collect::<Vec<_>>();
        added_components.sort_by(|left, right| left.type_name.cmp(&right.type_name));

        if modified_components.is_empty()
            && removed_components.is_empty()
            && added_components.is_empty()
        {
            ecs.get_store_mut::<PrefabOverrides>().remove(entity);
            continue;
        }

        ecs.add_component_to_entity(
            entity,
            PrefabOverrides {
                modified_components,
                removed_components,
                added_components,
            },
        );
    }
}

pub(crate) fn clear_prefab_metadata_from_root(ecs: &mut Ecs, root_entity: Entity) {
    for entity in linked_prefab_subtree_entities(ecs, root_entity) {
        ecs.get_store_mut::<PrefabOverrides>().remove(entity);
        ecs.get_store_mut::<PrefabInstanceNode>().remove(entity);
    }
    ecs.get_store_mut::<PrefabInstanceRoot>()
        .remove(root_entity);
}

pub(crate) fn replace_linked_instances_with_snapshots(
    game: &mut Game,
    prefab_id: PrefabId,
    snapshots: &[GroupSnapshot],
) {
    let roots = linked_prefab_instance_roots(&game.ecs, prefab_id);
    for root_entity in roots {
        let mut ctx = game.ctx_mut();
        Ecs::remove_entity(&mut ctx, root_entity);
    }

    for snapshot in snapshots {
        let mut ctx = game.ctx_mut();
        restore_subtree(&mut ctx, snapshot);
    }
}

fn linked_prefab_subtree_entities(ecs: &Ecs, root_entity: Entity) -> Vec<Entity> {
    let mut entities = ecs
        .get_store::<PrefabInstanceNode>()
        .data
        .iter()
        .filter_map(|(&entity, metadata)| (metadata.root_entity == root_entity).then_some(entity))
        .collect::<Vec<_>>();
    entities.sort_by_key(|entity| entity.0);

    if entities.is_empty() && ecs.has::<PrefabInstanceRoot>(root_entity) {
        entities.push(root_entity);
    }

    entities
}

fn component_map(components: &[ComponentSnapshot]) -> HashMap<String, ComponentSnapshot> {
    components
        .iter()
        .map(|component| (component.type_name.clone(), component.clone()))
        .collect()
}
