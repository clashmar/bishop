// engine_core/src/rendering/render_room.rs
// NOTE: Multi-pass rendering temporarily disabled while rewiring codebase.

use crate::prelude::*;
use bishop::prelude::*;
use std::collections::{BTreeMap, HashMap};

/// Draws everything needed for the given room.
/// Currently uses simplified single-pass rendering.
pub fn render_room<C: BishopContext>(
    ctx: &mut C,
    game_ctx: &mut GameCtxMut<'_>,
    render_system: &mut RenderSystem,
    render_cam: &Camera2D,
    alpha: f32,
    prev_positions: Option<&HashMap<Entity, Vec2>>,
) {
    let render_start = std::time::Instant::now();
    let Some(cur_world) = game_ctx.cur_world.as_deref_mut() else {
        return;
    };
    let Some(current_room) = cur_world.current_room() else {
        return;
    };

    let grid_size = cur_world.grid_size;

    // Organize entities by layer
    let layer_map = collect_interpolated_layer_map(
        game_ctx.ecs,
        current_room,
        game_ctx.asset_manager,
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
        game_ctx.asset_manager,
        current_room.position,
        grid_size,
    );

    // Draw all entities sorted by layer
    for (_z, layer) in layer_map {
        for (entity, pos) in layer.entities {
            draw_entity(
                ctx,
                game_ctx.ecs,
                game_ctx.asset_manager,
                entity,
                pos,
                grid_size,
            );
        }

        // TODO: Re-enable multi-pass rendering
        // render_system.run_ambient_pass(ctx, room.darkness);
        // render_system.run_glow_pass(ctx, render_cam, glows, asset_manager);
        // render_system.run_undarkened_pass(ctx);
        // render_system.run_scene_pass(ctx);
    }

    // TODO: Re-enable multi-pass rendering
    // let lights = collect_lights(ecs, room, alpha, prev_positions);
    // render_system.run_spotlight_pass(ctx, render_cam, lights, room.darkness);
    // render_system.run_final_pass(ctx);

    render_system.render_time_ms = render_start.elapsed().as_secs_f32() * 1000.0;
}

fn draw_entity<C: BishopContext>(
    ctx: &mut C,
    ecs: &Ecs,
    asset_manager: &mut AssetManager,
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
        && cf.draw(ctx, asset_manager, &params)
    {
        return;
    }

    if let Some(sprite) = ecs.get_store::<Sprite>().get(visual_entity)
        && sprite.draw(ctx, asset_manager, &params)
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
    asset_manager: &AssetManager,
    alpha: f32,
    prev_positions: Option<&HashMap<Entity, Vec2>>,
    grid_size: f32,
) -> BTreeMap<i32, LayerData<'a>> {
    let mut map: BTreeMap<i32, LayerData<'a>> = BTreeMap::new();

    let trans_store = ecs.get_store::<Transform>();
    let cam_store = ecs.get_store::<RoomCamera>();
    let room_store = ecs.get_store::<CurrentRoom>();
    let layer_store = ecs.get_store::<Layer>();
    let glow_store = ecs.get_store::<Glow>();

    for (entity, transform) in &trans_store.data {
        // Skip invisible entities
        if !transform.visible {
            continue;
        }

        // Skip camera
        if cam_store.get(*entity).is_some() {
            continue;
        }

        // Filter by current room
        if let Some(cr) = room_store.get(*entity) {
            if cr.0 != room.id {
                continue;
            }
        } else {
            continue;
        }

        let draw_pos =
            interpolate_draw_position(*entity, transform.position, alpha, prev_positions);

        // Default layer is 0 if missing
        let z = layer_store.get(*entity).map_or(0, |l| l.z);

        let entry = map.entry(z).or_default();
        entry.entities.push((*entity, draw_pos));

        // If the entity also has a Glow component, apply pivot to glow position
        if let Some(glow) = glow_store.get(*entity) {
            let glow_size = asset_manager
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
            lerp_rounded(*prev_pos, current_pos, alpha)
        } else {
            current_pos
        }
    } else {
        current_pos
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
