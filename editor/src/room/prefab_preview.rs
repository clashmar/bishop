use bishop::prelude::*;
use engine_core::constants::world as world_constants;
use engine_core::prelude::*;

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

    if items.is_empty() {
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
    }

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
    }
}

fn preview_item_from_node(
    node: &PrefabNode,
    resolve_sprite_size: &mut impl FnMut(SpriteId) -> Option<Vec2>,
    resolve_animation_sprite: &mut impl FnMut(&VariantFolder, &ClipId) -> Option<SpriteId>,
) -> Option<PrefabPreviewItem> {
    let transform = node
        .components
        .iter()
        .find(|component| component.type_name == comp_type_name::<Transform>())
        .and_then(|component| ron::from_str::<Transform>(&component.ron).ok())
        .unwrap_or_default();
    if !transform.visible {
        return None;
    }

    let z = node
        .components
        .iter()
        .find(|component| component.type_name == comp_type_name::<Layer>())
        .and_then(|component| ron::from_str::<Layer>(&component.ron).ok())
        .map_or(0, |layer| layer.z);

    if let Some(frame) = node
        .components
        .iter()
        .find(|component| component.type_name == comp_type_name::<CurrentFrame>())
        .and_then(|component| parse_preview_current_frame(&component.ron))
    {
        if frame.sprite_id.0 != 0 {
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

    if let Some(sprite) = node
        .components
        .iter()
        .find(|component| component.type_name == comp_type_name::<Sprite>())
        .and_then(|component| ron::from_str::<Sprite>(&component.ron).ok())
    {
        if sprite.sprite.0 != 0 {
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

    if let Some(animation) = node
        .components
        .iter()
        .find(|component| component.type_name == comp_type_name::<Animation>())
        .and_then(|component| ron::from_str::<Animation>(&component.ron).ok())
    {
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

struct PreviewCurrentFrame {
    sprite_id: SpriteId,
    col: usize,
    row: usize,
    offset: [f32; 2],
    frame_size: [f32; 2],
    flip_x: bool,
}

fn parse_preview_current_frame(ron: &str) -> Option<PreviewCurrentFrame> {
    Some(PreviewCurrentFrame {
        sprite_id: SpriteId(parse_usize_field(ron, "sprite_id")?),
        col: parse_usize_field(ron, "col")?,
        row: parse_usize_field(ron, "row")?,
        offset: parse_vec2_field(ron, "offset")?,
        frame_size: parse_vec2_field(ron, "frame_size")?,
        flip_x: parse_bool_field(ron, "flip_x")?,
    })
}

fn parse_usize_field(ron: &str, key: &str) -> Option<usize> {
    let value = field_value(ron, key)?;
    let trimmed = value.trim().trim_start_matches('(').trim_end_matches(')');
    trimmed.parse().ok()
}

fn parse_bool_field(ron: &str, key: &str) -> Option<bool> {
    match field_value(ron, key)?.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_vec2_field(ron: &str, key: &str) -> Option<[f32; 2]> {
    let value = field_value(ron, key)?.trim();
    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    let mut parts = inner.split(',').map(str::trim);
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    Some([x, y])
}

fn field_value<'a>(ron: &'a str, key: &str) -> Option<&'a str> {
    let start = ron.find(&format!("{key}:"))? + key.len() + 1;
    let rest = &ron[start..];
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    for (index, ch) in rest.char_indices() {
        match ch {
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            ',' if depth_paren == 0 && depth_bracket == 0 => return Some(&rest[..index]),
            _ => {}
        }
    }
    Some(rest.trim_end_matches(')'))
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
