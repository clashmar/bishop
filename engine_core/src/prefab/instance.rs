use super::component_sync::instantiate_prefab_components;
#[cfg(feature = "editor")]
use super::component_sync::apply_prefab_node;
use super::{PrefabInstanceNode, PrefabInstanceRoot};
#[cfg(feature = "editor")]
use super::PrefabOverrides;
use crate::ecs::capture::restore_entity;
use crate::ecs::entity::{Entity, set_parent};
#[cfg(feature = "editor")]
use crate::ecs::entity::remove_parent;
#[cfg(feature = "editor")]
use crate::ecs::Transform;
use crate::game::GameCtxMut;
use crate::onscreen_error;
use crate::prefab::{PrefabAsset, validate_prefab};
use crate::worlds::room::RoomId;
use bishop::prelude::*;
use std::collections::HashMap;
#[cfg(feature = "editor")]
use crate::ecs::Ecs;

pub fn instantiate_prefab(
    ctx: &mut GameCtxMut<'_>,
    prefab: &PrefabAsset,
    root_position: Vec2,
    room_id: Option<RoomId>,
) -> Entity {
    if let Err(error) = validate_prefab(prefab) {
        onscreen_error!("Failed to instantiate prefab '{}': {error}", prefab.name);
        return Entity::null();
    }

    let mut entities = HashMap::new();
    let mut nodes = prefab.nodes.iter().collect::<Vec<_>>();
    nodes.sort_by_key(|node| node.node_id);

    for node in &nodes {
        let entity = ctx.ecs().create_entity().finish();
        entities.insert(node.node_id, entity);
    }

    let Some(root_entity) = entities.get(&prefab.root_node_id).copied() else {
        onscreen_error!("Failed to instantiate prefab '{}': missing root node", prefab.name);
        return Entity::null();
    };

    for node in &nodes {
        let Some(entity) = entities.get(&node.node_id).copied() else {
            continue;
        };
        restore_entity(
            ctx,
            entity,
            instantiate_prefab_components(node, root_position, room_id),
        );
    }

    for node in &nodes {
        if let Some(parent_node_id) = node.parent_node_id
            && let (Some(&entity), Some(&parent)) =
                (entities.get(&node.node_id), entities.get(&parent_node_id))
        {
            set_parent(ctx.ecs(), entity, parent);
        }
    }

    for node in &nodes {
        if let Some(&entity) = entities.get(&node.node_id) {
            ctx.ecs().add_component_to_entity(
                entity,
                PrefabInstanceNode {
                    prefab_id: prefab.id,
                    node_id: node.node_id,
                    root_entity,
                },
            );
        }
    }

    ctx.ecs().add_component_to_entity(
        root_entity,
        PrefabInstanceRoot {
            prefab_id: prefab.id,
        },
    );

    root_entity
}

#[cfg(feature = "editor")]
pub fn refresh_prefab_instance(
    ctx: &mut GameCtxMut<'_>,
    root_entity: Entity,
    prefab: &PrefabAsset,
    room_id: Option<RoomId>,
) {
    if let Err(error) = validate_prefab(prefab) {
        onscreen_error!("Failed to refresh prefab '{}': {error}", prefab.name);
        return;
    }

    let root_position = ctx
        .ecs()
        .get::<Transform>(root_entity)
        .map(|transform| transform.position)
        .unwrap_or_default();
    let prefab_nodes = prefab
        .nodes
        .iter()
        .map(|node| (node.node_id, node))
        .collect::<HashMap<_, _>>();
    let mut instance_entities = prefab_instance_entities(ctx.ecs(), root_entity);
    let stale_entities = instance_entities
        .iter()
        .filter(|(node_id, _)| !prefab_nodes.contains_key(node_id))
        .map(|(_, entity)| *entity)
        .collect::<Vec<_>>();

    for entity in stale_entities {
        Ecs::remove_entity(ctx, entity);
    }

    instance_entities = prefab_instance_entities(ctx.ecs(), root_entity);
    let mut missing_nodes = prefab_nodes
        .keys()
        .filter(|node_id| !instance_entities.contains_key(node_id))
        .copied()
        .collect::<Vec<_>>();
    missing_nodes.sort_unstable();

    for node_id in missing_nodes {
        let entity = ctx.ecs().create_entity().finish();
        instance_entities.insert(node_id, entity);
    }

    let mut ordered_nodes = prefab.nodes.iter().collect::<Vec<_>>();
    ordered_nodes.sort_by_key(|node| node.node_id);

    for node in &ordered_nodes {
        let Some(entity) = instance_entities.get(&node.node_id).copied() else {
            continue;
        };
        let overrides = ctx.ecs().get::<PrefabOverrides>(entity).cloned();

        apply_prefab_node(
            ctx,
            entity,
            node,
            root_position,
            room_id,
            overrides.as_ref(),
            entity == root_entity && node.node_id == prefab.root_node_id,
        );
    }

    for node in &ordered_nodes {
        let Some(entity) = instance_entities.get(&node.node_id).copied() else {
            continue;
        };

        if let Some(parent_node_id) = node.parent_node_id {
            if let Some(&parent_entity) = instance_entities.get(&parent_node_id) {
                set_parent(ctx.ecs(), entity, parent_entity);
            }
        } else {
            remove_parent(ctx.ecs(), entity);
        }

        ctx.ecs().add_component_to_entity(
            entity,
            PrefabInstanceNode {
                prefab_id: prefab.id,
                node_id: node.node_id,
                root_entity,
            },
        );
    }

    ctx.ecs().add_component_to_entity(
        root_entity,
        PrefabInstanceRoot {
            prefab_id: prefab.id,
        },
    );
}

#[cfg(feature = "editor")]
pub(super) fn prefab_instance_entities(ecs: &Ecs, root_entity: Entity) -> HashMap<usize, Entity> {
    ecs.get_store::<PrefabInstanceNode>()
        .data
        .iter()
        .filter_map(|(&entity, metadata)| {
            (metadata.root_entity == root_entity).then_some((metadata.node_id, entity))
        })
        .collect()
}