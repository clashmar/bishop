use bishop::prelude::*;
use super::DevTools;

/// Draw the dev tools overlay in the top-right corner.
pub fn draw_dev_tools_overlay<C: BishopContext>(ctx: &mut C, dev_tools: &DevTools) {
    const PADDING: f32 = 10.0;
    const LINE_HEIGHT: f32 = 18.0;
    const FONT_SIZE: f32 = 14.0;
    const BG_ALPHA: f32 = 0.7;

    let lines = vec![format!(
        "Colliders: {}",
        if dev_tools.colliders_visible { "ON" } else { "OFF" }
    )];

    let max_width = lines
        .iter()
        .map(|s| ctx.measure_text(s, FONT_SIZE).width)
        .fold(0.0_f32, f32::max);

    let bg_width = max_width + PADDING * 2.0;
    let bg_height = lines.len() as f32 * LINE_HEIGHT + PADDING * 2.0;

    let x = ctx.screen_width() - bg_width - PADDING;
    let y = PADDING;

    ctx.draw_rectangle(x, y, bg_width, bg_height, Color::new(0.0, 0.0, 0.0, BG_ALPHA));

    for (i, line) in lines.iter().enumerate() {
        let text_y = y + PADDING + LINE_HEIGHT * i as f32 + FONT_SIZE;
        ctx.draw_text(line, x + PADDING, text_y, FONT_SIZE, Color::WHITE);
    }
}
