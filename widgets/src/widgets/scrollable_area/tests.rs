use super::*;
use crate::widgets::test_support::WidgetTestContext;

fn active_area(
    rect: Rect,
    content_height: f32,
) -> (ActiveScrollArea, ScrollState, WidgetTestContext) {
    let mut ctx = WidgetTestContext::new();
    let mut state = ScrollState::new();
    let area = ScrollableArea::new(rect, content_height).begin(&mut ctx, &mut state);
    (area, state, ctx)
}

#[test]
fn scroll_state_new_starts_at_top_without_auto_scroll() {
    let state = ScrollState::new();
    assert_eq!(state.scroll_y, 0.0);
    assert!(!state.auto_scroll);
}

#[test]
fn scroll_state_with_auto_scroll_starts_at_top_with_auto_scroll() {
    let state = ScrollState::with_auto_scroll();
    assert_eq!(state.scroll_y, 0.0);
    assert!(state.auto_scroll);
}

#[test]
fn scroll_state_default_matches_new() {
    let default_state = ScrollState::default();
    let new_state = ScrollState::new();
    assert_eq!(default_state.scroll_y, new_state.scroll_y);
    assert_eq!(default_state.auto_scroll, new_state.auto_scroll);
}

#[test]
fn begin_scroll_range_is_zero_when_content_fits() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let (area, _, _) = active_area(rect, 100.0);
    assert_eq!(area.scroll_range(), 0.0);
}

#[test]
fn begin_scroll_range_is_positive_when_content_exceeds_rect() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let (area, _, _) = active_area(rect, 300.0);
    assert_eq!(area.scroll_range(), 200.0);
}

#[test]
fn begin_clamps_scroll_y_to_valid_range() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let mut ctx = WidgetTestContext::new();
    let mut state = ScrollState::new();
    state.scroll_y = -999.0;

    let _ = ScrollableArea::new(rect, 300.0).begin(&mut ctx, &mut state);

    assert_eq!(state.scroll_y, -200.0);
}

#[test]
fn begin_clamps_positive_scroll_y_to_zero() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let mut ctx = WidgetTestContext::new();
    let mut state = ScrollState::new();
    state.scroll_y = 50.0;

    let _ = ScrollableArea::new(rect, 300.0).begin(&mut ctx, &mut state);

    assert_eq!(state.scroll_y, 0.0);
}

#[test]
fn begin_wheel_scroll_moves_scroll_y() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (60.0, 50.0);
    ctx.mouse_wheel_delta = (0.0, -1.0);
    let mut state = ScrollState::new();

    let _ = ScrollableArea::new(rect, 300.0).begin(&mut ctx, &mut state);

    assert!(state.scroll_y < 0.0);
}

#[test]
fn begin_wheel_scroll_clears_auto_scroll() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (60.0, 50.0);
    ctx.mouse_wheel_delta = (0.0, -1.0);
    let mut state = ScrollState::with_auto_scroll();

    let _ = ScrollableArea::new(rect, 300.0).begin(&mut ctx, &mut state);

    assert!(!state.auto_scroll);
}

#[test]
fn begin_wheel_scroll_blocked_when_mouse_outside_rect() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (200.0, 50.0);
    ctx.mouse_wheel_delta = (0.0, -1.0);
    let mut state = ScrollState::new();

    let _ = ScrollableArea::new(rect, 300.0).begin(&mut ctx, &mut state);

    assert_eq!(state.scroll_y, 0.0);
}

#[test]
fn begin_wheel_scroll_blocked_when_area_is_blocked() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let mut ctx = WidgetTestContext::new();
    ctx.mouse_pos = (60.0, 50.0);
    ctx.mouse_wheel_delta = (0.0, -1.0);
    let mut state = ScrollState::new();

    let _ = ScrollableArea::new(rect, 300.0)
        .blocked(true)
        .begin(&mut ctx, &mut state);

    assert_eq!(state.scroll_y, 0.0);
}

#[test]
fn begin_auto_scroll_jumps_to_bottom() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let mut ctx = WidgetTestContext::new();
    let mut state = ScrollState::with_auto_scroll();

    let _ = ScrollableArea::new(rect, 300.0).begin(&mut ctx, &mut state);

    assert_eq!(state.scroll_y, -200.0);
}

#[test]
fn begin_auto_scroll_reenabled_when_near_bottom() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let mut ctx = WidgetTestContext::new();
    let mut state = ScrollState::new();
    state.scroll_y = -199.5;

    let _ = ScrollableArea::new(rect, 300.0).begin(&mut ctx, &mut state);

    assert!(state.auto_scroll);
}

#[test]
fn content_rect_shrinks_when_scrollable() {
    let rect = Rect::new(10.0, 20.0, 120.0, 100.0);
    let (area, _, _) = active_area(rect, 300.0);
    let cr = area.content_rect();
    assert!(cr.w < rect.w);
    assert_eq!(cr.x, rect.x);
    assert_eq!(cr.y, rect.y);
    assert_eq!(cr.h, rect.h);
}

#[test]
fn content_rect_unchanged_when_content_fits() {
    let rect = Rect::new(10.0, 20.0, 120.0, 100.0);
    let (area, _, _) = active_area(rect, 100.0);
    let cr = area.content_rect();
    assert_eq!(cr.x, rect.x);
    assert_eq!(cr.w, rect.w);
}

#[test]
fn usable_width_shrinks_when_scrollable() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let (area, _, _) = active_area(rect, 300.0);
    assert!(area.usable_width() < rect.w);
}

#[test]
fn usable_width_unchanged_when_content_fits() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let (area, _, _) = active_area(rect, 100.0);
    assert_eq!(area.usable_width(), rect.w - CONTENT_MARGIN);
}

#[test]
fn is_visible_true_for_partially_overlapping_item() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let (area, _, _) = active_area(rect, 300.0);
    assert!(area.is_visible(-10.0, 20.0));
    assert!(area.is_visible(90.0, 20.0));
}

#[test]
fn is_visible_false_for_fully_outside_item() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let (area, _, _) = active_area(rect, 300.0);
    assert!(!area.is_visible(-20.0, 10.0));
    assert!(!area.is_visible(105.0, 10.0));
}

#[test]
fn is_fully_visible_true_for_contained_item() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let (area, _, _) = active_area(rect, 300.0);
    assert!(area.is_fully_visible(10.0, 20.0));
}

#[test]
fn is_fully_visible_false_for_partially_overlapping_item() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let (area, _, _) = active_area(rect, 300.0);
    assert!(!area.is_fully_visible(-5.0, 20.0));
    assert!(!area.is_fully_visible(85.0, 20.0));
}

#[test]
fn drag_edge_autoscroll_is_noop_when_drag_is_inactive() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let (area, mut state, mut ctx) = active_area(rect, 300.0);
    state.scroll_y = -40.0;
    ctx.mouse_pos = (60.0, 4.0);

    let changed = area.apply_drag_edge_autoscroll(&ctx, &mut state, false);

    assert!(!changed);
    assert_eq!(state.scroll_y, -40.0);
}

#[test]
fn drag_edge_autoscroll_is_noop_when_content_fits() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let (area, mut state, mut ctx) = active_area(rect, 100.0);
    ctx.mouse_pos = (60.0, 4.0);

    let changed = area.apply_drag_edge_autoscroll(&ctx, &mut state, true);

    assert!(!changed);
    assert_eq!(state.scroll_y, 0.0);
}

#[test]
fn drag_edge_autoscroll_is_noop_away_from_edge_bands() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let (area, mut state, mut ctx) = active_area(rect, 300.0);
    state.scroll_y = -40.0;
    ctx.mouse_pos = (60.0, 50.0);

    let changed = area.apply_drag_edge_autoscroll(&ctx, &mut state, true);

    assert!(!changed);
    assert_eq!(state.scroll_y, -40.0);
}

#[test]
fn drag_edge_autoscroll_moves_up_in_top_band() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let (area, mut state, mut ctx) = active_area(rect, 300.0);
    state.scroll_y = -40.0;
    ctx.mouse_pos = (60.0, 2.0);

    let changed = area.apply_drag_edge_autoscroll(&ctx, &mut state, true);

    assert!(changed);
    assert!(state.scroll_y > -40.0);
    assert!(state.scroll_y <= 0.0);
}

#[test]
fn drag_edge_autoscroll_moves_down_in_bottom_band() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let (area, mut state, mut ctx) = active_area(rect, 300.0);
    state.scroll_y = -40.0;
    ctx.mouse_pos = (60.0, 98.0);

    let changed = area.apply_drag_edge_autoscroll(&ctx, &mut state, true);

    assert!(changed);
    assert!(state.scroll_y < -40.0);
}

#[test]
fn drag_edge_autoscroll_clamps_at_top_and_bottom() {
    let rect = Rect::new(0.0, 0.0, 120.0, 100.0);
    let (area, mut state, mut ctx) = active_area(rect, 300.0);

    state.scroll_y = 0.0;
    ctx.mouse_pos = (60.0, 1.0);
    let changed_top = area.apply_drag_edge_autoscroll(&ctx, &mut state, true);
    assert!(!changed_top);
    assert_eq!(state.scroll_y, 0.0);

    state.scroll_y = -200.0;
    ctx.mouse_pos = (60.0, 99.0);
    let changed_bottom = area.apply_drag_edge_autoscroll(&ctx, &mut state, true);
    assert!(!changed_bottom);
    assert_eq!(state.scroll_y, -200.0);
}
