// editor/src/menu_editor/resize_handle.rs
use bishop::prelude::*;

const HANDLE_SIZE: f32 = 6.0;
const HALF: f32 = HANDLE_SIZE / 2.0;
const HIT_SIZE: f32 = 10.0;
const HIT_HALF: f32 = HIT_SIZE / 2.0;
const MIN_SIZE: f32 = 0.01;

/// Which of the 8 resize handles is being dragged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HandlePosition {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

/// Active resize drag state.
pub struct ResizeHandleState {
    pub element_index: usize,
    /// Set when resizing a child element inside a layout group.
    pub child_index: Option<usize>,
    pub handle: HandlePosition,
    pub original_rect: Rect,
    pub start_mouse: Vec2,
}

/// Returns the screen-space center positions for all 8 handles around `rect`.
pub fn handle_centers(rect: Rect) -> [(f32, f32); 8] {
    [
        (rect.x, rect.y),                         // TopLeft
        (rect.x + rect.w / 2.0, rect.y),          // Top
        (rect.x + rect.w, rect.y),                // TopRight
        (rect.x + rect.w, rect.y + rect.h / 2.0), // Right
        (rect.x + rect.w, rect.y + rect.h),       // BottomRight
        (rect.x + rect.w / 2.0, rect.y + rect.h), // Bottom
        (rect.x, rect.y + rect.h),                // BottomLeft
        (rect.x, rect.y + rect.h / 2.0),          // Left
    ]
}

const HANDLE_ORDER: [HandlePosition; 8] = [
    HandlePosition::TopLeft,
    HandlePosition::Top,
    HandlePosition::TopRight,
    HandlePosition::Right,
    HandlePosition::BottomRight,
    HandlePosition::Bottom,
    HandlePosition::BottomLeft,
    HandlePosition::Left,
];

/// Returns which handle the mouse is over, if any.
pub fn hit_test_handles(world_mouse: Vec2, element_screen_rect: Rect) -> Option<HandlePosition> {
    let centers = handle_centers(element_screen_rect);
    for (i, (cx, cy)) in centers.iter().enumerate() {
        let hit = Rect::new(cx - HIT_HALF, cy - HIT_HALF, HIT_SIZE, HIT_SIZE);
        if hit.contains(world_mouse) {
            return Some(HANDLE_ORDER[i]);
        }
    }
    None
}

/// Applies a normalized mouse delta to produce a new rect for the given handle.
pub fn apply_resize(original: Rect, handle: HandlePosition, delta: Vec2) -> Rect {
    let mut x = original.x;
    let mut y = original.y;
    let mut w = original.w;
    let mut h = original.h;

    match handle {
        HandlePosition::TopLeft => {
            x += delta.x;
            y += delta.y;
            w -= delta.x;
            h -= delta.y;
        }
        HandlePosition::Top => {
            y += delta.y;
            h -= delta.y;
        }
        HandlePosition::TopRight => {
            y += delta.y;
            w += delta.x;
            h -= delta.y;
        }
        HandlePosition::Right => {
            w += delta.x;
        }
        HandlePosition::BottomRight => {
            w += delta.x;
            h += delta.y;
        }
        HandlePosition::Bottom => {
            h += delta.y;
        }
        HandlePosition::BottomLeft => {
            x += delta.x;
            w -= delta.x;
            h += delta.y;
        }
        HandlePosition::Left => {
            x += delta.x;
            w -= delta.x;
        }
    }

    // Clamp to minimum size
    if w < MIN_SIZE {
        if matches!(
            handle,
            HandlePosition::TopLeft | HandlePosition::BottomLeft | HandlePosition::Left
        ) {
            x = original.x + original.w - MIN_SIZE;
        }
        w = MIN_SIZE;
    }
    if h < MIN_SIZE {
        if matches!(
            handle,
            HandlePosition::TopLeft | HandlePosition::TopRight | HandlePosition::Top
        ) {
            y = original.y + original.h - MIN_SIZE;
        }
        h = MIN_SIZE;
    }

    Rect::new(x, y, w, h)
}

/// Applies a resize while keeping the element's center fixed on unaffected axes.
///
/// Edge handles (Top, Bottom, Left, Right) only re-center the axis being resized.
/// Corner handles re-center both axes.
pub fn apply_resize_centered(original: Rect, handle: HandlePosition, delta: Vec2) -> Rect {
    let resized = apply_resize(original, handle, delta);
    let cx = original.x + original.w / 2.0;
    let cy = original.y + original.h / 2.0;

    let (x, y) = match handle {
        HandlePosition::Top | HandlePosition::Bottom => (resized.x, cy - resized.h / 2.0),
        HandlePosition::Left | HandlePosition::Right => (cx - resized.w / 2.0, resized.y),
        _ => (cx - resized.w / 2.0, cy - resized.h / 2.0),
    };

    Rect::new(x, y, resized.w, resized.h)
}

/// Draws the 8 resize handles around `rect` in screen space.
pub fn draw_resize_handles(ctx: &mut WgpuContext, rect: Rect) {
    let centers = handle_centers(rect);
    for (cx, cy) in centers {
        ctx.draw_rectangle(cx - HALF, cy - HALF, HANDLE_SIZE, HANDLE_SIZE, Color::WHITE);
        ctx.draw_rectangle_lines(
            cx - HALF,
            cy - HALF,
            HANDLE_SIZE,
            HANDLE_SIZE,
            1.0,
            Color::new(0.3, 0.3, 0.3, 1.0),
        );
    }
}
