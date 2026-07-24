use std::collections::HashSet;

use crate::ecs::capture::{ComponentSnapshot, capture_entity};
use crate::ecs::component_registry::{COMPONENTS, component_has_dependents};
use crate::ecs::{Animation, CurrentFrame, Ecs, Entity, Script, Solid, TilePlacement};
use crate::game::GameCtxMut;
use crate::tiles::TileDefId;

/// First-class ECS component types legal on tile definitions in Phase 1.
pub const TILE_DEFINITION_COMPONENT_TYPES: &[&str] = &[
    Solid::TYPE_NAME,
    Animation::TYPE_NAME,
    Script::TYPE_NAME,
];

/// Returns true when `type_name` is authored directly on tile definitions.
pub fn tile_definition_component_allowed(type_name: &str) -> bool {
    TILE_DEFINITION_COMPONENT_TYPES.contains(&type_name)
}

/// Captures only tile-definition-authored components from `entity`.
pub fn capture_tile_definition_components(ecs: &mut Ecs, entity: Entity) -> Vec<ComponentSnapshot> {
    let mut components: Vec<_> = capture_entity(ecs, entity)
        .into_iter()
        .filter(|snapshot| tile_definition_component_allowed(&snapshot.type_name))
        .collect();
    components.sort_by(|left, right| left.type_name.cmp(&right.type_name));
    components
}

/// Adds one authored tile-definition component to `entity`.
pub fn add_tile_definition_component_by_type_name(
    ctx: &mut GameCtxMut<'_>,
    entity: Entity,
    type_name: &str,
) {
    if !tile_definition_component_allowed(type_name) {
        return;
    }

    let Some(reg) = COMPONENTS.iter().find(|reg| reg.type_name == type_name) else {
        return;
    };

    (reg.factory)(ctx.ecs(), entity);

    if !(reg.has)(ctx.ecs(), entity) {
        return;
    }

    let mut boxed = (reg.clone)(ctx.ecs(), entity);
    (reg.post_create)(&mut *boxed, &entity, ctx);
    (reg.inserter)(ctx.ecs(), entity, boxed);
    ctx.ecs().run_registered_on_insert(reg, entity);
}

/// Removes one authored tile-definition component from `entity`.
pub fn remove_tile_definition_component_by_type_name(
    ctx: &mut GameCtxMut<'_>,
    entity: Entity,
    type_name: &str,
) {
    if !tile_definition_component_allowed(type_name) {
        return;
    }

    if type_name == Animation::TYPE_NAME {
        Ecs::remove_component::<CurrentFrame>(ctx, entity);
    }

    Ecs::remove_component_by_type_name(ctx, entity, type_name);
    prune_hidden_tile_dependencies(ctx, entity, &[type_name]);
}

/// Replaces all authored tile-definition components on `entity`.
pub fn replace_tile_definition_components(
    ctx: &mut GameCtxMut<'_>,
    entity: Entity,
    components: &[ComponentSnapshot],
) {
    let removed = clear_tile_definition_components(ctx, entity);

    for snapshot in components {
        restore_component_snapshot(ctx, entity, snapshot);
    }

    if !removed.is_empty() {
        prune_hidden_tile_dependencies(ctx, entity, &removed);
    }
}

/// Applies definition-owned components from `tile_id` onto `entity`.
pub fn apply_tile_definition_to_entity(
    ctx: &mut GameCtxMut<'_>,
    entity: Entity,
    tile_id: TileDefId,
) {
    let components = ctx
        .tile_registry
        .get(tile_id)
        .map(|def| def.components.clone())
        .unwrap_or_default();

    replace_tile_definition_components(ctx, entity, &components);
}

/// Applies the linked placement definition onto `entity` when it has `TilePlacement`.
pub fn apply_tile_placement_definition(ctx: &mut GameCtxMut<'_>, entity: Entity) {
    let Some(tile_id) = ctx.ecs.get::<TilePlacement>(entity).map(|placement| placement.definition)
    else {
        return;
    };

    apply_tile_definition_to_entity(ctx, entity, tile_id);
}

/// Re-applies one tile definition to every linked placement.
pub(crate) fn sync_tile_definition_in_ctx(ctx: &mut GameCtxMut<'_>, tile_id: TileDefId) {
    let linked_entities: Vec<_> = ctx
        .ecs
        .get_store::<TilePlacement>()
        .data
        .iter()
        .filter_map(|(&entity, placement)| (placement.definition == tile_id).then_some(entity))
        .collect();

    for entity in linked_entities {
        apply_tile_definition_to_entity(ctx, entity, tile_id);
    }
}

/// Rebuilds definition-owned components for every stored tile placement.
pub(crate) fn sync_all_tile_placements_in_ctx(ctx: &mut GameCtxMut<'_>) {
    let tile_entities: Vec<_> = ctx.ecs.get_store::<TilePlacement>().data.keys().copied().collect();

    for entity in tile_entities {
        apply_tile_placement_definition(ctx, entity);
    }
}

fn clear_tile_definition_components(ctx: &mut GameCtxMut<'_>, entity: Entity) -> Vec<&'static str> {
    let mut removed = Vec::new();

    for &type_name in TILE_DEFINITION_COMPONENT_TYPES {
        let Some(reg) = COMPONENTS.iter().find(|reg| reg.type_name == type_name) else {
            continue;
        };

        if !(reg.has)(ctx.ecs(), entity) {
            continue;
        }

        if type_name == Animation::TYPE_NAME {
            Ecs::remove_component::<CurrentFrame>(ctx, entity);
        }

        Ecs::remove_component_by_type_name(ctx, entity, type_name);
        removed.push(type_name);
    }

    removed
}

fn restore_component_snapshot(
    ctx: &mut GameCtxMut<'_>,
    entity: Entity,
    snapshot: &ComponentSnapshot,
) {
    if !tile_definition_component_allowed(&snapshot.type_name) {
        return;
    }

    let Some(reg) = COMPONENTS.iter().find(|reg| reg.type_name == snapshot.type_name) else {
        return;
    };

    let mut boxed = (reg.from_ron_component)(snapshot.ron.clone());
    (reg.post_create)(&mut *boxed, &entity, ctx);
    (reg.inserter)(ctx.ecs(), entity, boxed);
    ctx.ecs().run_registered_on_insert(reg, entity);
}

fn prune_hidden_tile_dependencies(
    ctx: &mut GameCtxMut<'_>,
    entity: Entity,
    removed_type_names: &[&str],
) {
    let pending = dependency_closure(removed_type_names);

    loop {
        let mut removed_any = false;

        for &type_name in &pending {
            if tile_definition_component_allowed(type_name) {
                continue;
            }

            let Some(reg) = COMPONENTS.iter().find(|reg| reg.type_name == type_name) else {
                continue;
            };

            if !(reg.has)(ctx.ecs(), entity) || component_has_dependents(type_name, entity, ctx.ecs()) {
                continue;
            }

            Ecs::remove_component_by_type_name(ctx, entity, type_name);
            removed_any = true;
        }

        if !removed_any {
            break;
        }
    }
}

fn dependency_closure(type_names: &[&str]) -> Vec<&'static str> {
    let mut pending: Vec<&'static str> = type_names
        .iter()
        .filter_map(|type_name| COMPONENTS.iter().find(|reg| reg.type_name == *type_name))
        .flat_map(|reg| reg.deps.iter().copied())
        .collect();
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    while let Some(dep_type_name) = pending.pop() {
        if !seen.insert(dep_type_name) {
            continue;
        }

        result.push(dep_type_name);

        if let Some(reg) = COMPONENTS.iter().find(|reg| reg.type_name == dep_type_name) {
            pending.extend(reg.deps.iter().copied());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::{Name, SpriteId};
    use crate::game::Game;
    use crate::tiles::{TileDef, tile_definition_component_snapshot};
    use crate::worlds::RoomId;

    #[test]
    fn tile_definition_reflow_when_definition_changes_then_linked_placements_refresh_definition_owned_components() {
        let mut game = Game::default();
        let tile_id = game.tile_registry.insert(TileDef {
            sprite_id: SpriteId(1),
            components: vec![tile_definition_component_snapshot(Solid(true))],
        });
        let entity = game
            .ecs
            .create_entity()
            .with(TilePlacement::new(tile_id, 0, 0))
            .with_current_room(RoomId(1))
            .finish();

        {
            let mut ctx = game.ctx_mut();
            apply_tile_definition_to_entity(&mut ctx, entity, tile_id);
        }
        assert!(game.ecs.get::<Solid>(entity).is_some_and(|solid| solid.0));

        game.tile_registry.replace(
            tile_id,
            TileDef {
                sprite_id: SpriteId(1),
                components: Vec::new(),
            },
        );
        {
            let mut ctx = game.ctx_mut();
            sync_tile_definition_in_ctx(&mut ctx, tile_id);
        }

        assert!(!game.ecs.has::<Solid>(entity));
    }

    #[test]
    fn tile_definition_reflow_when_runtime_state_exists_then_runtime_state_is_preserved() {
        let mut game = Game::default();
        let tile_id = game.tile_registry.insert(TileDef {
            sprite_id: SpriteId(1),
            components: Vec::new(),
        });
        let entity = game
            .ecs
            .create_entity()
            .with(TilePlacement::new(tile_id, 1, 1))
            .with_current_room(RoomId(1))
            .with(Name("Cracked".into()))
            .finish();

        {
            let mut ctx = game.ctx_mut();
            apply_tile_definition_to_entity(&mut ctx, entity, tile_id);
        }

        game.tile_registry.replace(
            tile_id,
            TileDef {
                sprite_id: SpriteId(1),
                components: vec![tile_definition_component_snapshot(Solid(true))],
            },
        );
        {
            let mut ctx = game.ctx_mut();
            sync_tile_definition_in_ctx(&mut ctx, tile_id);
        }

        assert_eq!(game.ecs.get::<Name>(entity).map(|name| name.0.as_str()), Some("Cracked"));
        assert!(game.ecs.get::<Solid>(entity).is_some_and(|solid| solid.0));
    }
}
