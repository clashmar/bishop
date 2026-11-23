use std::collections::HashMap;

use bishop::prelude::{
    AmbientPassParams, BishopContext, Camera2D, Color, PointLightPassParams, Vec2, Vec3,
};
use crate::assets::SpriteManager;
use crate::ecs::{Ecs, Entity, LayerDoor, PointLightStyle};
use crate::game::GameCtxMut;
use crate::rendering::helpers::RoomVisibilityContext;
use crate::rendering::lighting::{
    ambient_multiplier, point_light_affects_layer, point_light_attenuation,
    point_light_pixel_size_in_target,
};
use crate::rendering::render_system_wgpu::RenderSystem;
use crate::rendering::room_composition::{RoomCompositionContext, RoomRenderState};
use crate::rendering::room_layer_draw::{
    CollectedRoomLayerMaps, LayerDrawItem, collect_interpolated_room_layer_maps,
    draw_layer_items,
};
use crate::rendering::room_layer_rendering::{
    ContributionLightingPlan, LayerLightingBuffer, RoomLightingContribution, RoutedPointLight,
    build_room_lighting_plan,
};
use crate::tiles::{TileRegistry, draw_room_tile_placements};
use crate::worlds::{LayerCompositionMode, RoomLayer, RoomLayers, RoomVariant};
use crate::rendering::particles::ParticleSystem;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisibleRoomLayers {
    pub ordered_layers: Vec<RoomLayer>,
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
            (LayerCompositionMode::DollsHouse, RoomLayer::Front)
            | (LayerCompositionMode::DollsHouse, RoomLayer::Back) => {
                vec![RoomLayer::Back, RoomLayer::Front]
            }
        },
    };

    VisibleRoomLayers { ordered_layers }
}

/// Draws the room through isolated background/layer lighting contributions.
pub fn render_room<C: BishopContext>(
    ctx: &mut C,
    render_system: &mut RenderSystem,
    game_ctx: &mut GameCtxMut<'_>,
    render_cam: &Camera2D,
    state: RoomRenderState,
    alpha: f32,
    prev_positions: Option<&HashMap<Entity, Vec2>>,
    particles: Option<&ParticleSystem>,
) -> bool {
    let Some(world) = game_ctx.world.as_deref_mut() else {
        return false;
    };
    let Some(current_room) = world.current_room() else {
        return false;
    };

    let grid_size = world.grid_size;
    let room_visibility = RoomVisibilityContext {
        world,
        room: current_room,
        grid_size,
    };
    let composition = RoomCompositionContext::resolve(current_room, state, grid_size);
    let room_position = current_room.position;
    let room_size = current_room.size;
    let variant = current_room.current_variant();
    let visible_layers = visible_layers_for_state(&variant.layers, state);
    let draw_door_ghosts = composition.should_draw_hidden_back_layer_door_ghosts();
    let layer_maps = collect_interpolated_room_layer_maps(
        game_ctx.ecs,
        &room_visibility,
        game_ctx.sprite_manager,
        alpha,
        prev_positions,
        particles,
    );

    let ambient = room_ambient_params(
        current_room.darkness,
        current_room.ambient_color,
        current_room.exposure,
    );

    let target_to_display_scale =
        render_system.viewport_rect(ctx).w / render_system.rt_width.max(1.0);
    let routed_lights = prepare_routed_lights(
        render_cam,
        render_system.rt_width,
        render_system.rt_height,
        target_to_display_scale,
        &visible_layers.ordered_layers,
        &layer_maps,
    );

    let plan = build_room_lighting_plan(
        &visible_layers.ordered_layers,
        &routed_lights,
        draw_door_ghosts,
    );

    let mut room_draw = RoomSceneDrawContext {
        ecs: game_ctx.ecs,
        tile_registry: game_ctx.tile_registry,
        sprite_manager: game_ctx.sprite_manager,
        render_cam,
        composition: &composition,
        variant,
        room_position,
        room_size,
        layer_maps: &layer_maps,
        grid_size,
        visible_layers: &visible_layers.ordered_layers,
        includes_door_ghosts: draw_door_ghosts,
    };

    if !render_system.prepare_room(ctx, &plan) {
        draw_unlit_fallback(ctx, render_system, &mut room_draw);
        return false;
    }

    let (lit_buffers, lighting_succeeded) =
        render_room_contributions(ctx, render_system, &mut room_draw, &plan, ambient);
    if render_system.composite_room(ctx, &plan, &lit_buffers, lighting_succeeded) {
        true
    } else {
        draw_unlit_fallback(ctx, render_system, &mut room_draw);
        false
    }
}

fn prepare_routed_lights(
    render_cam: &Camera2D,
    target_width: f32,
    target_height: f32,
    target_to_display_scale: f32,
    visible_layers: &[RoomLayer],
    layer_maps: &CollectedRoomLayerMaps<'_>,
) -> Vec<RoutedPointLight> {
    let mut lights = Vec::new();
    for &owner_layer in visible_layers {
        for layer_data in layer_maps.for_layer(owner_layer).values() {
            for (entity, light, world_position) in &layer_data.lighting.point_lights {
                let has_visible_target = visible_layers.iter().any(|target_layer| {
                    point_light_affects_layer(light, owner_layer, *target_layer, visible_layers)
                });
                let radius = finite_nonnegative(light.radius, 0.0);
                let intensity = finite_nonnegative(light.intensity, 0.0);
                if !world_position.is_finite()
                    || !has_visible_target
                    || point_light_attenuation(0.0, radius, light.attenuation) <= 0.0
                    || intensity <= 0.0
                {
                    continue;
                }
                let center = render_cam.world_to_screen(*world_position, target_width, target_height);
                let radius_edge = render_cam.world_to_screen(
                    *world_position + Vec2::new(radius, 0.0),
                    target_width,
                    target_height,
                );
                lights.push(RoutedPointLight {
                    entity: *entity,
                    owner_layer,
                    scope: light.layer_scope,
                    params: PointLightPassParams {
                        center: [center.x, center.y],
                        color: [
                            finite_nonnegative(light.color.x, 0.0),
                            finite_nonnegative(light.color.y, 0.0),
                            finite_nonnegative(light.color.z, 0.0),
                        ],
                        intensity,
                        radius: center.distance(radius_edge),
                        attenuation: finite_nonnegative(light.attenuation, 1.0),
                        pixelated: light.style == PointLightStyle::Pixel,
                        pixel_size: point_light_pixel_size_in_target(
                            light.pixel_size,
                            target_to_display_scale,
                        ),
                    },
                });
            }
        }
    }
    lights
}

fn finite_nonnegative(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        fallback
    }
}

fn draw_room_layer<C: BishopContext>(
    ctx: &mut C,
    ecs: &Ecs,
    tile_registry: &mut TileRegistry,
    sprite_manager: &mut SpriteManager,
    composition: &RoomCompositionContext,
    layer_maps: &CollectedRoomLayerMaps<'_>,
    room_layer: RoomLayer,
    grid_size: f32,
) {
    draw_room_tile_placements(ctx, ecs, room_layer, composition, tile_registry, sprite_manager);
    draw_layer_items(
        ctx,
        ecs,
        sprite_manager,
        layer_maps.for_layer(room_layer),
        grid_size,
        composition,
        |_, _| true,
    );
}

struct RoomSceneDrawContext<'context, 'maps> {
    ecs: &'context Ecs,
    tile_registry: &'context mut TileRegistry,
    sprite_manager: &'context mut SpriteManager,
    render_cam: &'context Camera2D,
    composition: &'context RoomCompositionContext,
    variant: &'context RoomVariant,
    room_position: Vec2,
    room_size: Vec2,
    layer_maps: &'context CollectedRoomLayerMaps<'maps>,
    grid_size: f32,
    visible_layers: &'context [RoomLayer],
    includes_door_ghosts: bool,
}

impl RoomSceneDrawContext<'_, '_> {
    fn draw_contribution<C: BishopContext>(
        &mut self,
        ctx: &mut C,
        render_system: &RenderSystem,
        contribution: RoomLightingContribution,
    ) {
        render_system.begin_contribution(ctx, contribution);
        ctx.set_camera(self.render_cam);
        match contribution {
            RoomLightingContribution::Background => self.variant.draw_background(
                ctx,
                self.room_position,
                self.room_size,
                self.grid_size,
            ),
            RoomLightingContribution::Layer(room_layer) => self.draw_layer(ctx, room_layer),
            RoomLightingContribution::DoorGhosts => self.draw_door_ghosts(ctx),
        }
        render_system.end_contribution(ctx);
    }

    fn draw_unlit<C: BishopContext>(&mut self, ctx: &mut C) {
        ctx.set_camera(self.render_cam);
        ctx.clear_background(Color::BLACK);
        self.variant.draw_background(
            ctx,
            self.room_position,
            self.room_size,
            self.grid_size,
        );
        for &room_layer in self.visible_layers {
            self.draw_layer(ctx, room_layer);
        }
        if self.includes_door_ghosts {
            self.draw_door_ghosts(ctx);
        }
    }

    fn draw_layer<C: BishopContext>(&mut self, ctx: &mut C, room_layer: RoomLayer) {
        draw_room_layer(
            ctx,
            self.ecs,
            self.tile_registry,
            self.sprite_manager,
            self.composition,
            self.layer_maps,
            room_layer,
            self.grid_size,
        );
    }

    fn draw_door_ghosts<C: BishopContext>(&mut self, ctx: &mut C) {
        draw_layer_items(
            ctx,
            self.ecs,
            self.sprite_manager,
            self.layer_maps.for_layer(RoomLayer::Front),
            self.grid_size,
            self.composition,
            |ecs, item| {
                matches!(item, LayerDrawItem::Entity { entity, .. } if ecs.has::<LayerDoor>(*entity))
            },
        );
    }
}

fn render_room_contributions<C: BishopContext>(
    ctx: &mut C,
    render_system: &mut RenderSystem,
    room_draw: &mut RoomSceneDrawContext<'_, '_>,
    plan: &[ContributionLightingPlan],
    ambient: AmbientPassParams,
) -> (HashMap<RoomLightingContribution, LayerLightingBuffer>, bool) {
    let mut lit_buffers = HashMap::new();
    let mut lighting_succeeded = true;

    for entry in plan {
        let contribution = entry.contribution;
        room_draw.draw_contribution(ctx, render_system, contribution);
        if !lighting_succeeded || contribution == RoomLightingContribution::DoorGhosts {
            continue;
        }

        let point_lights = entry
            .point_lights
            .iter()
            .map(|light| light.params)
            .collect::<Vec<_>>();
        match render_system.light_contribution(ctx, contribution, ambient, &point_lights) {
            Ok(buffer) => {
                lit_buffers.insert(contribution, buffer);
            }
            Err(error) => {
                RenderSystem::report_pass_failure(&error);
                lighting_succeeded = false;
            }
        }
    }

    (lit_buffers, lighting_succeeded)
}

fn draw_unlit_fallback<C: BishopContext>(
    ctx: &mut C,
    render_system: &mut RenderSystem,
    room_draw: &mut RoomSceneDrawContext<'_, '_>,
) {
    render_system.draw_unlit_room_fallback();
    room_draw.draw_unlit(ctx);
}

fn room_ambient_params(darkness: f32, color: Color, exposure: f32) -> AmbientPassParams {
    let ambient_color = Vec3::new(color.r, color.g, color.b);
    let ambient_rgb = ambient_multiplier(darkness, ambient_color, exposure);
    AmbientPassParams {
        color: [ambient_rgb.x, ambient_rgb.y, ambient_rgb.z],
        exposure: 1.0,
        darkness: 0.0,
    }
}

#[cfg(test)]
#[path = "tests/render_room_tests.rs"]
mod render_room_tests;
#[cfg(test)]
#[path = "tests/particle_render_tests.rs"]
mod particle_render_tests;
