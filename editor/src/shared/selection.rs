// editor/src/shared/selection.rs
use bishop::prelude::*;

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

/// Draws a selection box rectangle in world space.
pub fn draw_selection_box(ctx: &mut WgpuContext, start: Vec2, end: Vec2) {
    let min_x = start.x.min(end.x);
    let min_y = start.y.min(end.y);
    let max_x = start.x.max(end.x);
    let max_y = start.y.max(end.y);
    let width = max_x - min_x;
    let height = max_y - min_y;

    // Semi-transparent fill
    ctx.draw_rectangle(min_x, min_y, width, height, Color::new(1.0, 1.0, 0.0, 0.1));
    // Yellow outline
    ctx.draw_rectangle_lines(min_x, min_y, width, height, 1.0, Color::YELLOW);
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
}
