// editor/src/menu_editor/ui.rs
use crate::gui::menu_bar::{draw_top_panel_full, menu_panel_rect};
use crate::menu::game_theme::{discover_themes, load_theme};
use crate::menu::MenuEditor;
use bishop::prelude::*;
use engine_core::ui::with_theme;
use engine_core::ui::Button;

impl MenuEditor {
    /// Draws the menu editor ui.
    pub fn draw_ui(&mut self, ctx: &mut WgpuContext) {
        const LEFT_COLUMN_WIDTH: f32 = 200.0;
        const PROPERTIES_WIDTH: f32 = 250.0;
        const SPACING: f32 = 8.0;
        const THEME_ROW_H: f32 = 28.0;

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

        // Game theme picker row
        let theme_row_rect = self.register_rect(Rect::new(
            screen_rect.x + SPACING,
            screen_rect.y + SPACING,
            LEFT_COLUMN_WIDTH,
            THEME_ROW_H,
        ));
        draw_theme_picker(ctx, theme_row_rect, self);

        let content_y = theme_row_rect.bottom() + SPACING;
        let remaining_h = screen_rect.h - THEME_ROW_H - SPACING * 3.0;
        let half_height = (remaining_h - SPACING) / 2.0;

        let menu_list_rect = self.register_rect(Rect::new(
            screen_rect.x + SPACING,
            content_y,
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
            with_theme(|theme| theme.background),
        );

        ctx.draw_rectangle_lines(
            menu_list_rect.x,
            menu_list_rect.y,
            menu_list_rect.w,
            menu_list_rect.h,
            1.0,
            with_theme(|theme| theme.border),
        );

        self.draw_menu_list_panel(ctx, menu_list_rect, blocked);

        // Draw element palette background
        ctx.draw_rectangle(
            palette_rect.x,
            palette_rect.y,
            palette_rect.w,
            palette_rect.h,
            with_theme(|theme| theme.background),
        );

        ctx.draw_rectangle_lines(
            palette_rect.x,
            palette_rect.y,
            palette_rect.w,
            palette_rect.h,
            1.0,
            with_theme(|theme| theme.border),
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
            with_theme(|theme| theme.background),
        );

        ctx.draw_rectangle_lines(
            properties_rect.x,
            properties_rect.y,
            properties_rect.w,
            properties_rect.h,
            1.0,
            with_theme(|theme| theme.border),
        );

        self.draw_properties_panel(ctx, properties_rect, blocked);

        // Draw top menu
        self.register_rect(draw_top_panel_full(ctx));
    }
}

fn draw_theme_picker(ctx: &mut WgpuContext, rect: Rect, editor: &mut MenuEditor) {
    let themes = discover_themes();

    let label_text = if let Some(ref name) = editor.selected_theme_name {
        format!("Theme: {name}")
    } else {
        "Theme: None".into()
    };

    if Button::new(rect, &label_text).suppressed(false).show(ctx) {
        // Cycle through themes + None option
        let current_idx = editor
            .selected_theme_name
            .as_ref()
            .and_then(|n| themes.iter().position(|t| t == n))
            .map(|i| i as isize)
            .unwrap_or(-1);

        let next_idx = current_idx + 1;
        if next_idx >= themes.len() as isize {
            editor.selected_theme_name = None;
            editor.game_theme = None;
        } else {
            let name = themes[next_idx as usize].clone();
            editor.game_theme = load_theme(&name);
            editor.selected_theme_name = Some(name);
        }
    }
}
