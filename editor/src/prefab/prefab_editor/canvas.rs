use super::selection::is_prefab_entity;
use bishop::prelude::*;
use crate::editor_assets::assets::{entity_icon, entry_icon, exit_icon, portal_icon};
use crate::shared::entity_icon::{
    draw_camera_icon, draw_glow_placeholder, draw_light_placeholder, resolve_entity_visual,
    EntityVisual, PLACEHOLDER_OPACITY,
};
use engine_core::assets::*;
use engine_core::ecs::*;
use engine_core::rendering::{EntityDrawParams, Renderable, pivot_adjusted_position, resolve_visual_entity};
use std::collections::BTreeMap;

/// Draws all visible prefab entities sorted by layer z-order.
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
        color: Color::WHITE,
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

    let draw_pos = pivot_adjusted_position(pos, Vec2::splat(grid_size), pivot);

    match resolve_entity_visual(ecs, entity) {
        EntityVisual::SpriteOrAnimation => {}
        EntityVisual::CameraIcon => {
            draw_camera_icon(ctx, pos, grid_size);
        }
        visual @ (EntityVisual::PortalIcon | EntityVisual::EntryIcon | EntityVisual::ExitIcon) => {
            let icon = match visual {
                EntityVisual::PortalIcon => portal_icon(),
                EntityVisual::EntryIcon => entry_icon(),
                EntityVisual::ExitIcon => exit_icon(),
                _ => unreachable!(),
            };
            ctx.draw_texture_ex(
                icon,
                draw_pos.x,
                draw_pos.y,
                Color::new(1.0, 1.0, 1.0, PLACEHOLDER_OPACITY),
                DrawTextureParams {
                    dest_size: Some(vec2(grid_size, grid_size)),
                    ..Default::default()
                },
            );
        }
        EntityVisual::LightPlaceholder => {
            draw_light_placeholder(ctx, pos, grid_size);
        }
        EntityVisual::GlowPlaceholder => {
            draw_glow_placeholder(ctx, sprite_manager, ecs, entity, pos, grid_size);
        }
        EntityVisual::GenericPlaceholder => {
            ctx.draw_texture_ex(
                entity_icon(),
                draw_pos.x,
                draw_pos.y,
                Color::new(1.0, 1.0, 1.0, PLACEHOLDER_OPACITY),
                DrawTextureParams {
                    dest_size: Some(vec2(grid_size, grid_size)),
                    ..Default::default()
                },
            );
        }
    }
}
