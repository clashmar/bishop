use std::collections::{BTreeMap, HashMap, HashSet};

use bishop::prelude::{BishopContext, Color, Rect, Vec2};
use crate::assets::SpriteManager;
use crate::ecs::{
    Cover, CurrentFrame, CurrentRoom, Ecs, Entity, Glow, LayerDoor,
    PointLight, RoomCamera, Sprite, SubPixel, Transform,
};
use crate::ecs::components::transform::Pivot;
use crate::rendering::helpers::{
    entity_visible_in_room, entity_visual_rect, interpolate_position, pivot_adjusted_position,
    resolve_visual_entity, spillover_candidate_room_ids, visual_position,
    visual_rect_visible_in_room, RoomVisibilityContext,
};
use crate::rendering::lighting::{GlowVisual, RoomLayerLighting};
use crate::rendering::particle_draw::{draw_particle, particle_visual_bounds};
use crate::rendering::particles::{Particle, ParticleSystem};
use crate::rendering::renderable::{EntityDrawParams, Renderable};
use crate::rendering::room_composition::RoomCompositionContext;
use crate::worlds::RoomLayer;

/// Render data collected for a room layer and depth group.
#[derive(Default)]
pub(super) struct LayerData<'a> {
    pub(super) items: Vec<LayerDrawItem<'a>>,
    pub(super) lighting: RoomLayerLighting<'a>,
}

pub(super) enum LayerDrawItem<'a> {
    Entity { entity: Entity, position: Vec2 },
    Particle { particle: &'a Particle, bounds: Rect },
}

impl LayerDrawItem<'_> {
    fn sort_key(&self) -> (Entity, u8, u64) {
        match self {
            Self::Entity { entity, .. } => (*entity, 0, 0),
            Self::Particle { particle, .. } => (particle.owner, 1, particle.emission_sequence),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct GlowSourceDiagnostic {
    pub(super) entity: Entity,
    pub(super) owner_layer: RoomLayer,
}

/// Depth-grouped render data collected from the visible room neighborhood.
#[derive(Default)]
pub(super) struct CollectedRoomLayerMaps<'a> {
    front: BTreeMap<i32, LayerData<'a>>,
    back: BTreeMap<i32, LayerData<'a>>,
    pub(super) glow_source_diagnostics: Vec<GlowSourceDiagnostic>,
}

impl<'a> CollectedRoomLayerMaps<'a> {
    pub(super) fn for_layer(&self, room_layer: RoomLayer) -> &BTreeMap<i32, LayerData<'a>> {
        match room_layer {
            RoomLayer::Front => &self.front,
            RoomLayer::Back => &self.back,
        }
    }
}

pub(super) fn draw_layer_items<C: BishopContext, F>(
    ctx: &mut C,
    ecs: &Ecs,
    sprite_manager: &mut SpriteManager,
    layer_map: &BTreeMap<i32, LayerData<'_>>,
    grid_size: f32,
    composition: &RoomCompositionContext,
    include_item: F,
) where
    F: Fn(&Ecs, &LayerDrawItem<'_>) -> bool,
{
    for layer in layer_map.values() {
        for item in &layer.items {
            if !include_item(ecs, item) {
                continue;
            }
            match item {
                LayerDrawItem::Entity { entity, position } => draw_entity(
                    ctx,
                    ecs,
                    sprite_manager,
                    *entity,
                    *position,
                    grid_size,
                    composition,
                ),
                LayerDrawItem::Particle { particle, bounds } => {
                    draw_particle(ctx, ecs, sprite_manager, particle, *bounds, composition);
                }
            }
        }
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

fn resolve_glow_visual_source(
    ecs: &Ecs,
    sprite_manager: &SpriteManager,
    entity: Entity,
    position: Vec2,
    grid_size: f32,
) -> Option<GlowVisual> {
    let visual_entity = resolve_visual_entity(ecs, entity);
    let pivot = ecs
        .get_store::<Transform>()
        .get(entity)
        .map(|transform| transform.pivot)
        .unwrap_or(Pivot::BottomCenter);

    if let Some(frame) = ecs.get_store::<CurrentFrame>().get(visual_entity)
        && frame.has_valid_asset()
    {
        let frame_size = frame.frame_size;
        if !frame_size.is_finite() || frame_size.x <= 0.0 || frame_size.y <= 0.0 {
            return None;
        }

        let frame_position =
            pivot_adjusted_position(position, frame_size, pivot) + frame.offset;
        return Some(GlowVisual {
            sprite_id: frame.sprite_id,
            source_rect: Rect::new(
                frame.col as f32 * frame_size.x,
                frame.row as f32 * frame_size.y,
                frame_size.x,
                frame_size.y,
            ),
            destination: Rect::new(
                frame_position.x,
                frame_position.y,
                frame_size.x,
                frame_size.y,
            ),
            flip_x: frame.flip_x,
        });
    }

    let sprite = ecs.get_store::<Sprite>().get(visual_entity)?;
    if !sprite.has_valid_asset() {
        return None;
    }
    let (width, height) = sprite_manager
        .texture_size(sprite.sprite)
        .unwrap_or((grid_size, grid_size));
    let size = Vec2::new(width, height);
    if !size.is_finite() || size.x <= 0.0 || size.y <= 0.0 {
        return None;
    }
    let draw_position = pivot_adjusted_position(position, size, pivot);

    Some(GlowVisual {
        sprite_id: sprite.sprite,
        source_rect: Rect::new(0.0, 0.0, size.x, size.y),
        destination: Rect::new(draw_position.x, draw_position.y, size.x, size.y),
        flip_x: false,
    })
}

pub(super) fn collect_interpolated_room_layer_maps<'a>(
    ecs: &'a Ecs,
    room_ctx: &RoomVisibilityContext<'_>,
    sprite_manager: &SpriteManager,
    alpha: f32,
    prev_positions: Option<&HashMap<Entity, Vec2>>,
    particles: Option<&'a ParticleSystem>,
) -> CollectedRoomLayerMaps<'a> {
    let world = room_ctx.world;
    let room = room_ctx.room;
    let mut maps = CollectedRoomLayerMaps::default();
    let mut seen = HashSet::new();

    let trans_store = ecs.get_store::<Transform>();
    let cam_store = ecs.get_store::<RoomCamera>();
    let point_light_store = ecs.get_store::<PointLight>();
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
            if !transform.visible || cam_store.get(entity).is_some() {
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
                room_ctx,
            ) {
                continue;
            }

            let room_layer = ecs
                .get::<CurrentRoom>(entity)
                .map_or(RoomLayer::Front, |current_room| current_room.layer);
            let layer_map = match room_layer {
                RoomLayer::Front => &mut maps.front,
                RoomLayer::Back => &mut maps.back,
            };
            let entry = layer_map.entry(transform.z).or_default();
            entry.items.push(LayerDrawItem::Entity {
                entity,
                position: draw_pos,
            });

            if let Some(light) = point_light_store.get(entity) {
                entry.lighting.collect_point_light(entity, light, draw_pos);
            }
            let visual_entity = resolve_visual_entity(ecs, entity);
            let glow_entry = glow_store
                .get(entity)
                .map(|glow| (entity, glow))
                .or_else(|| glow_store.get(visual_entity).map(|glow| (visual_entity, glow)));
            if let Some((glow_entity, glow)) = glow_entry {
                if let Some(source) = resolve_glow_visual_source(
                    ecs,
                    sprite_manager,
                    entity,
                    draw_pos,
                    room_ctx.grid_size,
                ) {
                    entry.lighting.collect_glow(glow_entity, glow, source);
                } else {
                    maps.glow_source_diagnostics
                        .push(GlowSourceDiagnostic {
                            entity: glow_entity,
                            owner_layer: room_layer,
                        });
                }
            }
        }
    }

    if let Some(particles) = particles {
        for particle in particles.particles() {
            let Some(current_room) = ecs.get::<CurrentRoom>(particle.owner) else {
                continue;
            };
            let Some(transform) = ecs.get::<Transform>(particle.owner) else {
                continue;
            };
            let bounds = particle_visual_bounds(particle);
            if !visual_rect_visible_in_room(current_room.room_id, bounds, room_ctx) {
                continue;
            }

            let layer_map = match current_room.layer {
                RoomLayer::Front => &mut maps.front,
                RoomLayer::Back => &mut maps.back,
            };
            layer_map
                .entry(transform.z)
                .or_default()
                .items
                .push(LayerDrawItem::Particle { particle, bounds });
        }
    }

    for layer in maps.front.values_mut().chain(maps.back.values_mut()) {
        layer.items.sort_by_key(LayerDrawItem::sort_key);
        layer.lighting.sort();
    }
    maps.glow_source_diagnostics.sort_by_key(|issue| issue.entity);
    maps.glow_source_diagnostics.dedup();

    maps
}
