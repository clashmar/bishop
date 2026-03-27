pub mod clipboard;
mod constants;
mod focus;
mod helpers;
mod state;
mod tab_registry;
pub mod widget_id;
mod widgets;

pub use clipboard::*;
pub use constants::*;
pub use focus::*;
pub use helpers::*;
pub use state::*;
pub use tab_registry::*;
pub use widget_id::*;
pub use widgets::*;

pub use bishop::BishopContext;
pub use bishop::TextDimensions;

pub use bishop::{Color, KeyCode, MouseButton, Rect, Vec2};

/// Draws text at the given position using the provided context.
pub(crate) fn draw_text_ui<C: BishopContext>(
    ctx: &mut C,
    text: &str,
    x: f32,
    y: f32,
    font_size: f32,
    color: Color,
) -> TextDimensions {
    ctx.draw_text(text, x, y, font_size, color)
}

/// Measures text dimensions using the provided context.
pub(crate) fn measure_text_ui<C: BishopContext>(
    ctx: &C,
    text: &str,
    font_size: f32,
) -> TextDimensions {
    ctx.measure_text(text, font_size)
}

/// Draws text within a clipped rectangle with horizontal scroll offset.
pub(crate) fn draw_text_clipped<C: BishopContext>(
    ctx: &mut C,
    text: &str,
    rect_x: f32,
    rect_y: f32,
    rect_w: f32,
    rect_h: f32,
    scroll_offset: f32,
    font_size: f32,
    color: Color,
) {
    let text_x = rect_x + WIDGET_PADDING / 2. - scroll_offset;
    let text_y = rect_y + rect_h * 0.7;

    let clip_left = rect_x + WIDGET_PADDING / 2.;
    let clip_right = rect_x + rect_w - WIDGET_PADDING / 2.;

    let mut visible_start_byte = 0;
    let mut visible_end_byte = text.len();
    let mut render_x = text_x;
    let mut found_start = false;

    for (byte_idx, ch) in text.char_indices() {
        let char_start_x = text_x + measure_text_ui(ctx, &text[..byte_idx], font_size).width;
        let char_end_x =
            text_x + measure_text_ui(ctx, &text[..byte_idx + ch.len_utf8()], font_size).width;

        if !found_start && char_start_x >= clip_left {
            visible_start_byte = byte_idx;
            render_x = char_start_x;
            found_start = true;
        }

        if char_end_x > clip_right {
            visible_end_byte = byte_idx;
            break;
        }
    }

    if visible_start_byte < visible_end_byte {
        let visible_text = &text[visible_start_byte..visible_end_byte];
        draw_text_ui(ctx, visible_text, render_x, text_y, font_size, color);
    }
}
