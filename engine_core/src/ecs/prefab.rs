use crate::camera::game_camera::RoomCamera;
use crate::ecs::capture::{ComponentSnapshot, capture_entity, capture_subtree, restore_entity};
use crate::ecs::component::{CurrentRoom, Global, Player, PlayerProxy, comp_type_name};
use crate::ecs::component_registry::ComponentRegistry;
use crate::ecs::ecs::Ecs;
use crate::ecs::entity::{Entity, Parent, get_parent, remove_parent, set_parent};
use crate::ecs::transform::Transform;
use crate::game::EngineCtxMut;
use crate::onscreen_error;
use crate::prefab::{PrefabAsset, PrefabId, PrefabNode, validate_prefab};
use crate::worlds::room::RoomId;
use bishop::prelude::*;
use ecs_component::ecs_component;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Marks the root entity for a linked prefab instance.
#[ecs_component]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrefabInstanceRoot {
    /// Stable prefab asset id for this linked instance.
    pub prefab_id: PrefabId,
}

/// Marks an entity as belonging to a linked prefab node.
#[ecs_component]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct PrefabInstanceNode {
    /// Stable prefab asset id for this linked instance.
    pub prefab_id: PrefabId,
    /// Stable prefab node id within the asset.
    pub node_id: usize,
    /// Root entity for the linked instance subtree.
    pub root_entity: Entity,
}

/// Stores local divergence from the source prefab definition.
#[ecs_component]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrefabOverrides {
    /// Component type names modified locally on this instance entity.
    pub modified_components: Vec<String>,
    /// Component type names removed locally on this instance entity.
    pub removed_components: Vec<String>,
    /// Components added locally on this instance entity.
    pub added_components: Vec<ComponentSnapshot>,
}

/// Captures a room subtree as a room-agnostic prefab asset.
pub fn capture_prefab(
    ecs: &mut Ecs,
    root: Entity,
    prefab_id: PrefabId,
    name: String,
) -> PrefabAsset {
    capture_prefab_with_existing(ecs, root, prefab_id, name, None)
}

/// Captures a room subtree while preserving stable node ids from an existing prefab when possible.
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

/// Instantiates a prefab hierarchy into ECS and returns the root entity.
pub fn instantiate_prefab(
    ctx: &mut dyn EngineCtxMut,
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

/// Refreshes a linked prefab instance subtree from the source asset.
pub fn refresh_prefab_instance(
    ctx: &mut dyn EngineCtxMut,
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

fn instantiate_prefab_components(
    node: &PrefabNode,
    root_position: Vec2,
    room_id: Option<RoomId>,
) -> Vec<ComponentSnapshot> {
    let mut components = node
        .components
        .iter()
        .map(|component| translate_transform_snapshot(component, root_position))
        .collect::<Vec<_>>();

    if let Some(room_id) = room_id
        && !components
            .iter()
            .any(|component| component.type_name == comp_type_name::<CurrentRoom>())
        && let Ok(ron) = ron::to_string(&CurrentRoom(room_id))
    {
        components.push(ComponentSnapshot {
            type_name: comp_type_name::<CurrentRoom>().to_string(),
            ron,
        });
    }

    components
}

fn prefab_instance_entities(ecs: &Ecs, root_entity: Entity) -> HashMap<usize, Entity> {
    ecs.get_store::<PrefabInstanceNode>()
        .data
        .iter()
        .filter_map(|(&entity, metadata)| {
            (metadata.root_entity == root_entity).then_some((metadata.node_id, entity))
        })
        .collect()
}

fn apply_prefab_node(
    ctx: &mut dyn EngineCtxMut,
    entity: Entity,
    node: &PrefabNode,
    root_position: Vec2,
    room_id: Option<RoomId>,
    overrides: Option<&PrefabOverrides>,
    is_instance_root: bool,
) {
    let modified_components = overrides
        .map(|value| value.modified_components.iter().cloned().collect::<HashSet<_>>())
        .unwrap_or_default();
    let removed_components = overrides
        .map(|value| value.removed_components.iter().cloned().collect::<HashSet<_>>())
        .unwrap_or_default();
    let prefab_components = instantiate_prefab_components(node, root_position, room_id);

    for component in &prefab_components {
        if removed_components.contains(&component.type_name) {
            remove_component_snapshot(ctx, entity, &component.type_name);
            continue;
        }

        if modified_components.contains(&component.type_name) {
            continue;
        }

        if is_instance_root
            && component.type_name == comp_type_name::<Transform>()
        {
            apply_root_transform_snapshot(ctx, entity, component);
            continue;
        }

        apply_component_snapshot(ctx, entity, component.clone());
    }

    if let Some(overrides) = overrides {
        for type_name in &overrides.removed_components {
            remove_component_snapshot(ctx, entity, type_name);
        }

        for component in &overrides.added_components {
            apply_component_snapshot(ctx, entity, component.clone());
        }
    }

    remove_stale_prefab_components(
        ctx,
        entity,
        &prefab_components,
        overrides,
        is_instance_root,
    );
}

fn apply_component_snapshot(
    ctx: &mut dyn EngineCtxMut,
    entity: Entity,
    component: ComponentSnapshot,
) {
    remove_component_snapshot(ctx, entity, &component.type_name);
    restore_entity(ctx, entity, vec![component]);
}

fn remove_component_snapshot(ctx: &mut dyn EngineCtxMut, entity: Entity, type_name: &str) {
    let Some(component_reg) = inventory::iter::<ComponentRegistry>()
        .find(|registry| registry.type_name == type_name)
    else {
        return;
    };

    if !(component_reg.has)(ctx.ecs(), entity) {
        return;
    }

    let mut boxed = (component_reg.clone)(ctx.ecs(), entity);
    (component_reg.post_remove)(&mut *boxed, &entity, ctx);
    (component_reg.remove)(ctx.ecs(), entity);
}

fn apply_root_transform_snapshot(
    ctx: &mut dyn EngineCtxMut,
    entity: Entity,
    component: &ComponentSnapshot,
) {
    let Ok(mut prefab_transform) = ron::from_str::<Transform>(&component.ron) else {
        return;
    };

    if let Some(current_transform) = ctx.ecs().get::<Transform>(entity).copied() {
        prefab_transform.position = current_transform.position;
    }

    let Ok(ron) = ron::to_string(&prefab_transform) else {
        return;
    };

    apply_component_snapshot(
        ctx,
        entity,
        ComponentSnapshot {
            type_name: component.type_name.clone(),
            ron,
        },
    );
}

fn remove_stale_prefab_components(
    ctx: &mut dyn EngineCtxMut,
    entity: Entity,
    prefab_components: &[ComponentSnapshot],
    overrides: Option<&PrefabOverrides>,
    is_instance_root: bool,
) {
    let prefab_types = prefab_components
        .iter()
        .map(|component| component.type_name.clone())
        .collect::<HashSet<_>>();
    let modified_types = overrides
        .map(|value| value.modified_components.iter().cloned().collect::<HashSet<_>>())
        .unwrap_or_default();
    let added_types = overrides
        .map(|value| {
            value
                .added_components
                .iter()
                .map(|component| component.type_name.clone())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();

    for component in capture_entity(ctx.ecs(), entity) {
        let is_reserved_type = component.type_name == comp_type_name::<PrefabInstanceRoot>()
            || component.type_name == comp_type_name::<PrefabInstanceNode>()
            || component.type_name == comp_type_name::<PrefabOverrides>()
            || component.type_name == comp_type_name::<CurrentRoom>()
            || component.type_name == comp_type_name::<Parent>()
            || component.type_name == comp_type_name::<crate::ecs::entity::Children>()
            || (is_instance_root
                && component.type_name == comp_type_name::<Transform>()
                && prefab_types.contains(comp_type_name::<Transform>()));

        if is_reserved_type
            || prefab_types.contains(&component.type_name)
            || modified_types.contains(&component.type_name)
            || added_types.contains(&component.type_name)
        {
            continue;
        }

        remove_component_snapshot(ctx, entity, &component.type_name);
    }
}

fn excluded_from_prefab_asset(type_name: &str) -> bool {
    type_name == comp_type_name::<crate::ecs::entity::Children>()
        || type_name == comp_type_name::<Parent>()
        || type_name == comp_type_name::<CurrentRoom>()
        || type_name == comp_type_name::<RoomCamera>()
        || type_name == comp_type_name::<PlayerProxy>()
        || type_name == comp_type_name::<Player>()
        || type_name == comp_type_name::<Global>()
        || type_name == comp_type_name::<PrefabInstanceRoot>()
        || type_name == comp_type_name::<PrefabInstanceNode>()
        || type_name == comp_type_name::<PrefabOverrides>()
}

fn translate_transform_snapshot(
    component: &ComponentSnapshot,
    delta: Vec2,
) -> ComponentSnapshot {
    if component.type_name != comp_type_name::<Transform>() {
        return component.clone();
    }

    let Ok(mut transform) = ron::from_str::<Transform>(&component.ron) else {
        return component.clone();
    };
    transform.position += delta;

    match ron::to_string(&transform) {
        Ok(ron) => ComponentSnapshot {
            type_name: component.type_name.clone(),
            ron,
        },
        Err(_) => component.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::asset_manager::AssetManager;
    use crate::ecs::component::{CurrentRoom, Name, Velocity};
    use crate::ecs::transform::{Pivot, Transform};
    use crate::game::Game;
    use crate::scripting::script_manager::ScriptManager;
    use crate::worlds::world::{World, WorldId};
    use uuid::Uuid;

    fn test_game() -> Game {
        let world_id = WorldId(Uuid::new_v4());
        Game {
            id: Uuid::new_v4(),
            name: "prefab_tests".to_string(),
            worlds: vec![World {
                id: world_id,
                ..Default::default()
            }],
            current_world_id: world_id,
            asset_manager: AssetManager::default(),
            script_manager: ScriptManager::default(),
            ..Default::default()
        }
    }

    fn find_entity_for_node(ecs: &Ecs, root_entity: Entity, node_id: usize) -> Option<Entity> {
        ecs.get_store::<PrefabInstanceNode>()
            .data
            .iter()
            .find_map(|(entity, metadata)| {
                (metadata.root_entity == root_entity && metadata.node_id == node_id)
                    .then_some(*entity)
            })
    }

    #[test]
    fn capture_prefab_normalizes_root_offset_and_instantiate_restores_world_positions() {
        let mut game = test_game();
        let room_id = RoomId(7);

        let root = game
            .ecs
            .create_entity()
            .with(Name("Root".to_string()))
            .with(Transform {
                position: Vec2::new(10.0, 15.0),
                ..Default::default()
            })
            .with(CurrentRoom(room_id))
            .finish();
        let child = game
            .ecs
            .create_entity()
            .with(Name("Child".to_string()))
            .with(Transform {
                position: Vec2::new(14.0, 18.0),
                ..Default::default()
            })
            .finish();
        set_parent(&mut game.ecs, child, root);

        let prefab = capture_prefab(&mut game.ecs, root, PrefabId(1), "crate".to_string());

        let saved_root = prefab
            .nodes
            .iter()
            .find(|node| node.node_id == prefab.root_node_id)
            .unwrap();
        let saved_child = prefab
            .nodes
            .iter()
            .find(|node| node.node_id != prefab.root_node_id)
            .unwrap();
        let root_transform = saved_root
            .components
            .iter()
            .find(|component| component.type_name == "Transform")
            .unwrap();
        let child_transform = saved_child
            .components
            .iter()
            .find(|component| component.type_name == "Transform")
            .unwrap();

        assert!(root_transform.ron.contains("position:(0.0,0.0)"));
        assert!(child_transform.ron.contains("position:(4.0,3.0)"));
        assert!(!saved_root
            .components
            .iter()
            .any(|component| component.type_name == "CurrentRoom"));

        let root_entity = {
            let mut ctx = game.ctx_mut();
            instantiate_prefab(&mut ctx, &prefab, Vec2::new(100.0, 200.0), Some(room_id))
        };

        let child_entity = crate::ecs::entity::get_children(&game.ecs, root_entity)
            .into_iter()
            .next()
            .unwrap();
        let instantiated_root = game.ecs.get::<Transform>(root_entity).unwrap();
        let instantiated_child = game.ecs.get::<Transform>(child_entity).unwrap();

        assert_eq!(instantiated_root.position, Vec2::new(100.0, 200.0));
        assert_eq!(instantiated_child.position, Vec2::new(104.0, 203.0));
        assert_eq!(
            game.ecs.get::<CurrentRoom>(root_entity).map(|room| room.0),
            Some(room_id)
        );
        assert_eq!(
            game.ecs.get::<CurrentRoom>(child_entity).map(|room| room.0),
            Some(room_id)
        );
    }

    #[test]
    fn refresh_prefab_instance_preserves_added_local_components() {
        let mut game = test_game();
        let prefab = PrefabAsset {
            id: PrefabId(1),
            name: "crate".to_string(),
            next_node_id: 2,
            root_node_id: 1,
            nodes: vec![PrefabNode {
                node_id: 1,
                parent_node_id: None,
                components: vec![ComponentSnapshot {
                    type_name: "Name".to_string(),
                    ron: "(\"Prefab Root\")".to_string(),
                }],
            }],
        };

        let root_entity = {
            let mut ctx = game.ctx_mut();
            instantiate_prefab(&mut ctx, &prefab, Vec2::ZERO, None)
        };

        game.ecs
            .add_component_to_entity(root_entity, Velocity { x: 2.0, y: 4.0 });
        game.ecs.add_component_to_entity(
            root_entity,
            PrefabOverrides {
                added_components: vec![ComponentSnapshot {
                    type_name: "Velocity".to_string(),
                    ron: "(x:2.0,y:4.0)".to_string(),
                }],
                ..Default::default()
            },
        );

        let updated_prefab = PrefabAsset {
            nodes: vec![PrefabNode {
                node_id: 1,
                parent_node_id: None,
                components: vec![ComponentSnapshot {
                    type_name: "Name".to_string(),
                    ron: "(\"Updated Root\")".to_string(),
                }],
            }],
            ..prefab
        };

        {
            let mut ctx = game.ctx_mut();
            refresh_prefab_instance(&mut ctx, root_entity, &updated_prefab, None);
        }

        assert_eq!(
            game.ecs.get::<Name>(root_entity).map(|name| name.0.as_str()),
            Some("Updated Root")
        );
        assert_eq!(
            game.ecs
                .get::<Velocity>(root_entity)
                .map(|velocity| (velocity.x, velocity.y)),
            Some((2.0, 4.0))
        );
    }

    #[test]
    fn refresh_prefab_instance_applies_node_additions_and_removals_by_node_id() {
        let mut game = test_game();
        let prefab = PrefabAsset {
            id: PrefabId(1),
            name: "tree".to_string(),
            next_node_id: 3,
            root_node_id: 1,
            nodes: vec![
                PrefabNode {
                    node_id: 1,
                    parent_node_id: None,
                    components: vec![ComponentSnapshot {
                        type_name: "Name".to_string(),
                        ron: "(\"Root\")".to_string(),
                    }],
                },
                PrefabNode {
                    node_id: 2,
                    parent_node_id: Some(1),
                    components: vec![ComponentSnapshot {
                        type_name: "Name".to_string(),
                        ron: "(\"Old Child\")".to_string(),
                    }],
                },
            ],
        };

        let root_entity = {
            let mut ctx = game.ctx_mut();
            instantiate_prefab(&mut ctx, &prefab, Vec2::ZERO, None)
        };
        let old_child = find_entity_for_node(&game.ecs, root_entity, 2).unwrap();

        let updated_prefab = PrefabAsset {
            next_node_id: 4,
            nodes: vec![
                PrefabNode {
                    node_id: 1,
                    parent_node_id: None,
                    components: vec![ComponentSnapshot {
                        type_name: "Name".to_string(),
                        ron: "(\"Root\")".to_string(),
                    }],
                },
                PrefabNode {
                    node_id: 3,
                    parent_node_id: Some(1),
                    components: vec![ComponentSnapshot {
                        type_name: "Name".to_string(),
                        ron: "(\"New Child\")".to_string(),
                    }],
                },
            ],
            ..prefab
        };

        {
            let mut ctx = game.ctx_mut();
            refresh_prefab_instance(&mut ctx, root_entity, &updated_prefab, None);
        }

        let new_child = find_entity_for_node(&game.ecs, root_entity, 3).unwrap();
        assert!(game.ecs.get::<Name>(old_child).is_none());
        assert_eq!(get_parent(&game.ecs, new_child), Some(root_entity));
        assert_eq!(
            game.ecs.get::<Name>(new_child).map(|name| name.0.as_str()),
            Some("New Child")
        );
    }

    #[test]
    fn refresh_prefab_instance_removes_deleted_prefab_components_without_local_override() {
        let mut game = test_game();
        let prefab = PrefabAsset {
            id: PrefabId(1),
            name: "mover".to_string(),
            next_node_id: 2,
            root_node_id: 1,
            nodes: vec![PrefabNode {
                node_id: 1,
                parent_node_id: None,
                components: vec![
                    ComponentSnapshot {
                        type_name: "Name".to_string(),
                        ron: "(\"Mover\")".to_string(),
                    },
                    ComponentSnapshot {
                        type_name: "Velocity".to_string(),
                        ron: "(x:1.0,y:2.0)".to_string(),
                    },
                ],
            }],
        };

        let root_entity = {
            let mut ctx = game.ctx_mut();
            instantiate_prefab(&mut ctx, &prefab, Vec2::ZERO, None)
        };
        assert!(game.ecs.has::<Velocity>(root_entity));

        let updated_prefab = PrefabAsset {
            nodes: vec![PrefabNode {
                node_id: 1,
                parent_node_id: None,
                components: vec![ComponentSnapshot {
                    type_name: "Name".to_string(),
                    ron: "(\"Mover\")".to_string(),
                }],
            }],
            ..prefab
        };

        {
            let mut ctx = game.ctx_mut();
            refresh_prefab_instance(&mut ctx, root_entity, &updated_prefab, None);
        }

        assert!(!game.ecs.has::<Velocity>(root_entity));
    }

    #[test]
    fn refresh_prefab_instance_removes_parent_when_node_becomes_root_level() {
        let mut game = test_game();
        let prefab = PrefabAsset {
            id: PrefabId(1),
            name: "tree".to_string(),
            next_node_id: 3,
            root_node_id: 1,
            nodes: vec![
                PrefabNode {
                    node_id: 1,
                    parent_node_id: None,
                    components: vec![ComponentSnapshot {
                        type_name: "Name".to_string(),
                        ron: "(\"Root\")".to_string(),
                    }],
                },
                PrefabNode {
                    node_id: 2,
                    parent_node_id: Some(1),
                    components: vec![ComponentSnapshot {
                        type_name: "Name".to_string(),
                        ron: "(\"Child\")".to_string(),
                    }],
                },
            ],
        };

        let root_entity = {
            let mut ctx = game.ctx_mut();
            instantiate_prefab(&mut ctx, &prefab, Vec2::ZERO, None)
        };
        let child_entity = find_entity_for_node(&game.ecs, root_entity, 2).unwrap();
        assert_eq!(get_parent(&game.ecs, child_entity), Some(root_entity));

        let updated_prefab = PrefabAsset {
            nodes: vec![PrefabNode {
                node_id: 1,
                parent_node_id: None,
                components: vec![ComponentSnapshot {
                    type_name: "Name".to_string(),
                    ron: "(\"Root\")".to_string(),
                }],
            }],
            ..prefab
        };

        {
            let mut ctx = game.ctx_mut();
            refresh_prefab_instance(&mut ctx, root_entity, &updated_prefab, None);
        }

        assert_eq!(get_parent(&game.ecs, child_entity), None);
    }

    #[test]
    fn capture_prefab_with_existing_preserves_stable_node_ids() {
        let mut game = test_game();
        let prefab = PrefabAsset {
            id: PrefabId(1),
            name: "crate".to_string(),
            next_node_id: 10,
            root_node_id: 4,
            nodes: vec![
                PrefabNode {
                    node_id: 4,
                    parent_node_id: None,
                    components: vec![ComponentSnapshot {
                        type_name: "Name".to_string(),
                        ron: "(\"Root\")".to_string(),
                    }],
                },
                PrefabNode {
                    node_id: 7,
                    parent_node_id: Some(4),
                    components: vec![ComponentSnapshot {
                        type_name: "Name".to_string(),
                        ron: "(\"Child\")".to_string(),
                    }],
                },
            ],
        };

        let root_entity = {
            let mut ctx = game.ctx_mut();
            instantiate_prefab(&mut ctx, &prefab, Vec2::ZERO, None)
        };
        let extra_child = game
            .ecs
            .create_entity()
            .with(Name("Extra".to_string()))
            .finish();
        set_parent(&mut game.ecs, extra_child, root_entity);

        let captured =
            capture_prefab_with_existing(
                &mut game.ecs,
                root_entity,
                prefab.id,
                "crate".to_string(),
                Some(&prefab),
            );
        let node_ids = captured
            .nodes
            .iter()
            .map(|node| node.node_id)
            .collect::<HashSet<_>>();

        assert!(node_ids.contains(&4));
        assert!(node_ids.contains(&7));
        assert!(node_ids.contains(&10));
        assert_eq!(captured.next_node_id, 11);
    }

    #[test]
    fn refresh_prefab_instance_updates_root_transform_fields_but_keeps_position() {
        let mut game = test_game();
        let prefab = PrefabAsset {
            id: PrefabId(1),
            name: "root_transform".to_string(),
            next_node_id: 2,
            root_node_id: 1,
            nodes: vec![PrefabNode {
                node_id: 1,
                parent_node_id: None,
                components: vec![ComponentSnapshot {
                    type_name: "Transform".to_string(),
                    ron: ron::to_string(&Transform::default()).unwrap(),
                }],
            }],
        };

        let root_entity = {
            let mut ctx = game.ctx_mut();
            instantiate_prefab(&mut ctx, &prefab, Vec2::new(12.0, 34.0), None)
        };

        let updated_prefab = PrefabAsset {
            nodes: vec![PrefabNode {
                node_id: 1,
                parent_node_id: None,
                components: vec![ComponentSnapshot {
                    type_name: "Transform".to_string(),
                    ron: ron::to_string(&Transform {
                        visible: false,
                        position: Vec2::ZERO,
                        pivot: Pivot::TopLeft,
                    })
                    .unwrap(),
                }],
            }],
            ..prefab
        };

        {
            let mut ctx = game.ctx_mut();
            refresh_prefab_instance(&mut ctx, root_entity, &updated_prefab, None);
        }

        let transform = game.ecs.get::<Transform>(root_entity).copied().unwrap();
        assert_eq!(transform.position, Vec2::new(12.0, 34.0));
        assert!(!transform.visible);
        assert_eq!(transform.pivot, Pivot::TopLeft);
    }
}
