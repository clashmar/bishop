use std::collections::HashMap;

use bishop::prelude::{
    AmbientPassParams, BishopContext, Camera2D, Color, PointLightPassParams, Rect, Vec2, Vec3,
};
use crate::assets::SpriteManager;
use crate::ecs::{Ecs, Entity, LayerDoor, PointLightStyle};
use crate::game::GameCtxMut;
use crate::rendering::helpers::RoomVisibilityContext;
use crate::rendering::lighting::{
    GlowPassParams, GlowVisual, ambient_multiplier, light_scope_affects_layer,
    point_light_affects_layer, point_light_attenuation, point_light_pixel_size_in_target,
};
use crate::rendering::render_system_wgpu::RenderSystem;
use crate::rendering::renderable::draw_emission_mask;
use crate::rendering::room_composition::{RoomCompositionContext, RoomRenderState};
use crate::rendering::room_layer_draw::{
    CollectedRoomLayerMaps, LayerDrawItem, collect_interpolated_room_layer_maps,
    draw_layer_items,
};
use crate::rendering::room_layer_rendering::{
    ContributionLightingPlan, LayerLightingBuffer, RoomLightingContribution, RoutedGlow,
    RoutedPointLight, build_room_lighting_plan,
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
    let visible_glow_diagnostics = layer_maps
        .glow_source_diagnostics
        .iter()
        .filter(|issue| visible_layers.ordered_layers.contains(&issue.owner_layer))
        .copied()
        .collect::<Vec<_>>();
    render_system.report_glow_source_diagnostics(&visible_glow_diagnostics);

    let ambient = room_ambient_params(
        current_room.darkness,
        current_room.ambient_color,
        current_room.exposure,
    );

    let world_to_target_scale = target_pixels_per_world_unit(
        render_cam,
        render_system.rt_width,
        render_system.rt_height,
    );
    let pixel_grid_origin = render_cam.world_to_screen(
        Vec2::ZERO,
        render_system.rt_width,
        render_system.rt_height,
    );
    let routed_lights = prepare_routed_lights(
        render_cam,
        render_system.rt_width,
        render_system.rt_height,
        world_to_target_scale,
        pixel_grid_origin,
        &visible_layers.ordered_layers,
        &layer_maps,
    );
    let routed_glows = prepare_routed_glows(
        render_cam,
        render_system.rt_width,
        render_system.rt_height,
        world_to_target_scale,
        pixel_grid_origin,
        &visible_layers.ordered_layers,
        &layer_maps,
    );

    let plan = build_room_lighting_plan(
        &visible_layers.ordered_layers,
        &routed_lights,
        &routed_glows,
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
    world_to_target_scale: f32,
    pixel_grid_origin: Vec2,
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
                            world_to_target_scale,
                        ),
                        pixel_grid_origin: [pixel_grid_origin.x, pixel_grid_origin.y],
                    },
                });
            }
        }
    }
    lights
}

fn prepare_routed_glows(
    render_cam: &Camera2D,
    target_width: f32,
    target_height: f32,
    world_to_target_scale: f32,
    pixel_grid_origin: Vec2,
    visible_layers: &[RoomLayer],
    layer_maps: &CollectedRoomLayerMaps<'_>,
) -> Vec<RoutedGlow> {
    let mut glows = Vec::new();
    let world_to_target_scale = finite_nonnegative(world_to_target_scale, 0.0);

    for &owner_layer in visible_layers {
        for layer_data in layer_maps.for_layer(owner_layer).values() {
            for collected in &layer_data.lighting.glows {
                let glow = collected.glow;
                let has_visible_target = visible_layers.iter().any(|target_layer| {
                    light_scope_affects_layer(
                        glow.layer_scope,
                        owner_layer,
                        *target_layer,
                        visible_layers,
                    )
                });
                let intensity = finite_clamped(glow.intensity, 0.0, 20.0, 1.0);
                if !has_visible_target || intensity <= 0.0 {
                    continue;
                }

                let halo_spread = finite_clamped(glow.halo_spread, 0.0, 256.0, 16.0)
                    * world_to_target_scale;
                if !glow_source_visible_with_halo(
                    render_cam,
                    collected.source.destination,
                    halo_spread,
                    target_width,
                    target_height,
                ) {
                    continue;
                }

                glows.push(RoutedGlow {
                    entity: collected.entity,
                    owner_layer,
                    scope: glow.layer_scope,
                    params: GlowPassParams {
                        color: [
                            finite_clamped(glow.color.x, 0.0, 1.0, 1.0),
                            finite_clamped(glow.color.y, 0.0, 1.0, 1.0),
                            finite_clamped(glow.color.z, 0.0, 1.0, 1.0),
                        ],
                        intensity,
                        halo_spread,
                        pixelated: glow.style == PointLightStyle::Pixel,
                        pixel_size: point_light_pixel_size_in_target(
                            glow.pixel_size.clamp(1, 16),
                            world_to_target_scale,
                        ),
                        pixel_grid_origin: [pixel_grid_origin.x, pixel_grid_origin.y],
                    },
                    source: collected.source.clone(),
                });
            }
        }
    }

    glows.sort_by_key(|glow| glow.entity);
    glows
}

fn glow_source_visible_with_halo(
    camera: &Camera2D,
    source: Rect,
    halo_spread: f32,
    target_width: f32,
    target_height: f32,
) -> bool {
    if target_width <= 0.0 || target_height <= 0.0 || source.w <= 0.0 || source.h <= 0.0 {
        return false;
    }

    let corners = [
        Vec2::new(source.x, source.y),
        Vec2::new(source.x + source.w, source.y),
        Vec2::new(source.x, source.y + source.h),
        Vec2::new(source.x + source.w, source.y + source.h),
    ]
    .map(|corner| camera.world_to_screen(corner, target_width, target_height));
    if corners.iter().any(|corner| !corner.is_finite()) {
        return false;
    }

    let mut left = f32::INFINITY;
    let mut top = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    let mut bottom = f32::NEG_INFINITY;
    for corner in corners {
        left = left.min(corner.x);
        top = top.min(corner.y);
        right = right.max(corner.x);
        bottom = bottom.max(corner.y);
    }
    let left = left - halo_spread;
    let top = top - halo_spread;
    let right = right + halo_spread;
    let bottom = bottom + halo_spread;
    Rect::new(left, top, right - left, bottom - top)
        .overlaps(&Rect::new(0.0, 0.0, target_width, target_height))
}

fn finite_clamped(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

fn target_pixels_per_world_unit(camera: &Camera2D, target_width: f32, target_height: f32) -> f32 {
    let origin = camera.world_to_screen(Vec2::ZERO, target_width, target_height);
    let world_x = camera.world_to_screen(Vec2::X, target_width, target_height);
    let world_y = camera.world_to_screen(Vec2::Y, target_width, target_height);
    (origin.distance(world_x) * origin.distance(world_y)).sqrt()
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

    fn draw_emission_source<C: BishopContext>(
        &mut self,
        ctx: &mut C,
        source: &GlowVisual,
    ) -> bool {
        ctx.set_camera(self.render_cam);
        draw_emission_mask(
            ctx,
            self.sprite_manager,
            source.sprite_id,
            source.source_rect,
            source.destination,
            source.flip_x,
        )
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
        let mut illumination_source = match render_system.light_contribution(
            ctx,
            contribution,
            ambient,
            &point_lights,
        ) {
            Ok(buffer) => buffer,
            Err(error) => {
                RenderSystem::report_pass_failure(&error);
                lighting_succeeded = false;
                continue;
            }
        };

        for glow in &entry.glows {
            match render_system.draw_emission_source_mask(
                ctx,
                contribution,
                |ctx| room_draw.draw_emission_source(ctx, &glow.source),
            ) {
                Some(true) => {}
                Some(false) => {
                    render_system.report_emission_source_unavailable(glow.entity);
                    continue;
                }
                None => continue,
            }

            match render_system.apply_emission_source(
                ctx,
                contribution,
                illumination_source,
                glow.params.into(),
            ) {
                Ok(next_source) => illumination_source = next_source,
                Err(error) => render_system.report_emission_pass_failure(glow.entity, &error),
            }
        }

        match render_system.composite_contribution(ctx, contribution, illumination_source) {
            Ok(()) => {
                lit_buffers.insert(contribution, illumination_source);
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
