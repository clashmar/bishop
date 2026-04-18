use super::*;
use engine_core::ecs::Pivot;
use std::path::Path;

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

fn animation_component(clips: Vec<(ClipId, ClipDef)>, variant: &str) -> ComponentSnapshot {
    let animation = Animation {
        clips: clips.into_iter().collect(),
        variant: VariantFolder(Path::new(variant).to_path_buf()),
        ..Default::default()
    };

    ComponentSnapshot {
        type_name: comp_type_name::<Animation>().to_string(),
        ron: ron::to_string(&animation).unwrap(),
    }
}

fn animation_node(
    node_id: usize,
    position: (f32, f32),
    pivot: Pivot,
    animation: ComponentSnapshot,
) -> PrefabNode {
    PrefabNode {
        node_id,
        parent_node_id: None,
        components: vec![
            ComponentSnapshot {
                type_name: comp_type_name::<Transform>().to_string(),
                ron: transform_ron(position, pivot),
            },
            animation,
        ],
    }
}

fn animation_sprite_id(variant: &VariantFolder, clip_id: &ClipId) -> Option<SpriteId> {
    if variant.0 != Path::new("animations/player/male") {
        return None;
    }

    match clip_id {
        ClipId::Idle => Some(SpriteId(11)),
        ClipId::Run => Some(SpriteId(12)),
        _ => None,
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
    let preview = build_prefab_preview_with(&prefab, sprite_size, |_, _| None);

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
    let preview = build_prefab_preview_with(&prefab, sprite_size, |_, _| None);

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
    let preview = build_prefab_preview_with(&prefab, sprite_size, |_, _| None);

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
    let preview = build_prefab_preview_with(&prefab, sprite_size, |_, _| None);

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

#[test]
fn animation_preview_prefers_idle_clip_when_present() {
    let prefab = make_prefab(vec![animation_node(
        1,
        (0.0, 0.0),
        Pivot::TopLeft,
        animation_component(
            vec![
                (
                    ClipId::Run,
                    ClipDef {
                        frame_size: Vec2::new(32.0, 24.0),
                        ..Default::default()
                    },
                ),
                (
                    ClipId::Idle,
                    ClipDef {
                        frame_size: Vec2::new(16.0, 12.0),
                        ..Default::default()
                    },
                ),
            ],
            "animations/player/male",
        ),
    )]);

    let preview = build_prefab_preview_with(&prefab, sprite_size, animation_sprite_id);

    assert_eq!(
        preview.items[0].visual,
        PrefabPreviewVisual::CurrentFrame {
            sprite_id: SpriteId(11),
            source: Rect::new(0.0, 0.0, 16.0, 12.0),
            flip_x: false,
        }
    );
}

#[test]
fn animation_preview_uses_only_available_clip_when_idle_missing() {
    let prefab = make_prefab(vec![animation_node(
        1,
        (0.0, 0.0),
        Pivot::TopLeft,
        animation_component(
            vec![(
                ClipId::Run,
                ClipDef {
                    frame_size: Vec2::new(32.0, 24.0),
                    ..Default::default()
                },
            )],
            "animations/player/male",
        ),
    )]);

    let preview = build_prefab_preview_with(&prefab, sprite_size, animation_sprite_id);

    assert_eq!(
        preview.items[0].visual,
        PrefabPreviewVisual::CurrentFrame {
            sprite_id: SpriteId(12),
            source: Rect::new(0.0, 0.0, 32.0, 24.0),
            flip_x: false,
        }
    );
}

#[test]
fn animation_preview_falls_back_when_animation_has_no_clips() {
    let prefab = make_prefab(vec![animation_node(
        1,
        (0.0, 0.0),
        Pivot::TopLeft,
        animation_component(vec![], "animations/player/male"),
    )]);

    let preview = build_prefab_preview_with(&prefab, sprite_size, animation_sprite_id);

    assert_eq!(preview.items[0].visual, PrefabPreviewVisual::Placeholder);
    assert!(!preview.has_drawable_visual);
}
