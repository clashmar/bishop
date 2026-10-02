use bishop::prelude::*;
use engine_core::ecs::{Collider, ColliderShape, Entity, DEFAULT_COLLIDER_DIMENSION, Pivot};
use widgets::constants::layout as layout_constants;

use super::{
    body_layout,
    collider_edit_target,
    default_collider_for_reset,
    reset_collider_to_default,
};
use super::edit::{compute_handles, HandleAction};
use crate::world::coord::round_to_grid;

#[test]
fn collider_edit_target_uses_frame_when_animation_frame_edit_is_active() {
    use crate::gui::inspector::animation_module::frame_edit;
    use engine_core::animation::{ClipDef, ClipId};
    use engine_core::ecs::{Animation, ColliderEditTarget, ColliderFrameKey};
    use std::collections::HashMap;

    let entity = Entity(42);
    let animation = Animation {
        current: Some(ClipId::Run),
        clips: HashMap::from([(
            ClipId::Run,
            ClipDef {
                cols: 2,
                rows: 1,
                ..Default::default()
            },
        )]),
        ..Default::default()
    };
    frame_edit::enter(entity, &animation);
    frame_edit::step_selected_frame(entity, &animation, 1);

    assert_eq!(
        collider_edit_target(entity),
        ColliderEditTarget::Frame {
            clip_id: ClipId::Run,
            frame: ColliderFrameKey { row: 0, col: 1 },
        },
    );

    frame_edit::exit(entity);
}

#[test]
fn collider_edit_target_defaults_to_static_without_frame_edit() {
    use engine_core::ecs::ColliderEditTarget;

    assert_eq!(collider_edit_target(Entity(99)), ColliderEditTarget::Static);
}

#[test]
fn layout_body_height_is_positive() {
    let body = body_layout();
    assert!(body.height() > 0.0, "body layout height should be positive");
}

#[test]
fn layout_body_height_includes_aseprite_slice_import_row() {
    let body = body_layout();
    let expected = layout_constants::WIDGET_SPACING
        + super::ROW_H * 4.0
        + layout_constants::WIDGET_SPACING * 3.0
        + layout_constants::WIDGET_SPACING;
    let actual = body.height();
    assert!(
        (actual - expected).abs() < 1.0,
        "expected height ~{}, got {}",
        expected,
        actual,
    );
}

#[test]
fn point_handles_include_move_offset_handle() {
    let collider = Collider {
        shape: ColliderShape::Point,
        offset: vec2(3.0, -2.0),
        ..Default::default()
};

    let handles = compute_handles(vec2(10.0, 20.0), Pivot::BottomCenter, &collider.static_data(), 16.0);

    assert_eq!(handles.len(), 1);
    assert_eq!(handles[0].action, HandleAction::MoveOffset);
    assert_eq!(handles[0].rect.x + handles[0].rect.w * 0.5, 13.0);
    assert_eq!(handles[0].rect.y + handles[0].rect.h * 0.5, 18.0);
}

#[test]
fn capsule_side_handles_are_centered_on_capsule_midline() {
    let collider = Collider {
        shape: ColliderShape::Capsule {
            radius: 4.0,
            height: 10.0,
        },
        ..Default::default()
    };

    let handles = compute_handles(Vec2::ZERO, Pivot::TopLeft, &collider.static_data(), 16.0);
    let left = &handles[0];
    let right = &handles[1];
    let move_handle = &handles[4];

    assert_eq!(left.action, HandleAction::ResizeCapsuleRadiusLeft);
    assert_eq!(right.action, HandleAction::ResizeCapsuleRadiusRight);
    assert_eq!(move_handle.action, HandleAction::MoveOffset);
    assert_eq!(left.rect.y + left.rect.h * 0.5, 9.0);
    assert_eq!(right.rect.y + right.rect.h * 0.5, 9.0);
    assert_eq!(move_handle.rect.y + move_handle.rect.h * 0.5, 9.0);
}

#[test]
fn round_to_grid_rounds_to_nearest_multiple() {
    assert_eq!(round_to_grid(7.0, 16.0), 0.0);
    assert_eq!(round_to_grid(9.0, 16.0), 16.0);
    assert_eq!(round_to_grid(23.0, 16.0), 16.0);
    assert_eq!(round_to_grid(25.0, 16.0), 32.0);
    assert_eq!(round_to_grid(0.0, 16.0), 0.0);
}

#[test]
fn round_to_grid_small_values_clamp_to_grid() {
    assert_eq!(round_to_grid(0.5, 16.0), 0.0);
    assert_eq!(round_to_grid(15.5, 16.0), 16.0);
}

#[test]
fn aabb_handles_include_edge_midpoints() {
    let collider = Collider {
        shape: ColliderShape::Aabb { width: 32.0, height: 32.0 },
        ..Default::default()
    };
    let grid_size = 16.0;

    let handles = compute_handles(Vec2::ZERO, Pivot::TopLeft, &collider.static_data(), grid_size);

    // 4 corners + 4 edges + 1 center = 9
    assert_eq!(handles.len(), 9);

    let actions: Vec<_> = handles.iter().map(|h| h.action).collect();
    assert!(actions.contains(&HandleAction::ResizeTop));
    assert!(actions.contains(&HandleAction::ResizeBottom));
    assert!(actions.contains(&HandleAction::ResizeLeft));
    assert!(actions.contains(&HandleAction::ResizeRight));
}

#[test]
fn reset_collider_preserves_aabb_shape_variant() {
    let default = Collider {
        shape: ColliderShape::Aabb {
            width: 32.0,
            height: 48.0,
        },
        offset: Vec2::ZERO,
        ..Default::default()
};
    let mut collider = Collider {
        shape: ColliderShape::Aabb {
            width: 100.0,
            height: 200.0,
        },
        offset: vec2(5.0, -3.0),
        ..Default::default()
};

    reset_collider_to_default(&mut collider, &default);

    let ColliderShape::Aabb { width, height } = collider.shape else {
        panic!("expected Aabb, got {:?}", collider.shape);
    };
    assert_eq!(width, 32.0);
    assert_eq!(height, 48.0);
    assert_eq!(collider.offset, Vec2::ZERO);
}

#[test]
fn reset_collider_preserves_circle_shape_variant() {
    let default_width = 20.0_f32;
    let default_height = 40.0_f32;
    let default = Collider {
        shape: ColliderShape::Aabb {
            width: default_width,
            height: default_height,
        },
        offset: Vec2::ZERO,
        ..Default::default()
};
    let mut collider = Collider {
        shape: ColliderShape::Circle { radius: 50.0 },
        offset: vec2(1.0, 2.0),
        ..Default::default()
};

    reset_collider_to_default(&mut collider, &default);

    let expected_radius = default_width.min(default_height) / 2.0;
    let ColliderShape::Circle { radius } = collider.shape else {
        panic!("expected Circle, got {:?}", collider.shape);
    };
    assert_eq!(radius, expected_radius);
    assert_eq!(collider.offset, Vec2::ZERO);
}

#[test]
fn collider_aseprite_slice_import_visible_only_for_animation_collider_entities() {
    use super::sync::should_show_aseprite_slice_import;
    use engine_core::animation::ClipDef;
    use engine_core::ecs::{Animation, Ecs};
    use std::collections::HashMap;

    let mut ecs = Ecs::default();
    let collider_only = ecs.create_entity().with(Collider::default()).finish();
    let animation_only = ecs
        .create_entity()
        .with(Animation {
            clips: HashMap::from([(engine_core::animation::ClipId::Run, ClipDef::default())]),
            ..Default::default()
        })
        .finish();
    let animated_collider = ecs
        .create_entity()
        .with(Collider::default())
        .with(Animation {
            clips: HashMap::from([(engine_core::animation::ClipId::Run, ClipDef::default())]),
            ..Default::default()
        })
        .finish();

    assert!(!should_show_aseprite_slice_import(&ecs, collider_only));
    assert!(!should_show_aseprite_slice_import(&ecs, animation_only));
    assert!(should_show_aseprite_slice_import(&ecs, animated_collider));
}

#[test]
fn collider_aseprite_slice_import_label_is_short_and_specific() {
    use super::sync::aseprite_slice_import_label;

    assert_eq!(aseprite_slice_import_label(), "Import Aseprite Slices");
}

#[test]
fn collider_aseprite_import_toast_reports_success_warning_and_failure() {
    use super::sync::{aseprite_slice_import_toast, AsepriteSliceImportToast};

    assert_eq!(
        aseprite_slice_import_toast(2, 0, 0),
        (
            AsepriteSliceImportToast::Success,
            "Imported 2 collider slice(s)".to_string(),
            2.0,
        ),
    );
    assert_eq!(
        aseprite_slice_import_toast(2, 1, 0),
        (
            AsepriteSliceImportToast::Warning,
            "Imported 2 collider slice(s), 1 warning(s), 0 failure(s)".to_string(),
            3.0,
        ),
    );
    assert_eq!(
        aseprite_slice_import_toast(0, 0, 0),
        (
            AsepriteSliceImportToast::Warning,
            "No Aseprite collider slices found".to_string(),
            3.0,
        ),
    );
    assert_eq!(
        aseprite_slice_import_toast(0, 0, 1),
        (
            AsepriteSliceImportToast::Failure,
            "Aseprite slice import failed for 1 clip(s)".to_string(),
            3.0,
        ),
    );
}

#[test]
fn collider_aseprite_import_applies_sparse_frames_only() {
    use super::sync::apply_imported_frame_data;
    use bishop::prelude::Vec2;
    use engine_core::animation::ClipId;
    use engine_core::ecs::{ColliderData, ColliderFrameKey};
    use std::collections::BTreeMap;

    let collider = Collider::default();
    let frame = ColliderFrameKey { row: 0, col: 1 };
    let imported_data = ColliderData {
        shape: ColliderShape::Aabb {
            width: 14.0,
            height: 15.0,
        },
        offset: Vec2::new(2.0, 3.0),
    };
    let imported = BTreeMap::from([(frame, imported_data)]);

    let synced = apply_imported_frame_data(&collider, ClipId::Run, &imported);

    assert_eq!(synced.effective_data_for_frame(&ClipId::Run, frame), imported_data);
    assert_eq!(
        synced.effective_data_for_frame(&ClipId::Run, ColliderFrameKey { row: 0, col: 0 }),
        collider.static_data(),
    );
}

#[test]
fn reset_default_collider_without_visual_components_uses_collider_default() {
    use engine_core::assets::sprite_manager::SpriteManager;
    use engine_core::ecs::Ecs;

    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().with(Collider::default()).finish();
    let mut sprite_manager = SpriteManager::default();

    let default = default_collider_for_reset(&ecs, &mut sprite_manager, entity);

    assert_eq!(default, Collider::default());
}

#[test]
fn reset_collider_preserves_capsule_shape_variant() {
    let default_width = 10.0_f32;
    let default_height = 24.0_f32;
    let default = Collider {
        shape: ColliderShape::Aabb {
            width: default_width,
            height: default_height,
        },
        offset: Vec2::ZERO,
        ..Default::default()
};
    let mut collider = Collider {
        shape: ColliderShape::Capsule {
            radius: 15.0,
            height: 30.0,
        },
        offset: vec2(-4.0, 7.0),
        ..Default::default()
};

    reset_collider_to_default(&mut collider, &default);

    let expected_radius = default_width.min(default_height) / 2.0;
    let expected_height = default_height - expected_radius * 2.0;
    let ColliderShape::Capsule { radius, height } = collider.shape else {
        panic!("expected Capsule, got {:?}", collider.shape);
    };
    assert_eq!(radius, expected_radius);
    assert_eq!(height, expected_height);
    assert_eq!(collider.offset, Vec2::ZERO);
}

#[test]
fn reset_collider_preserves_point_shape_variant() {
    let mut collider = Collider {
        shape: ColliderShape::Point,
        offset: vec2(10.0, -20.0),
        ..Default::default()
};
    let default = Collider::default();

    reset_collider_to_default(&mut collider, &default);

    assert_eq!(collider.shape, ColliderShape::Point);
    assert_eq!(collider.offset, Vec2::ZERO);
}

#[test]
fn reset_collider_with_default_fallback_preserves_circle() {
    let mut collider = Collider {
        shape: ColliderShape::Circle { radius: 99.0 },
        offset: vec2(3.0, 4.0),
        ..Default::default()
};
    let default = Collider::default();

    reset_collider_to_default(&mut collider, &default);

    let expected_radius = DEFAULT_COLLIDER_DIMENSION.min(DEFAULT_COLLIDER_DIMENSION) / 2.0;
    let ColliderShape::Circle { radius } = collider.shape else {
        panic!("expected Circle, got {:?}", collider.shape);
    };
    assert_eq!(radius, expected_radius);
    assert_eq!(collider.offset, Vec2::ZERO);
}