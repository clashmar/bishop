use super::selection::is_prefab_entity;
use bishop::prelude::*;
use engine_core::prelude::*;
use std::collections::BTreeMap;

pub(crate) fn draw_prefab_entities<C: BishopContext>(
    ctx: &mut C,
    ecs: &Ecs,
    sprite_manager: &mut SpriteManager,
    grid_size: f32,
) {
    let mut layer_map: BTreeMap<i32, Vec<(Entity, Vec2)>> = BTreeMap::new();

    for (entity, transform) in ecs.get_store::<Transform>().data.iter() {
        if !transform.visible || !is_prefab_entity(ecs, *entity) {
            continue;
        }

        let z = ecs
            .get_store::<Layer>()
            .get(*entity)
            .map_or(0, |layer| layer.z);
        layer_map
            .entry(z)
            .or_default()
            .push((*entity, transform.position));
    }

    for entities in layer_map.into_values() {
        for (entity, position) in entities {
            draw_prefab_entity(ctx, ecs, sprite_manager, entity, position, grid_size);
        }
    }
}

fn draw_prefab_entity<C: BishopContext>(
    ctx: &mut C,
    ecs: &Ecs,
    sprite_manager: &mut SpriteManager,
    entity: Entity,
    pos: Vec2,
    grid_size: f32,
) {
    let visual_entity = resolve_visual_entity(ecs, entity);
    let pivot = ecs
        .get_store::<Transform>()
        .get(entity)
        .map(|transform| transform.pivot)
        .unwrap_or(Pivot::BottomCenter);
    let params = EntityDrawParams {
        pos,
        pivot,
        grid_size,
    };

    if let Some(current_frame) = ecs.get_store::<CurrentFrame>().get(visual_entity) {
        if current_frame.draw(ctx, sprite_manager, &params) {
            return;
        }
    }

    if let Some(sprite) = ecs.get_store::<Sprite>().get(visual_entity) {
        if sprite.draw(ctx, sprite_manager, &params) {
            return;
        }
    }

    if ecs.has_any::<(Light, Glow)>(visual_entity) {
        return;
    }

    let draw_pos = pivot_adjusted_position(pos, Vec2::splat(grid_size), pivot);
    draw_entity_placeholder(ctx, draw_pos, grid_size);
}
