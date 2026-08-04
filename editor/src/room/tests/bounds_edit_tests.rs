use bishop::prelude::{Rect, Vec2, vec2};

use super::super::bounds_edit::{
    compute_circle_handles,
    compute_rect_handles,
    snap_rect_delta,
    HandleAction,
};

#[test]
fn rect_handles_include_corner_edge_and_center_actions() {
    let handles = compute_rect_handles(Rect::new(10.0, 20.0, 32.0, 16.0), 16.0);
    let actions: Vec<_> = handles.iter().map(|handle| handle.action).collect();

    assert_eq!(handles.len(), 9);
    assert!(actions.contains(&HandleAction::ResizeAabbTopLeft));
    assert!(actions.contains(&HandleAction::ResizeAabbTopRight));
    assert!(actions.contains(&HandleAction::ResizeAabbBottomLeft));
    assert!(actions.contains(&HandleAction::ResizeAabbBottomRight));
    assert!(actions.contains(&HandleAction::ResizeTop));
    assert!(actions.contains(&HandleAction::ResizeBottom));
    assert!(actions.contains(&HandleAction::ResizeLeft));
    assert!(actions.contains(&HandleAction::ResizeRight));
    assert!(actions.contains(&HandleAction::MoveOffset));
}

#[test]
fn circle_handles_include_move_and_radius_actions() {
    let handles = compute_circle_handles(vec2(32.0, 40.0), 12.0, 16.0);
    let radius_handles = handles
        .iter()
        .filter(|handle| handle.action == HandleAction::ResizeCircleRadius)
        .count();

    assert_eq!(handles.len(), 5);
    assert_eq!(radius_handles, 4);
    assert_eq!(handles.last().map(|handle| handle.action), Some(HandleAction::MoveOffset));
}

#[test]
fn snap_rect_delta_rounds_dragged_edge_to_grid() {
    let delta = snap_rect_delta(
        Rect::new(0.0, 0.0, 30.0, 18.0),
        HandleAction::ResizeRight,
        Vec2::new(5.0, 0.0),
        16.0,
    );

    assert_eq!(delta, vec2(2.0, 0.0));
}
