use bishop::prelude::{Color, Draw, Rect, Vec2, WgpuContext, vec2};

use crate::world::coord::round_to_grid;

#[derive(Clone, Copy)]
pub(crate) struct BoundsEditConfig {
    pub grid_size: f32,
    pub snap_enabled: bool,
    pub shift_held: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct Handle {
    pub rect: Rect,
    pub action: HandleAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HandleAction {
    ResizeAabbTopLeft,
    ResizeAabbTopRight,
    ResizeAabbBottomLeft,
    ResizeAabbBottomRight,
    ResizeCircleRadius,
    ResizeCapsuleRadiusLeft,
    ResizeCapsuleRadiusRight,
    ResizeCapsuleHeightTop,
    ResizeCapsuleHeightBottom,
    ResizeTop,
    ResizeBottom,
    ResizeLeft,
    ResizeRight,
    MoveOffset,
}

pub(crate) fn compute_rect_handles(bounds: Rect, grid_size: f32) -> Vec<Handle> {
    let hs = grid_size * 0.1;
    let left = bounds.x;
    let top = bounds.y;
    let right = bounds.x + bounds.w;
    let bottom = bounds.y + bounds.h;
    let center_x = bounds.x + bounds.w * 0.5;
    let center_y = bounds.y + bounds.h * 0.5;

    vec![
        Handle {
            rect: Rect::new(left - hs, top - hs, hs * 2.0, hs * 2.0),
            action: HandleAction::ResizeAabbTopLeft,
        },
        Handle {
            rect: Rect::new(right - hs, top - hs, hs * 2.0, hs * 2.0),
            action: HandleAction::ResizeAabbTopRight,
        },
        Handle {
            rect: Rect::new(left - hs, bottom - hs, hs * 2.0, hs * 2.0),
            action: HandleAction::ResizeAabbBottomLeft,
        },
        Handle {
            rect: Rect::new(right - hs, bottom - hs, hs * 2.0, hs * 2.0),
            action: HandleAction::ResizeAabbBottomRight,
        },
        Handle {
            rect: Rect::new(center_x - hs, top - hs, hs * 2.0, hs * 2.0),
            action: HandleAction::ResizeTop,
        },
        Handle {
            rect: Rect::new(center_x - hs, bottom - hs, hs * 2.0, hs * 2.0),
            action: HandleAction::ResizeBottom,
        },
        Handle {
            rect: Rect::new(left - hs, center_y - hs, hs * 2.0, hs * 2.0),
            action: HandleAction::ResizeLeft,
        },
        Handle {
            rect: Rect::new(right - hs, center_y - hs, hs * 2.0, hs * 2.0),
            action: HandleAction::ResizeRight,
        },
        Handle {
            rect: Rect::new(center_x - hs, center_y - hs, hs * 2.0, hs * 2.0),
            action: HandleAction::MoveOffset,
        },
    ]
}

pub(crate) fn compute_circle_handles(center: Vec2, radius: f32, grid_size: f32) -> Vec<Handle> {
    let hs = grid_size * 0.1;

    vec![
        Handle {
            rect: Rect::new(center.x + radius - hs, center.y - hs, hs * 2.0, hs * 2.0),
            action: HandleAction::ResizeCircleRadius,
        },
        Handle {
            rect: Rect::new(center.x - radius - hs, center.y - hs, hs * 2.0, hs * 2.0),
            action: HandleAction::ResizeCircleRadius,
        },
        Handle {
            rect: Rect::new(center.x - hs, center.y + radius - hs, hs * 2.0, hs * 2.0),
            action: HandleAction::ResizeCircleRadius,
        },
        Handle {
            rect: Rect::new(center.x - hs, center.y - radius - hs, hs * 2.0, hs * 2.0),
            action: HandleAction::ResizeCircleRadius,
        },
        Handle {
            rect: Rect::new(center.x - hs, center.y - hs, hs * 2.0, hs * 2.0),
            action: HandleAction::MoveOffset,
        },
    ]
}

pub(crate) fn hit_test_handles(mouse_pos: Vec2, handles: &[Handle]) -> Option<usize> {
    handles.iter().position(|handle| handle.rect.contains(mouse_pos))
}

pub(crate) fn draw_handles(ctx: &mut WgpuContext, handles: &[Handle]) {
    for handle in handles {
        ctx.draw_rectangle(
            handle.rect.x,
            handle.rect.y,
            handle.rect.w,
            handle.rect.h,
            Color::WHITE,
        );
        ctx.draw_rectangle_lines(
            handle.rect.x,
            handle.rect.y,
            handle.rect.w,
            handle.rect.h,
            0.5,
            Color::BLACK,
        );
    }
}

pub(crate) fn snap_rect_delta(
    bounds: Rect,
    action: HandleAction,
    delta: Vec2,
    grid_size: f32,
) -> Vec2 {
    let left = bounds.x;
    let top = bounds.y;
    let right = bounds.x + bounds.w;
    let bottom = bounds.y + bounds.h;

    match action {
        HandleAction::ResizeAabbTopLeft => vec2(
            round_to_grid(left + delta.x, grid_size) - left,
            round_to_grid(top + delta.y, grid_size) - top,
        ),
        HandleAction::ResizeAabbTopRight => vec2(
            round_to_grid(right + delta.x, grid_size) - right,
            round_to_grid(top + delta.y, grid_size) - top,
        ),
        HandleAction::ResizeAabbBottomLeft => vec2(
            round_to_grid(left + delta.x, grid_size) - left,
            round_to_grid(bottom + delta.y, grid_size) - bottom,
        ),
        HandleAction::ResizeAabbBottomRight => vec2(
            round_to_grid(right + delta.x, grid_size) - right,
            round_to_grid(bottom + delta.y, grid_size) - bottom,
        ),
        HandleAction::ResizeTop => vec2(
            delta.x,
            round_to_grid(top + delta.y, grid_size) - top,
        ),
        HandleAction::ResizeBottom => vec2(
            delta.x,
            round_to_grid(bottom + delta.y, grid_size) - bottom,
        ),
        HandleAction::ResizeLeft => vec2(
            round_to_grid(left + delta.x, grid_size) - left,
            delta.y,
        ),
        HandleAction::ResizeRight => vec2(
            round_to_grid(right + delta.x, grid_size) - right,
            delta.y,
        ),
        _ => delta,
    }
}

#[cfg(test)]
#[path = "tests/bounds_edit_tests.rs"]
mod tests;
