// editor/src/gui/mode_selector.rs
use crate::gui::gui_constants::*;
use crate::gui::modal::is_modal_open;
use bishop::prelude::*;
use engine_core::ui::text::*;

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
                    Color::YELLOW,
                );
            }

            // Click handling
            if ctx.is_mouse_button_pressed(MouseButton::Left)
                && rect.contains(ctx.mouse_position().into())
                && !is_modal_open()
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

    /// Draws tooltips for hovered mode icons. Call this after other UI elements
    /// to ensure tooltips appear on top.
    pub fn draw_tooltips(&self, ctx: &mut WgpuContext) {
        const PADDING: f32 = 8.0;
        let icon_size = MENU_PANEL_HEIGHT - 2.0 * PADDING;

        let total_width = self.options.len() as f32 * (icon_size + PADDING) - PADDING;
        let start_x = (ctx.screen_width() - total_width) / 2.0;

        for (i, mode) in self.options.iter().enumerate() {
            let x = start_x + i as f32 * (icon_size + PADDING);
            let rect = Rect::new(x, PADDING, icon_size, icon_size);

            if rect.contains(ctx.mouse_position().into()) && !is_modal_open() {
                let tip = mode.label();
                let tip_size = measure_text(ctx, tip, 16.0);

                let tip_rect = Rect::new(rect.x, rect.y + rect.h + 4.0, tip_size.width + 8.0, 20.0);

                ctx.draw_rectangle(
                    tip_rect.x,
                    tip_rect.y,
                    tip_rect.w,
                    tip_rect.h,
                    Color::new(0.0, 0.0, 0.0, 0.8),
                );

                ctx.draw_text(tip, tip_rect.x + 4.0, tip_rect.y + 15.0, 16.0, Color::WHITE);
            }
        }
    }
}

/// Computes the sub-mode strip layout values.
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

/// Draws only the background of the sub-mode strip.
/// Call this before drawing the mode selector so tooltips appear on top.
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
        PANEL_COLOR,
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
                Color::YELLOW,
            );
        }

        // Click handling
        if ctx.is_mouse_button_pressed(MouseButton::Left)
            && rect.contains(ctx.mouse_position().into())
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

        // Tooltip
        if rect.contains(ctx.mouse_position().into()) && !is_modal_open() {
            let tip = mode.label();
            let tip_size = measure_text(ctx, tip, 16.0);

            let tip_rect = Rect::new(rect.x, rect.y + rect.h + 4.0, tip_size.width + 8.0, 20.0);

            ctx.draw_rectangle(
                tip_rect.x,
                tip_rect.y,
                tip_rect.w,
                tip_rect.h,
                Color::new(0.0, 0.0, 0.0, 0.8),
            );

            ctx.draw_text(tip, tip_rect.x + 4.0, tip_rect.y + 15.0, 16.0, Color::WHITE);
        }
    }

    (strip_rect, changed)
}
