use bishop::prelude::*;
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
    sprite_manager: &mut SpriteManager,
) -> PrefabPreview {
    build_prefab_preview_with(prefab, |sprite_id| {
        preview_sprite_size(loader, sprite_manager, sprite_id)
    })
}

pub(crate) fn build_prefab_preview_with(
    prefab: &PrefabAsset,
    mut resolve_sprite_size: impl FnMut(SpriteId) -> Option<Vec2>,
) -> PrefabPreview {
    let mut has_drawable_visual = false;
    let mut items = prefab
        .nodes
        .iter()
        .filter_map(|node| {
            let item = preview_item_from_node(node, &mut resolve_sprite_size)?;
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
                Vec2::splat(DEFAULT_GRID_SIZE),
                Pivot::default(),
            ),
            size: Vec2::splat(DEFAULT_GRID_SIZE),
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

    let size = Vec2::splat(DEFAULT_GRID_SIZE);
    Some(PrefabPreviewItem {
        z,
        palette_position: transform.position,
        stamp_position: pivot_adjusted_position(transform.position, size, transform.pivot),
        size,
        visual: PrefabPreviewVisual::Placeholder,
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
mod tests {
    use super::*;
    use engine_core::ecs::transform::Pivot;

    fn sprite_ron(sprite_id: usize) -> String {
        format!("Sprite(sprite: SpriteId({sprite_id}))")
    }

    fn transform_ron(position: (f32, f32), pivot: Pivot) -> String {
        format!(
            "Transform(visible: true, position: ({:.1}, {:.1}), pivot: {:?})",
            position.0, position.1, pivot
        )
    }

    fn node(node_id: usize, transform_ron: String, sprite_ron: String) -> PrefabNode {
        PrefabNode {
            node_id,
            parent_node_id: None,
            components: vec![
                ComponentSnapshot {
                    type_name: comp_type_name::<Transform>().to_string(),
                    ron: transform_ron,
                },
                ComponentSnapshot {
                    type_name: comp_type_name::<Sprite>().to_string(),
                    ron: sprite_ron,
                },
            ],
        }
    }

    fn make_prefab(nodes: Vec<PrefabNode>) -> PrefabAsset {
        PrefabAsset {
            id: PrefabId(1),
            name: "Test".to_string(),
            next_node_id: nodes.len() + 1,
            root_node_id: 1,
            nodes,
        }
    }

    const SPRITE_SIZE: Vec2 = Vec2::new(64.0, 48.0);

    fn sprite_size(_: SpriteId) -> Option<Vec2> {
        Some(SPRITE_SIZE)
    }

    #[test]
    fn stamp_position_applies_pivot_offset() {
        let prefab = make_prefab(vec![node(
            1,
            transform_ron((0.0, 0.0), Pivot::BottomCenter),
            sprite_ron(1),
        )]);
        let preview = build_prefab_preview_with(&prefab, sprite_size);

        let item = preview.items.first().unwrap();
        let expected_stamp = pivot_adjusted_position(Vec2::ZERO, SPRITE_SIZE, Pivot::BottomCenter);
        assert_eq!(item.stamp_position, expected_stamp);
        assert_ne!(item.stamp_position, item.palette_position);
    }

    #[test]
    fn stamp_bounds_union_covers_all_pivot_adjusted_rects() {
        let prefab = make_prefab(vec![
            node(1, transform_ron((0.0, 0.0), Pivot::TopLeft), sprite_ron(1)),
            node(
                2,
                transform_ron((64.0, 0.0), Pivot::BottomCenter),
                sprite_ron(2),
            ),
        ]);
        let preview = build_prefab_preview_with(&prefab, sprite_size);

        let r = preview.stamp_bounds;
        assert!(
            r.x <= 0.0,
            "stamp bounds should extend to leftmost visual edge"
        );
        assert!(
            r.y <= 0.0,
            "stamp bounds should extend to topmost visual edge"
        );

        let rightmost_item_bottom = {
            let bottom_center =
                pivot_adjusted_position(Vec2::new(64.0, 0.0), SPRITE_SIZE, Pivot::BottomCenter);
            bottom_center.y + SPRITE_SIZE.y
        };
        let first_item_bottom =
            pivot_adjusted_position(Vec2::ZERO, SPRITE_SIZE, Pivot::TopLeft).y + SPRITE_SIZE.y;
        assert!(
            r.y + r.h >= first_item_bottom.max(rightmost_item_bottom),
            "stamp bounds should span both items"
        );
    }

    #[test]
    fn items_sorted_by_z_ascending() {
        let node_a = PrefabNode {
            node_id: 1,
            parent_node_id: None,
            components: vec![
                ComponentSnapshot {
                    type_name: comp_type_name::<Transform>().to_string(),
                    ron: transform_ron((0.0, 0.0), Pivot::TopLeft),
                },
                ComponentSnapshot {
                    type_name: comp_type_name::<Sprite>().to_string(),
                    ron: sprite_ron(1),
                },
                ComponentSnapshot {
                    type_name: comp_type_name::<Layer>().to_string(),
                    ron: "Layer(z: 5)".to_string(),
                },
            ],
        };
        let node_b = PrefabNode {
            node_id: 2,
            parent_node_id: None,
            components: vec![
                ComponentSnapshot {
                    type_name: comp_type_name::<Transform>().to_string(),
                    ron: transform_ron((0.0, 0.0), Pivot::TopLeft),
                },
                ComponentSnapshot {
                    type_name: comp_type_name::<Sprite>().to_string(),
                    ron: sprite_ron(2),
                },
                ComponentSnapshot {
                    type_name: comp_type_name::<Layer>().to_string(),
                    ron: "Layer(z: -2)".to_string(),
                },
            ],
        };
        let prefab = make_prefab(vec![node_a, node_b]);
        let preview = build_prefab_preview_with(&prefab, sprite_size);

        assert_eq!(preview.items.len(), 2);
        assert_eq!(preview.items[0].z, -2);
        assert_eq!(preview.items[1].z, 5);
    }

    #[test]
    fn palette_bounds_differ_from_stamp_bounds_when_pivot_requires_it() {
        let prefab = make_prefab(vec![node(
            1,
            transform_ron((0.0, 0.0), Pivot::BottomCenter),
            sprite_ron(1),
        )]);
        let preview = build_prefab_preview_with(&prefab, sprite_size);

        assert_ne!(preview.palette_bounds, preview.stamp_bounds);
        assert_eq!(
            preview.palette_bounds.x, 0.0,
            "palette_bounds uses raw transform position x"
        );
        assert!(
            preview.stamp_bounds.y < 0.0,
            "stamp_bounds.y should be negative for BottomCenter pivot"
        );
    }
}
