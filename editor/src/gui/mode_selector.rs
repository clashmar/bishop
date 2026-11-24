use crate::gui::gui_constants::*;
use crate::gui::modals::is_modal_open;
use bishop::prelude::*;
use engine_core::theme::with_theme;
use ::widgets::{color_picker_blocks_canvas, is_click_consumed, offer_tooltip};
use engine_core::ui::widgets::is_context_menu_open;

/// A trait that each editor’s mode enum must implement.
pub trait ModeInfo {
    /// Human‑readable label (used for tool‑tips etc).
    fn label(&self) -> &'static str;
    /// The texture that represents the mode.
    fn icon(&self) -> &'static Texture2D;
    /// Keyboard shortcut for the mode.
    fn shortcut(self) -> Option<fn(&WgpuContext) -> bool>;
}

/// The UI component.
pub struct ModeSelector<M: ModeInfo + Copy + PartialEq + 'static> {
    /// Currently active mode.
    pub current: M,
    /// All possible modes (order defines layout).
    pub options: &'static [M],
}

impl<M: ModeInfo + Copy + PartialEq> ModeSelector<M> {
    /// Draws icons and handles clicks. Returns the total Rect and whether the mode changed.
    pub fn draw(&mut self, ctx: &mut WgpuContext) -> (Rect, bool) {
        let mut changed = false;
        const PADDING: f32 = 8.0;
        let icon_size = MENU_PANEL_HEIGHT - 2.0 * PADDING;

        let total_width = self.options.len() as f32 * (icon_size + PADDING) - PADDING;
        let start_x = (ctx.screen_width() - total_width) / 2.0;

        let total_rect = Rect::new(start_x, PADDING - 2.0, total_width, MENU_PANEL_HEIGHT);

        for (i, mode) in self.options.iter().enumerate() {
            let x = start_x + i as f32 * (icon_size + PADDING);
            let rect = Rect::new(x, PADDING, icon_size, icon_size);

            // Highlight the active mode
            if *mode == self.current {
                ctx.draw_rectangle_lines(
                    rect.x - 2.0,
                    rect.y - 2.0,
                    rect.w + 4.0,
                    rect.h + 4.0,
                    2.0,
                    with_theme(|t| t.highlight),
                );
            }

            // Click handling
            if ctx.is_mouse_button_pressed(MouseButton::Left)
                && rect.contains(ctx.mouse_position().into())
                && !is_click_consumed()
                && !color_picker_blocks_canvas()
                && !is_modal_open()
                && !is_context_menu_open()
                && *mode != self.current
            {
                self.current = *mode;
                changed = true;
            }

            // Draw icon
            ctx.draw_texture_ex(
                mode.icon(),
                rect.x,
                rect.y,
                Color::WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(rect.w, rect.h)),
                    ..Default::default()
                },
            );
        }
        (total_rect, changed)
    }

    /// Offers tooltips for hovered mode icons.
    pub fn draw_tooltips(&self, ctx: &mut WgpuContext) {
        const PADDING: f32 = 8.0;
        let icon_size = MENU_PANEL_HEIGHT - 2.0 * PADDING;

        let total_width = self.options.len() as f32 * (icon_size + PADDING) - PADDING;
        let start_x = (ctx.screen_width() - total_width) / 2.0;

        for (i, mode) in self.options.iter().enumerate() {
            let x = start_x + i as f32 * (icon_size + PADDING);
            let rect = Rect::new(x, PADDING, icon_size, icon_size);

            if rect.contains(ctx.mouse_position().into())
                && !is_modal_open()
                && !is_context_menu_open()
            {
                offer_tooltip(rect, mode.label());
            }
        }
    }
}

/// Draws the background of the sub-mode strip.
pub fn draw_sub_mode_strip_background(
    ctx: &mut WgpuContext,
    anchor_x: f32,
    anchor_y: f32,
    option_count: usize,
) -> Rect {
    let (strip_rect, _, _) = sub_mode_strip_layout(anchor_x, anchor_y, option_count);

    ctx.draw_rectangle(
        strip_rect.x,
        strip_rect.y,
        strip_rect.w,
        strip_rect.h,
        with_theme(|t| t.panel),
    );

    strip_rect
}

/// Draws the sub-mode strip icons and handles interaction.
/// Call this after drawing the mode selector.
/// Returns the rect of the strip and whether the sub-mode changed.
pub fn draw_sub_mode_strip<S: ModeInfo + Copy + PartialEq + 'static>(
    ctx: &mut WgpuContext,
    anchor_x: f32,
    anchor_y: f32,
    options: &'static [S],
    current: &mut S,
) -> (Rect, bool) {
    let mut changed = false;
    let (strip_rect, start_x, icon_size) = sub_mode_strip_layout(anchor_x, anchor_y, options.len());
    const PADDING: f32 = 6.0;

    for (i, mode) in options.iter().enumerate() {
        let x = start_x + i as f32 * (icon_size + PADDING);
        let rect = Rect::new(x, anchor_y + PADDING, icon_size, icon_size);

        // Highlight the active sub-mode
        if *mode == *current {
            ctx.draw_rectangle_lines(
                rect.x - 2.0,
                rect.y - 2.0,
                rect.w + 4.0,
                rect.h + 4.0,
                2.0,
                with_theme(|t| t.highlight),
            );
        }

        // Click handling
        if ctx.is_mouse_button_pressed(MouseButton::Left)
            && rect.contains(ctx.mouse_position().into())
            && !is_click_consumed()
            && !color_picker_blocks_canvas()
            && !is_modal_open()
            && *mode != *current
        {
            *current = *mode;
            changed = true;
        }

        // Draw icon
        ctx.draw_texture_ex(
            mode.icon(),
            rect.x,
            rect.y,
            Color::WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(rect.w, rect.h)),
                ..Default::default()
            },
        );

        if rect.contains(ctx.mouse_position().into()) && !is_modal_open() {
            offer_tooltip(rect, mode.label());
        }
    }

    (strip_rect, changed)
}

fn sub_mode_strip_layout(anchor_x: f32, anchor_y: f32, option_count: usize) -> (Rect, f32, f32) {
    const PADDING: f32 = 6.0;
    let icon_size = (MENU_PANEL_HEIGHT - 2.0 * PADDING) * 0.75;
    let total_width = option_count as f32 * (icon_size + PADDING) - PADDING;
    let start_x = anchor_x + (MENU_PANEL_HEIGHT - total_width) / 2.0;

    let strip_rect = Rect::new(
        start_x - PADDING,
        anchor_y,
        total_width + PADDING * 2.0,
        icon_size + PADDING * 2.0,
    );

    (strip_rect, start_x, icon_size)
}
