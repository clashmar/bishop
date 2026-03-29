// editor/src/tilemap/background_module.rs
use bishop::prelude::*;
use engine_core::prelude::*;

// TODO: Add more complex backgrounds
/// Responsible for editing the background of a tilemap.
pub struct BackgroundModule {
    pub r_id: WidgetId,
    pub g_id: WidgetId,
    pub b_id: WidgetId,
    pub a_id: WidgetId,
}

impl BackgroundModule {
    pub fn new() -> Self {
        Self {
            r_id: WidgetId::default(),
            g_id: WidgetId::default(),
            b_id: WidgetId::default(),
            a_id: WidgetId::default(),
        }
    }

    pub fn draw(&mut self, ctx: &mut WgpuContext, rect: Rect, map: &mut TileMap, blocked: bool) {
        // Title
        ctx.draw_text(
            "Background",
            rect.x,
            rect.y + 18.0,
            DEFAULT_FONT_SIZE_16,
            Color::WHITE,
        );

        let mut r = map.background.r * 255.0;
        let mut g = map.background.g * 255.0;
        let mut b = map.background.b * 255.0;
        let mut a = map.background.a * 255.0;

        // Determine the width of a three‑digit number.
        // 255 is the widest possible value for an 8‑bit channel
        let sample = "255";
        let num_width = measure_text(ctx, sample, DEFAULT_FONT_SIZE_16).width;

        // Add padding so the cursor isn’t glued to the edge.
        let field_w = num_width + 13.0;
        let field_h = 30.0;
        let spacing = 5.0;

        // Position the four numeric inputs.
        let mut x = rect.x + 10.0;
        let y = rect.y + 30.0;

        r = NumberInput::new(self.r_id, Rect::new(x, y, field_w, field_h), r)
            .blocked(blocked)
            .show(ctx);
        x += field_w + spacing;
        g = NumberInput::new(self.g_id, Rect::new(x, y, field_w, field_h), g)
            .blocked(blocked)
            .show(ctx);
        x += field_w + spacing;
        b = NumberInput::new(self.b_id, Rect::new(x, y, field_w, field_h), b)
            .blocked(blocked)
            .show(ctx);
        x += field_w + spacing;
        a = NumberInput::new(self.a_id, Rect::new(x, y, field_w, field_h), a)
            .blocked(blocked)
            .show(ctx);
        x += field_w + spacing;

        // Clamp to a valid range (0‑255) and push the colour back
        r = r.clamp(0.0, 255.0);
        g = g.clamp(0.0, 255.0);
        b = b.clamp(0.0, 255.0);
        a = a.clamp(0.0, 255.0);

        map.background = Color::new(r / 255.0, g / 255.0, b / 255.0, a / 255.0);

        // Preview square
        let preview_sz = field_h; // same height as the input fields
        let preview_rect = Rect::new(x, y, preview_sz, preview_sz);
        ctx.draw_rectangle(
            preview_rect.x,
            preview_rect.y,
            preview_rect.w,
            preview_rect.h,
            map.background,
        );
        ctx.draw_rectangle_lines(
            preview_rect.x,
            preview_rect.y,
            preview_rect.w,
            preview_rect.h,
            2.0,
            Color::WHITE,
        );
    }
}
