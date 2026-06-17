use bishop::prelude::*;
use engine_core::animation::{ClipDef, ClipId, VariantFolder, resolve_sprite_id};
use engine_core::assets::*;
use engine_core::constants::world as world_constants;
use engine_core::ecs::component::comp_type_name;
use engine_core::ecs::components::{
    Animation, CurrentFrame, Glow, Light, RoomCamera, Sprite, WorldEntry, WorldExit,
};
use engine_core::ecs::*;
use engine_core::rendering::pivot_adjusted_position;
use serde::de::DeserializeOwned;
use crate::shared::entity_icon::EntityVisual;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PrefabPreviewVisual {
    Sprite {
        sprite_id: SpriteId,
    },
    CurrentFrame {
        sprite_id: SpriteId,
        source: Rect,
        flip_x: bool,
    },
    Placeholder,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PrefabPreviewItem {
    pub(crate) z: i32,
    pub(crate) palette_position: Vec2,
    pub(crate) stamp_position: Vec2,
    pub(crate) size: Vec2,
    pub(crate) visual: PrefabPreviewVisual,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PrefabPreview {
    pub(crate) items: Vec<PrefabPreviewItem>,
    pub(crate) palette_bounds: Rect,
    pub(crate) stamp_bounds: Rect,
    pub(crate) has_drawable_visual: bool,
    pub(crate) fallback_visuals: Vec<(Vec2, Vec2, EntityVisual)>,
}

pub(crate) fn build_prefab_preview(
    loader: &impl TextureLoader,
    prefab: &PrefabAsset,
    asset_registry: &mut AssetRegistry,
    sprite_manager: &mut SpriteManager,
) -> PrefabPreview {
    let sprite_manager = std::cell::RefCell::new(sprite_manager);
    build_prefab_preview_with(
        prefab,
        |sprite_id| {
            let mut sprite_manager = sprite_manager.borrow_mut();
            preview_sprite_size(loader, &mut sprite_manager, sprite_id)
        },
        |variant, clip_id| {
            let mut sprite_manager = sprite_manager.borrow_mut();
            let sprite_id = resolve_sprite_id(
                loader,
                asset_registry,
                &mut sprite_manager,
                variant,
                clip_id,
            );
            (sprite_id.0 != 0).then_some(sprite_id)
        },
    )
}

pub(crate) fn build_prefab_preview_with(
    prefab: &PrefabAsset,
    mut resolve_sprite_size: impl FnMut(SpriteId) -> Option<Vec2>,
    mut resolve_animation_sprite: impl FnMut(&VariantFolder, &ClipId) -> Option<SpriteId>,
) -> PrefabPreview {
    let mut has_drawable_visual = false;
    let mut items = prefab
        .nodes
        .iter()
        .filter_map(|node| {
            let item = preview_item_from_node(
                node,
                &mut resolve_sprite_size,
                &mut resolve_animation_sprite,
            )?;
            has_drawable_visual |= !matches!(item.visual, PrefabPreviewVisual::Placeholder);
            Some(item)
        })
        .collect::<Vec<_>>();

    let fallback_visuals = if items.is_empty() {
        items.push(PrefabPreviewItem {
            z: 0,
            palette_position: Vec2::ZERO,
            stamp_position: pivot_adjusted_position(
                Vec2::ZERO,
                Vec2::splat(world_constants::DEFAULT_GRID_SIZE),
                Pivot::default(),
            ),
            size: Vec2::splat(world_constants::DEFAULT_GRID_SIZE),
            visual: PrefabPreviewVisual::Placeholder,
        });
        vec![(
            Vec2::ZERO,
            pivot_adjusted_position(
                Vec2::ZERO,
                Vec2::splat(world_constants::DEFAULT_GRID_SIZE),
                Pivot::default(),
            ),
            EntityVisual::GenericPlaceholder,
        )]
    } else {
        collect_fallback_visuals(prefab)
    };

    items.sort_by_key(|item| item.z);

    let palette_bounds = items
        .iter()
        .map(|item| {
            Rect::new(
                item.palette_position.x,
                item.palette_position.y,
                item.size.x,
                item.size.y,
            )
        })
        .reduce(union_rect)
        .unwrap_or_default();
    let stamp_bounds = items
        .iter()
        .map(|item| {
            Rect::new(
                item.stamp_position.x,
                item.stamp_position.y,
                item.size.x,
                item.size.y,
            )
        })
        .reduce(union_rect)
        .unwrap_or_default();

    PrefabPreview {
        items,
        palette_bounds,
        stamp_bounds,
        has_drawable_visual,
        fallback_visuals,
    }
}

fn collect_fallback_visuals(prefab: &PrefabAsset) -> Vec<(Vec2, Vec2, EntityVisual)> {
    prefab
        .nodes
        .iter()
        .filter_map(|node| {
            let transform = parse_node_component::<Transform>(node).unwrap_or_default();
            if !transform.visible {
                return None;
            }
            if node_has_valid_visual(node) {
                return None;
            }
            let visual = placeholder_entity_visual(node);
            if matches!(visual, EntityVisual::GenericPlaceholder) {
                return None;
            }
            let size = Vec2::splat(world_constants::DEFAULT_GRID_SIZE);
            Some((
                transform.position,
                pivot_adjusted_position(transform.position, size, transform.pivot),
                visual,
            ))
        })
        .collect()
}

fn node_has_valid_visual(node: &PrefabNode) -> bool {
    parse_node_current_frame(node).is_some_and(|frame| frame.has_valid_asset())
        || parse_node_component::<Sprite>(node).is_some_and(|sprite| sprite.has_valid_asset())
        || parse_node_component::<Animation>(node)
            .is_some_and(|animation| preferred_animation_preview_clip(&animation).is_some())
}

fn placeholder_entity_visual(node: &PrefabNode) -> EntityVisual {
    let type_names: Vec<&str> = node.components.iter().map(|c| c.type_name.as_str()).collect();

    if type_names.contains(&comp_type_name::<RoomCamera>()) {
        return EntityVisual::CameraIcon;
    }

    let has_entry = type_names.contains(&comp_type_name::<WorldEntry>());
    let has_exit = type_names.contains(&comp_type_name::<WorldExit>());

    match (has_entry, has_exit) {
        (true, true) => return EntityVisual::PortalIcon,
        (true, false) => return EntityVisual::EntryIcon,
        (false, true) => return EntityVisual::ExitIcon,
        (false, false) => {}
    }

    if type_names.contains(&comp_type_name::<Light>()) {
        return EntityVisual::LightPlaceholder;
    }

    if type_names.contains(&comp_type_name::<Glow>()) {
        return EntityVisual::GlowPlaceholder;
    }

    EntityVisual::GenericPlaceholder
}

fn preview_item_from_node(
    node: &PrefabNode,
    resolve_sprite_size: &mut impl FnMut(SpriteId) -> Option<Vec2>,
    resolve_animation_sprite: &mut impl FnMut(&VariantFolder, &ClipId) -> Option<SpriteId>,
) -> Option<PrefabPreviewItem> {
    let transform = parse_node_component::<Transform>(node).unwrap_or_default();
    if !transform.visible {
        return None;
    }

    let z = parse_node_component::<Layer>(node).map_or(0, |layer| layer.z);

    if let Some(frame) = parse_node_current_frame(node) {
        if frame.has_valid_asset() {
            let frame_size = vec2(frame.frame_size[0], frame.frame_size[1]);
            let offset = vec2(frame.offset[0], frame.offset[1]);
            let source = Rect::new(
                frame.col as f32 * frame_size.x,
                frame.row as f32 * frame_size.y,
                frame_size.x,
                frame_size.y,
            );
            return Some(PrefabPreviewItem {
                z,
                palette_position: transform.position + offset,
                stamp_position: pivot_adjusted_position(
                    transform.position,
                    frame_size,
                    transform.pivot,
                ) + offset,
                size: frame_size,
                visual: PrefabPreviewVisual::CurrentFrame {
                    sprite_id: frame.sprite_id,
                    source,
                    flip_x: frame.flip_x,
                },
            });
        }
    }

    if let Some(sprite) = parse_node_component::<Sprite>(node) {
        if sprite.has_valid_asset() {
            if let Some(size) = resolve_sprite_size(sprite.sprite) {
                return Some(PrefabPreviewItem {
                    z,
                    palette_position: transform.position,
                    stamp_position: pivot_adjusted_position(
                        transform.position,
                        size,
                        transform.pivot,
                    ),
                    size,
                    visual: PrefabPreviewVisual::Sprite {
                        sprite_id: sprite.sprite,
                    },
                });
            }
        }
    }

    if let Some(animation) = parse_node_component::<Animation>(node) {
        if let Some((clip_id, clip)) = preferred_animation_preview_clip(&animation) {
            if let Some(sprite_id) = resolve_animation_sprite(&animation.variant, clip_id) {
                let frame_size = clip.frame_size;
                let offset = clip.offset;
                let source = Rect::new(0.0, 0.0, frame_size.x, frame_size.y);

                return Some(PrefabPreviewItem {
                    z,
                    palette_position: transform.position + offset,
                    stamp_position: pivot_adjusted_position(
                        transform.position,
                        frame_size,
                        transform.pivot,
                    ) + offset,
                    size: frame_size,
                    visual: PrefabPreviewVisual::CurrentFrame {
                        sprite_id,
                        source,
                        flip_x: false,
                    },
                });
            }
        }
    }

    let size = Vec2::splat(world_constants::DEFAULT_GRID_SIZE);
    Some(PrefabPreviewItem {
        z,
        palette_position: transform.position,
        stamp_position: pivot_adjusted_position(transform.position, size, transform.pivot),
        size,
        visual: PrefabPreviewVisual::Placeholder,
    })
}

fn preferred_animation_preview_clip(animation: &Animation) -> Option<(&ClipId, &ClipDef)> {
    animation.clips.get_key_value(&ClipId::Idle).or_else(|| {
        animation
            .clips
            .iter()
            .filter(|(clip_id, _)| **clip_id != ClipId::New)
            .min_by(|(left, _), (right, _)| left.cmp(right))
    })
}

fn union_rect(a: Rect, b: Rect) -> Rect {
    let min_x = a.x.min(b.x);
    let min_y = a.y.min(b.y);
    let max_x = a.right().max(b.right());
    let max_y = a.bottom().max(b.bottom());
    Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

fn parse_node_component<T: DeserializeOwned>(node: &PrefabNode) -> Option<T> {
    parse_node_component_with_type_name(node, comp_type_name::<T>())
}

fn parse_node_component_with_type_name<T: DeserializeOwned>(
    node: &PrefabNode,
    type_name: &str,
) -> Option<T> {
    node
        .components
        .iter()
        .find(|component| component.type_name == type_name)
        .and_then(|component| ron::from_str::<T>(&component.ron).ok())
}

fn parse_node_current_frame(node: &PrefabNode) -> Option<CurrentFrameSnapshot> {
    parse_node_component_with_type_name(node, comp_type_name::<CurrentFrame>())
}

fn preview_sprite_size(
    loader: &impl TextureLoader,
    sprite_manager: &mut SpriteManager,
    sprite_id: SpriteId,
) -> Option<Vec2> {
    if sprite_manager.texture_size(sprite_id).is_none() {
        let _ = sprite_manager.ensure_loaded(loader, sprite_id);
    }

    sprite_manager
        .texture_size(sprite_id)
        .map(|(width, height)| vec2(width, height))
}

#[cfg(test)]
#[path = "tests/prefab_preview_tests.rs"]
mod tests;
