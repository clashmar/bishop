use bishop::prelude::*;

/// Draws text using the provided context.
pub fn draw_text_ui<C: BishopContext>(
    ctx: &mut C,
    text: &str,
    x: f32,
    y: f32,
    font_size: f32,
    color: impl Into<Color>,
) -> TextDimensions {
    ctx.draw_text(text, x, y, font_size, color.into())
}

/// Measures text using the provided context.
pub fn measure_text<C: BishopContext>(ctx: &C, text: &str, font_size: f32) -> TextDimensions {
    ctx.measure_text(text, font_size)
}

/// Centers text horizontally around a given x position.
pub fn center_text<C: BishopContext>(ctx: &C, x: f32, text: &str, font_size: f32) -> (f32, f32) {
    let text_size = measure_text(ctx, text, font_size);
    let new_x = x - (text_size.width / 2.);
    (new_x, text_size.width)
}
