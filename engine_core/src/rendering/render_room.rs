// NOTE: Multi-pass rendering temporarily disabled while rewiring codebase.
use crate::assets::*;
use crate::ecs::components::transform::Pivot;
use crate::ecs::*;
use crate::game::*;
use crate::rendering::*;
use crate::tiles::draw_room_tile_placements;
use crate::worlds::room::Room;
use crate::worlds::*;
use bishop::prelude::*;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Default)]
pub struct LayerData<'a> {
    pub entities: Vec<(Entity, Vec2)>,
    pub glows: Vec<(&'a Glow, Vec2)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisibleRoomLayers {
    pub ordered_layers: Vec<RoomLayer>,
}

#[derive(Default)]
struct CollectedRoomLayerMaps<'a> {
    front: BTreeMap<i32, LayerData<'a>>,
    back: BTreeMap<i32, LayerData<'a>>,
}

impl<'a> CollectedRoomLayerMaps<'a> {
    fn for_layer(&self, room_layer: RoomLayer) -> &BTreeMap<i32, LayerData<'a>> {
        match room_layer {
            RoomLayer::Front => &self.front,
            RoomLayer::Back => &self.back,
        }
    }
}

pub fn visible_layers_for_state(
    layers: &RoomLayers,
    state: RoomRenderState,
) -> VisibleRoomLayers {
    let ordered_layers = match &layers.back {
        None => vec![RoomLayer::Front],
        Some(back) => match (back.composition_mode, state.current_layer) {
            (LayerCompositionMode::Hidden, RoomLayer::Front) => vec![RoomLayer::Front],
            (LayerCompositionMode::Hidden, RoomLayer::Back) => vec![RoomLayer::Back],
            (LayerCompositionMode::DollsHouse, RoomLayer::Front) => {
                vec![RoomLayer::Back, RoomLayer::Front]
            }
            (LayerCompositionMode::DollsHouse, RoomLayer::Back) => {
                vec![RoomLayer::Back, RoomLayer::Front]
            }
        },
    };

    VisibleRoomLayers { ordered_layers }
}

/// Draws everything needed for the given room.
/// Currently uses simplified single-pass rendering.
pub fn render_room<C: BishopContext>(
    ctx: &mut C,
    game_ctx: &mut GameCtxMut<'_>,
    render_cam: &Camera2D,
    state: RoomRenderState,
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
    let composition = RoomCompositionContext::resolve(current_room, state, grid_size);

    // Set up camera and clear background
    ctx.set_camera(render_cam);
    ctx.clear_background(Color::BLACK);

    let variant = current_room.current_variant();
    let visible_layers = visible_layers_for_state(&variant.layers, state);
    let layer_maps = collect_interpolated_room_layer_maps(
        game_ctx.ecs,
        world,
        current_room,
        game_ctx.sprite_manager,
        alpha,
        prev_positions,
        grid_size,
    );

    variant.draw_background(ctx, current_room.position, current_room.size, grid_size);

    for room_layer in visible_layers.ordered_layers {
        draw_room_tile_placements(
            ctx,
            game_ctx.ecs,
            room_layer,
            &composition,
            game_ctx.tile_registry,
            game_ctx.sprite_manager,
        );
        draw_layer_entities(
            ctx,
            game_ctx.ecs,
            game_ctx.sprite_manager,
            layer_maps.for_layer(room_layer),
            grid_size,
            &composition,
            |_, _| true,
        );
    }

    if composition.should_draw_hidden_back_layer_door_ghosts() {
        draw_layer_entities(
            ctx,
            game_ctx.ecs,
            game_ctx.sprite_manager,
            layer_maps.for_layer(RoomLayer::Front),
            grid_size,
            &composition,
            |ecs, entity| ecs.has::<LayerDoor>(entity),
        );
    }

    // TODO: Re-enable multi-pass rendering
    // let lights = collect_lights(ecs, room, alpha, prev_positions);
    // render_system.run_spotlight_pass(ctx, render_cam, lights, room.darkness);
    // render_system.run_final_pass(ctx);
}

fn draw_layer_entities<C: BishopContext, F>(
    ctx: &mut C,
    ecs: &Ecs,
    sprite_manager: &mut SpriteManager,
    layer_map: &BTreeMap<i32, LayerData<'_>>,
    grid_size: f32,
    composition: &RoomCompositionContext,
    include_entity: F,
) where
    F: Fn(&Ecs, Entity) -> bool,
{
    for layer in layer_map.values() {
        for &(entity, pos) in &layer.entities {
            if !include_entity(ecs, entity) {
                continue;
            }

            draw_entity(
                ctx,
                ecs,
                sprite_manager,
                entity,
                pos,
                grid_size,
                composition,
            );
        }

        // TODO: Re-enable multi-pass rendering
        // render_system.run_ambient_pass(ctx, room.darkness);
        // render_system.run_glow_pass(ctx, render_cam, glows, sprite_manager);
        // render_system.run_undarkened_pass(ctx);
        // render_system.run_scene_pass(ctx);
    }
}

fn draw_entity<C: BishopContext>(
    ctx: &mut C,
    ecs: &Ecs,
    sprite_manager: &mut SpriteManager,
    entity: Entity,
    pos: Vec2,
    grid_size: f32,
    composition: &RoomCompositionContext,
) {
    let visual_entity = resolve_visual_entity(ecs, entity);
    let pivot = ecs
        .get_store::<Transform>()
        .get(entity)
        .map(|transform| transform.pivot)
        .unwrap_or(Pivot::BottomCenter);

    let room_layer = ecs.get::<CurrentRoom>(entity).map(|current_room| current_room.layer);

    let color = if room_layer == Some(RoomLayer::Front) {
        let visual_bounds = if ecs.has::<Cover>(entity) || ecs.has::<LayerDoor>(entity) {
            Some(entity_visual_rect(ecs, sprite_manager, entity, pos, grid_size))
        } else {
            None
        };

        let Some(color) = composition
            .front_layer_composition(ecs, entity, visual_bounds)
            .tint()
        else {
            return;
        };

        color
    } else if room_layer == Some(RoomLayer::Back) {
        let visual_bounds = entity_visual_rect(ecs, sprite_manager, entity, pos, grid_size);
        if !composition.back_layer_bounds_visible(visual_bounds) {
            return;
        }
        Color::WHITE
    } else {
        Color::WHITE
    };

    let params = EntityDrawParams {
        pos,
        pivot,
        grid_size,
        color,
    };

    if let Some(cf) = ecs.get_store::<CurrentFrame>().get(visual_entity)
        && cf.draw(ctx, sprite_manager, &params)
    {
        return;
    }

    if let Some(sprite) = ecs.get_store::<Sprite>().get(visual_entity) {
        sprite.draw(ctx, sprite_manager, &params);
    }
}

fn collect_interpolated_room_layer_maps<'a>(
    ecs: &'a Ecs,
    world: &World,
    room: &Room,
    sprite_manager: &SpriteManager,
    alpha: f32,
    prev_positions: Option<&HashMap<Entity, Vec2>>,
    grid_size: f32,
) -> CollectedRoomLayerMaps<'a> {
    let mut maps = CollectedRoomLayerMaps::default();
    let mut seen = HashSet::new();
    let room_ctx = RoomVisibilityContext {
        world,
        room,
        grid_size,
    };

    let trans_store = ecs.get_store::<Transform>();
    let cam_store = ecs.get_store::<RoomCamera>();
    let glow_store = ecs.get_store::<Glow>();
    let sub_pixel_store = ecs.get_store::<SubPixel>();

    for candidate_room_id in spillover_candidate_room_ids(world, room) {
        for &entity in ecs.entities_in_room(candidate_room_id) {
            ecs.assert_room_membership(candidate_room_id, entity);

            if !seen.insert(entity) {
                continue;
            }

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
            let draw_pos = interpolate_position(entity, current_pos, alpha, prev_positions);

            if !entity_visible_in_room(
                ecs,
                sprite_manager,
                entity,
                candidate_room_id,
                draw_pos,
                &room_ctx,
            ) {
                continue;
            }

            let room_layer = ecs
                .get::<CurrentRoom>(entity)
                .map_or(RoomLayer::Front, |current_room| current_room.layer);
            let z = transform.z;
            let layer_map = match room_layer {
                RoomLayer::Front => &mut maps.front,
                RoomLayer::Back => &mut maps.back,
            };
            let entry = layer_map.entry(z).or_default();
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
    }

    for (&z, layer) in &mut maps.front {
        layer.entities.sort_by(|(a, _), (b, _)| compare_entity_draw_order(*a, z, *b, z));
    }
    for (&z, layer) in &mut maps.back {
        layer.entities.sort_by(|(a, _), (b, _)| compare_entity_draw_order(*a, z, *b, z));
    }

    maps
}

// TODO: Re-enable for multi-pass rendering
// fn collect_lights(
//     ecs: &Ecs,
//     room: &Room,
//     alpha: f32,
//     prev_positions: Option<&HashMap<Entity, Vec2>>,
// ) -> Vec<(Vec2, Light)> { ... }

#[cfg(test)]
#[path = "render_room_tests.rs"]
mod render_room_tests;
