use bishop::prelude::*;
use engine_core::rendering::{outline_thickness};
use widgets::*;

const BOX_SELECTION_DRAG_THRESHOLD_PX: f32 = 6.0;

/// Creates a Rect from two corner points, handling any orientation.
pub fn rect_from_two_points(a: Vec2, b: Vec2) -> Rect {
    let min_x = a.x.min(b.x);
    let min_y = a.y.min(b.y);
    let max_x = a.x.max(b.x);
    let max_y = a.y.max(b.y);
    Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

/// Returns true if two rectangles intersect.
pub fn rects_intersect(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y
}

/// Returns true when a marquee gesture moved far enough to count as a drag.
pub fn box_selection_drag_started(start_screen: Vec2, end_screen: Vec2) -> bool {
    (end_screen - start_screen).length_squared()
        >= BOX_SELECTION_DRAG_THRESHOLD_PX * BOX_SELECTION_DRAG_THRESHOLD_PX
}

/// Draws a selection box rectangle in world space.
pub fn draw_selection_box(ctx: &mut WgpuContext, start: Vec2, end: Vec2, grid_size: f32) {
    let min_x = start.x.min(end.x);
    let min_y = start.y.min(end.y);
    let max_x = start.x.max(end.x);
    let max_y = start.y.max(end.y);
    let width = max_x - min_x;
    let height = max_y - min_y;

    let color = with_theme(|t| t.highlight);
    ctx.draw_rectangle(min_x, min_y, width, height, color.with_alpha(0.1));
    ctx.draw_rectangle_lines(
        min_x,
        min_y,
        width,
        height,
        outline_thickness(grid_size) * 0.25,
        color,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rects_intersect_detects_overlap() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 5.0, 10.0, 10.0);
        assert!(rects_intersect(a, b));
    }

    #[test]
    fn rects_intersect_detects_non_overlap() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(20.0, 20.0, 10.0, 10.0);
        assert!(!rects_intersect(a, b));
    }

    #[test]
    fn rect_from_two_points_creates_correct_rect() {
        let a = Vec2::new(100.0, 50.0);
        let b = Vec2::new(200.0, 150.0);
        let r = rect_from_two_points(a, b);
        assert_eq!(r.x, 100.0);
        assert_eq!(r.y, 50.0);
        assert_eq!(r.w, 100.0);
        assert_eq!(r.h, 100.0);
    }

    #[test]
    fn rect_from_two_points_handles_reversed_input() {
        let a = Vec2::new(200.0, 150.0);
        let b = Vec2::new(100.0, 50.0);
        let r = rect_from_two_points(a, b);
        assert_eq!(r.x, 100.0);
        assert_eq!(r.y, 50.0);
        assert_eq!(r.w, 100.0);
        assert_eq!(r.h, 100.0);
    }

    #[test]
    fn box_selection_requires_drag_threshold() {
        assert!(!box_selection_drag_started(
            Vec2::new(100.0, 100.0),
            Vec2::new(102.0, 101.0),
        ));
        assert!(box_selection_drag_started(
            Vec2::new(100.0, 100.0),
            Vec2::new(108.0, 100.0),
        ));
    }
}
