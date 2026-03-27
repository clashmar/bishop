// editor/src/menu_editor/ui.rs
use crate::gui::menu_bar::{draw_top_panel_full, menu_panel_rect};
use crate::menu::MenuEditor;
use bishop::prelude::*;

impl MenuEditor {
    /// Draws the menu editor ui.
    pub fn draw_ui(&mut self, ctx: &mut WgpuContext) {
        const LEFT_COLUMN_WIDTH: f32 = 200.0;
        const PROPERTIES_WIDTH: f32 = 250.0;
        const SPACING: f32 = 8.0;

        let blocked = false;

        // Reset to static camera
        ctx.set_default_camera();

        // Calculate top panel
        let menu_panel = menu_panel_rect(ctx);

        let screen_rect = Rect::new(
            0.0,
            menu_panel.h,
            ctx.screen_width(),
            ctx.screen_height() - menu_panel.h,
        );

        let half_height = (screen_rect.h - SPACING * 3.0) / 2.0;

        let menu_list_rect = self.register_rect(Rect::new(
            screen_rect.x + SPACING,
            screen_rect.y + SPACING,
            LEFT_COLUMN_WIDTH,
            half_height,
        ));

        let palette_rect = self.register_rect(Rect::new(
            screen_rect.x + SPACING,
            menu_list_rect.bottom() + SPACING,
            LEFT_COLUMN_WIDTH,
            half_height,
        ));

        let properties_rect = self.register_rect(Rect::new(
            screen_rect.right() - PROPERTIES_WIDTH - SPACING,
            screen_rect.y + SPACING,
            PROPERTIES_WIDTH,
            screen_rect.h - SPACING * 2.0,
        ));

        // Draw menu list background
        ctx.draw_rectangle(
            menu_list_rect.x,
            menu_list_rect.y,
            menu_list_rect.w,
            menu_list_rect.h,
            Color::new(0.15, 0.15, 0.18, 1.0),
        );

        ctx.draw_rectangle_lines(
            menu_list_rect.x,
            menu_list_rect.y,
            menu_list_rect.w,
            menu_list_rect.h,
            1.0,
            Color::new(0.4, 0.4, 0.4, 1.0),
        );

        self.draw_menu_list_panel(ctx, menu_list_rect, blocked);

        // Draw element palette background
        ctx.draw_rectangle(
            palette_rect.x,
            palette_rect.y,
            palette_rect.w,
            palette_rect.h,
            Color::new(0.15, 0.15, 0.18, 1.0),
        );

        ctx.draw_rectangle_lines(
            palette_rect.x,
            palette_rect.y,
            palette_rect.w,
            palette_rect.h,
            1.0,
            Color::new(0.4, 0.4, 0.4, 1.0),
        );

        // Handle palette clicks to set pending element type
        if let Some(kind) = self.element_palette.draw(ctx, palette_rect, blocked) {
            self.pending_element_type = Some(kind);
        }

        // Draw properties background
        ctx.draw_rectangle(
            properties_rect.x,
            properties_rect.y,
            properties_rect.w,
            properties_rect.h,
            Color::new(0.15, 0.15, 0.18, 1.0),
        );

        ctx.draw_rectangle_lines(
            properties_rect.x,
            properties_rect.y,
            properties_rect.w,
            properties_rect.h,
            1.0,
            Color::new(0.4, 0.4, 0.4, 1.0),
        );

        self.draw_properties_panel(ctx, properties_rect, blocked);

        // Draw top menu
        self.register_rect(draw_top_panel_full(ctx));
    }
}
