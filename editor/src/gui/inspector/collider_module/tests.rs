use bishop::prelude::*;
use engine_core::ecs::{Collider, ColliderShape, Pivot};
use widgets::constants::layout as layout_constants;

use super::body_layout;
use super::edit::{compute_handles, HandleAction};
use crate::world::coord::round_to_grid;

#[test]
fn layout_body_height_is_positive() {
    let body = body_layout();
    assert!(body.height() > 0.0, "body layout height should be positive");
}

#[test]
fn layout_body_height_includes_three_rows() {
    let body = body_layout();
    let expected = layout_constants::WIDGET_SPACING
        + super::ROW_H * 3.0
        + layout_constants::WIDGET_SPACING * 2.0
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
    };

    let handles = compute_handles(vec2(10.0, 20.0), Pivot::BottomCenter, &collider, 16.0);

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

    let handles = compute_handles(Vec2::ZERO, Pivot::TopLeft, &collider, 16.0);
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

    let handles = compute_handles(Vec2::ZERO, Pivot::TopLeft, &collider, grid_size);

    // 4 corners + 4 edges + 1 center = 9
    assert_eq!(handles.len(), 9);

    let actions: Vec<_> = handles.iter().map(|h| h.action).collect();
    assert!(actions.contains(&HandleAction::ResizeTop));
    assert!(actions.contains(&HandleAction::ResizeBottom));
    assert!(actions.contains(&HandleAction::ResizeLeft));
    assert!(actions.contains(&HandleAction::ResizeRight));
}
