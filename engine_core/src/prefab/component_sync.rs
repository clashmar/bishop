#[cfg(feature = "editor")]
use super::{PrefabInstanceNode, PrefabInstanceRoot, PrefabOverrides};
use crate::ecs::capture::ComponentSnapshot;
use crate::ecs::component::comp_type_name;
#[cfg(feature = "editor")]
use crate::ecs::capture::{capture_entity, restore_entity};
#[cfg(feature = "editor")]
use crate::ecs::component_registry::ComponentRegistry;
#[cfg(feature = "editor")]
use crate::ecs::components::hierarchy::{Children, Parent};
#[cfg(feature = "editor")]
use crate::ecs::entity::Entity;
#[cfg(feature = "editor")]
use crate::ecs::{CurrentFrame, Global, Player, PlayerProxy, RoomCamera};
use crate::ecs::{CurrentRoom, Transform};
#[cfg(feature = "editor")]
use crate::game::GameCtxMut;
use crate::prefab::PrefabNode;
use crate::worlds::room::RoomId;
use bishop::prelude::*;
#[cfg(feature = "editor")]
use std::collections::HashSet;

pub(super) fn instantiate_prefab_components(
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

#[cfg(feature = "editor")]
pub(super) fn apply_prefab_node(
    ctx: &mut GameCtxMut<'_>,
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

        if is_instance_root && component.type_name == comp_type_name::<Transform>() {
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

#[cfg(feature = "editor")]
pub(super) fn excluded_from_prefab_asset(type_name: &str) -> bool {
    type_name == comp_type_name::<Children>()
        || type_name == comp_type_name::<Parent>()
        || type_name == comp_type_name::<CurrentRoom>()
        || type_name == comp_type_name::<CurrentFrame>()
        || type_name == comp_type_name::<RoomCamera>()
        || type_name == comp_type_name::<PlayerProxy>()
        || type_name == comp_type_name::<Player>()
        || type_name == comp_type_name::<Global>()
        || type_name == comp_type_name::<PrefabInstanceRoot>()
        || type_name == comp_type_name::<PrefabInstanceNode>()
        || type_name == comp_type_name::<PrefabOverrides>()
}

pub(super) fn translate_transform_snapshot(
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

#[cfg(feature = "editor")]
fn apply_component_snapshot(ctx: &mut GameCtxMut<'_>, entity: Entity, component: ComponentSnapshot) {
    remove_component_snapshot(ctx, entity, &component.type_name);
    restore_entity(ctx, entity, vec![component]);
}

#[cfg(feature = "editor")]
fn remove_component_snapshot(ctx: &mut GameCtxMut<'_>, entity: Entity, type_name: &str) {
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
    (component_reg.on_remove)(&mut *boxed, &entity, ctx.ecs());
    (component_reg.remove)(ctx.ecs(), entity);
}

#[cfg(feature = "editor")]
fn apply_root_transform_snapshot(ctx: &mut GameCtxMut<'_>, entity: Entity, component: &ComponentSnapshot) {
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

#[cfg(feature = "editor")]
fn remove_stale_prefab_components(
    ctx: &mut GameCtxMut<'_>,
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
        let is_reserved_type = excluded_from_prefab_asset(&component.type_name)
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