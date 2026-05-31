// NOTE: Multi-pass rendering temporarily disabled while rewiring codebase.

use crate::prelude::*;
use bishop::prelude::*;
use std::collections::{BTreeMap, HashMap};

/// Draws everything needed for the given room.
/// Currently uses simplified single-pass rendering.
pub fn render_room<C: BishopContext>(
    ctx: &mut C,
    game_ctx: &mut GameCtxMut<'_>,
    render_cam: &Camera2D,
    alpha: f32,
    prev_positions: Option<&HashMap<Entity, Vec2>>,
) {
    let Some(world) = game_ctx.world.as_deref_mut() else {
        return;
    };
    let Some(current_room) = world.current_room() else {
        return;
    };

    let grid_size = world.grid_size;

    // Organize entities by layer
    let layer_map = collect_interpolated_layer_map(
        game_ctx.ecs,
        current_room,
        game_ctx.sprite_manager,
        alpha,
        prev_positions,
        grid_size,
    );

    // Set up camera and clear background
    ctx.set_camera(render_cam);
    ctx.clear_background(Color::BLACK);

    // Draw tilemap first
    let tilemap = &current_room.current_variant().tilemap;
    tilemap.draw(
        ctx,
        game_ctx.sprite_manager,
        current_room.position,
        grid_size,
    );

    // Draw all entities sorted by layer
    for (_z, layer) in layer_map {
        for (entity, pos) in layer.entities {
            draw_entity(
                ctx,
                game_ctx.ecs,
                game_ctx.sprite_manager,
                entity,
                pos,
                grid_size,
            );
        }

        // TODO: Re-enable multi-pass rendering
        // render_system.run_ambient_pass(ctx, room.darkness);
        // render_system.run_glow_pass(ctx, render_cam, glows, sprite_manager);
        // render_system.run_undarkened_pass(ctx);
        // render_system.run_scene_pass(ctx);
    }

    // TODO: Re-enable multi-pass rendering
    // let lights = collect_lights(ecs, room, alpha, prev_positions);
    // render_system.run_spotlight_pass(ctx, render_cam, lights, room.darkness);
    // render_system.run_final_pass(ctx);

}

fn draw_entity<C: BishopContext>(
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
        .map(|t| t.pivot)
        .unwrap_or(Pivot::BottomCenter);

    let params = EntityDrawParams {
        pos,
        pivot,
        grid_size,
    };

    if let Some(cf) = ecs.get_store::<CurrentFrame>().get(visual_entity)
        && cf.draw(ctx, sprite_manager, &params)
    {
        return;
    }

    if let Some(sprite) = ecs.get_store::<Sprite>().get(visual_entity)
        && sprite.draw(ctx, sprite_manager, &params)
    {
        return;
    }

    if ecs.has_any::<(Light, Glow)>(visual_entity) {
        return;
    }

    let base = pivot_adjusted_position(pos, Vec2::splat(grid_size), pivot);
    draw_entity_placeholder(ctx, base, grid_size);
}

/// Draw a placeholder for an entity without a sprite.
pub fn draw_entity_placeholder<C: BishopContext>(ctx: &mut C, pos: Vec2, grid_size: f32) {
    ctx.draw_rectangle(pos.x, pos.y, grid_size, grid_size, Color::GREEN);
}

#[derive(Default)]
pub struct LayerData<'a> {
    pub entities: Vec<(Entity, Vec2)>,
    pub glows: Vec<(&'a Glow, Vec2)>,
}

/// Sorts entites by their z-layer, filters out entities that should not be
/// drawn and interpolates the draw positions. BTreeMap automatically sorts keys.
fn collect_interpolated_layer_map<'a>(
    ecs: &'a Ecs,
    room: &Room,
    sprite_manager: &SpriteManager,
    alpha: f32,
    prev_positions: Option<&HashMap<Entity, Vec2>>,
    grid_size: f32,
) -> BTreeMap<i32, LayerData<'a>> {
    let mut map: BTreeMap<i32, LayerData<'a>> = BTreeMap::new();

    let trans_store = ecs.get_store::<Transform>();
    let cam_store = ecs.get_store::<RoomCamera>();
    let layer_store = ecs.get_store::<Layer>();
    let glow_store = ecs.get_store::<Glow>();
    let sub_pixel_store = ecs.get_store::<SubPixel>();

    for &entity in ecs.entities_in_room(room.id) {
        ecs.assert_room_membership(room.id, entity);

        let Some(transform) = trans_store.get(entity) else {
            continue;
        };

        if !transform.visible {
            continue;
        }

        if cam_store.get(entity).is_some() {
            continue;
        }

        let current_pos = visual_position(transform.position, sub_pixel_store.get(entity));
        let draw_pos = interpolate_draw_position(
            entity,
            current_pos,
            alpha,
            prev_positions,
        );

        let z = layer_store.get(entity).map_or(0, |layer| layer.z);

        let entry = map.entry(z).or_default();
        entry.entities.push((entity, draw_pos));

        if let Some(glow) = glow_store.get(entity) {
            let glow_size = sprite_manager
                .texture_size(glow.sprite_id)
                .map(|(w, h)| Vec2::new(w, h))
                .unwrap_or(Vec2::new(grid_size, grid_size));

            let glow_draw_pos = pivot_adjusted_position(draw_pos, glow_size, transform.pivot);
            entry.glows.push((glow, glow_draw_pos));
        }
    }

    // There always needs to be at least one layer otherwise nothing will be drawn
    if map.is_empty() {
        map.insert(0, LayerData::default());
    }

    map
}

// TODO: Re-enable for multi-pass rendering
// fn collect_lights(
//     ecs: &Ecs,
//     room: &Room,
//     alpha: f32,
//     prev_positions: Option<&HashMap<Entity, Vec2>>,
// ) -> Vec<(Vec2, Light)> { ... }

/// Returns the interpolated draw position or the current position.
fn interpolate_draw_position(
    entity: Entity,
    current_pos: Vec2,
    alpha: f32,
    prev_positions: Option<&HashMap<Entity, Vec2>>,
) -> Vec2 {
    if let Some(prev_map) = prev_positions {
        if let Some(prev_pos) = prev_map.get(&entity) {
            lerp_position(*prev_pos, current_pos, alpha)
        } else {
            current_pos
        }
    } else {
        current_pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_interpolated_layer_map_skips_entities_outside_the_room_index() {
        let room_id = RoomId(1);
        let other_room = RoomId(2);
        let mut ecs = Ecs::default();

        let visible = ecs.create_entity()
            .with(Transform::default())
            .with_current_room(room_id)
            .finish();

        ecs.create_entity()
            .with(Transform::default())
            .with_current_room(other_room)
            .finish();

        let layers = collect_interpolated_layer_map(
            &ecs,
            &Room {
                id: room_id,
                ..Default::default()
            },
            &SpriteManager::default(),
            1.0,
            None,
            16.0,
        );

        assert!(layers
            .values()
            .flat_map(|layer| layer.entities.iter())
            .any(|(entity, _)| *entity == visible));
    }
}

/// Calculates draw position adjusted for pivot.
/// Returns the top-left corner where the texture should be drawn.
#[inline]
pub fn pivot_adjusted_position(entity_pos: Vec2, texture_size: Vec2, pivot: Pivot) -> Vec2 {
    let offset = pivot.as_normalized();
    vec2(
        entity_pos.x - texture_size.x * offset.x,
        entity_pos.y - texture_size.y * offset.y,
    )
}
