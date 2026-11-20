use crate::gui::gui_constants::MENU_PANEL_HEIGHT;
use crate::gui::panel_text_color;
use crate::Editor;
use bishop::prelude::*;
use ::widgets::*;
use ::widgets::{focused_panel, is_context_menu_open, set_focused_panel};

/// Must be globally unique.
pub type PanelId = &'static str;

/// Minimum dimensions for panels that support resizing.
#[derive(Clone, Copy)]
pub struct PanelResizeLimits {
    /// Minimum panel width.
    pub min_width: f32,
    /// Minimum panel height.
    pub min_height: f32,
}

/// Defines the features and content of the panel.
pub trait PanelDefinition {
    /// Unique title (also used as id).
    fn title(&self) -> &'static str;
    /// Default rect when first created.
    fn default_rect(&self, ctx: &WgpuContext) -> Rect;
    /// Draws panel contents. When `blocked` is true, the panel should not respond to mouse input.
    fn draw(&mut self, ctx: &mut WgpuContext, rect: Rect, editor: &mut Editor, blocked: bool);
    /// Returns the panel's resize constraints, if resizing is enabled.
    fn resize_limits(&self) -> Option<PanelResizeLimits> {
        None
    }
    /// Returns whether the panel owns an active interaction that Escape can cancel.
    fn has_active_escape_interaction(&self) -> bool {
        false
    }
    /// Cancels the active Escape-owned interaction.
    fn cancel_escape_interaction(&mut self) {}
    /// Return `true` if the panel rendered its own title content, skipping the default text.
    fn draw_custom_title(
        &mut self,
        _ctx: &mut WgpuContext,
        _title_bar: Rect,
        _blocked: bool,
    ) -> bool {
        false
    }
    /// Called when this panel loses focus. Override to clear selection, stop drag operations, etc.
    fn on_defocus(&mut self) {}
}

/// Movable and collabsible panel to be composed with the supplied `PanelDefinition`.
pub struct GenericPanel {
    pub title: &'static str,
    pub rect: Rect,
    pub visible: bool,
    /// Whether this panel is registered for the current editor mode.
    pub in_current_mode: bool,
    pub collapsed: bool,
    pub dragging: bool,
    pub(crate) resize_limits: Option<PanelResizeLimits>,
    drag_offset: Vec2,
    pub(crate) resizing: bool,
    resize_start_mouse: Vec2,
    resize_start_size: Vec2,
    definition: Box<dyn PanelDefinition>,
    had_focus_last_frame: bool,
}

impl GenericPanel {
    pub fn new(definition: impl PanelDefinition + 'static, ctx: &WgpuContext) -> Self {
        let title = definition.title();
        let rect = definition.default_rect(ctx);
        let resize_limits = definition.resize_limits();

        Self {
            title,
            rect,
            visible: false,
            in_current_mode: false,
            collapsed: false,
            dragging: false,
            resize_limits,
            drag_offset: Vec2::ZERO,
            resizing: false,
            resize_start_mouse: Vec2::ZERO,
            resize_start_size: Vec2::ZERO,
            definition: Box::new(definition),
            had_focus_last_frame: false,
        }
    }

    pub(crate) fn has_active_escape_interaction(&self) -> bool {
        self.definition.has_active_escape_interaction()
    }

    pub(crate) fn cancel_escape_interaction(&mut self) {
        self.definition.cancel_escape_interaction();
    }

    /// Explicitly defocus this panel, notifying its definition and clearing global focus.
    pub fn defocus(&mut self) {
        if focused_panel() == Some(self.title) {
            self.definition.on_defocus();
            set_focused_panel(None);
        }
    }

    pub fn update_and_draw(&mut self, ctx: &mut WgpuContext, editor: &mut Editor, blocked: bool) {
        let has_focus = focused_panel() == Some(self.title);
        if self.had_focus_last_frame && !has_focus {
            self.definition.on_defocus();
        }
        self.had_focus_last_frame = has_focus;

        if !self.visible {
            return;
        }

        const TITLE_BAR_H: f32 = 28.0;
        const CONTENT_BORDER_WIDTH: f32 = 2.0;
        const RESIZE_HANDLE_SIZE: f32 = 21.0;
        const RESIZE_GRIP_LINE_WIDTH: f32 = 1.5;

        // Process ongoing interactions before the panel rectangle snapshot
        let mouse: Vec2 = ctx.mouse_position().into();
        if let Some(limits) = self.resize_limits {
            let available = (
                (ctx.screen_width() - self.rect.x.max(0.0)).max(0.0),
                (ctx.screen_height() - self.rect.y.max(MENU_PANEL_HEIGHT)).max(0.0),
            );
            let size = clamp_panel_size((self.rect.w, self.rect.h), limits, available);
            self.rect.w = size.0;
            self.rect.h = size.1;
        }
        if self.resizing {
            if ctx.is_mouse_button_down(MouseButton::Left) {
                let size_delta = mouse - self.resize_start_mouse;
                let requested = (
                    self.resize_start_size.x + size_delta.x,
                    self.resize_start_size.y + size_delta.y,
                );
                let available = (
                    (ctx.screen_width() - self.rect.x).max(0.0),
                    (ctx.screen_height() - self.rect.y).max(0.0),
                );
                if let Some(limits) = self.resize_limits {
                    let size = clamp_panel_size(requested, limits, available);
                    self.rect.w = size.0;
                    self.rect.h = size.1;
                }
            } else {
                self.resizing = false;
            }
        }
        if self.dragging {
            if ctx.is_mouse_button_down(MouseButton::Left) {
                let new_pos = mouse - self.drag_offset;
                self.rect.x = new_pos.x;
                self.rect.y = new_pos.y;
            } else {
                self.dragging = false;
            }
        }

        // Clamp the panel within bounds
        let max_x = ctx.screen_width() - self.rect.w;
        if self.rect.x < 0.0 {
            self.rect.x = 0.0;
        } else if self.rect.x > max_x {
            self.rect.x = max_x;
        }

        let min_y = MENU_PANEL_HEIGHT;
        let max_y = if self.resize_limits.is_some() {
            ctx.screen_height() - self.rect.h
        } else {
            ctx.screen_height() - TITLE_BAR_H
        };
        if self.rect.y < min_y {
            self.rect.y = min_y;
        } else if self.rect.y > max_y {
            self.rect.y = max_y;
        }

        // Take snapshot after position updates so all drawing uses current frame's position
        let panel_rect = self.rect;
        let title_bar = Rect::new(panel_rect.x, panel_rect.y, panel_rect.w, TITLE_BAR_H);

        // Title bar background
        let has_focus = focused_panel() == Some(self.title);
        let title_color = with_theme(|t| {
            if has_focus {
                t.panel
            } else {
                Color::new(t.panel.r * 0.7, t.panel.g * 0.7, t.panel.b * 0.7, t.panel.a)
            }
        });
        ctx.draw_rectangle(
            title_bar.x,
            title_bar.y,
            title_bar.w,
            title_bar.h,
            title_color,
        );

        // Collapse button
        let collapse_rect = Rect::new(panel_rect.left() + 5., panel_rect.y + 4., 20., 20.);
        let collapse_clicked = Button::new(collapse_rect, if self.collapsed { "+" } else { "-" })
            .plain()
            .text_color(panel_text_color())
            .suppressed(blocked)
            .show(ctx);
        if !blocked && collapse_clicked {
            self.collapsed = !self.collapsed;
            if self.collapsed {
                self.defocus();
            }
        }

        // Custom title or default title
        let custom_drawn = self.definition.draw_custom_title(ctx, title_bar, blocked);
        if !custom_drawn {
            ctx.draw_text(
                self.title,
                collapse_rect.x + 25.,
                title_bar.y + 20.,
                16.,
                panel_text_color(),
            );
        }

        // Close button
        let close_rect = Rect::new(panel_rect.right() - 26., panel_rect.y + 4., 20., 20.);
        let close_clicked = Button::new(close_rect, "x")
            .plain()
            .text_color(panel_text_color())
            .suppressed(blocked)
            .show(ctx);
        if !blocked && close_clicked {
            self.visible = false;
            self.defocus();
        }

        // Start drag only after all title-bar interactions are processed
        if !blocked
            && !is_context_menu_open()
            && !self.dragging
            && ctx.is_mouse_button_pressed(MouseButton::Left)
            && title_bar.contains(mouse)
            && !widgets::is_click_consumed()
        {
            self.dragging = true;
            self.drag_offset = mouse - vec2(self.rect.x, self.rect.y);
        }

        if self.collapsed {
            return;
        }

        // Content area
        let content_rect = Rect::new(
            panel_rect.x,
            panel_rect.y + TITLE_BAR_H,
            panel_rect.w,
            panel_rect.h - TITLE_BAR_H,
        );

        // Background
        ctx.draw_rectangle(
            content_rect.x,
            content_rect.y,
            content_rect.w,
            content_rect.h,
            with_theme(|t| t.background),
        );
        let resize_handle = self.resize_limits.map(|_| {
            Rect::new(
                panel_rect.right() - RESIZE_HANDLE_SIZE,
                panel_rect.bottom() - RESIZE_HANDLE_SIZE,
                RESIZE_HANDLE_SIZE,
                RESIZE_HANDLE_SIZE,
            )
        });
        if let Some(handle) = resize_handle {
            if !blocked
                && !self.resizing
                && ctx.is_mouse_button_pressed(MouseButton::Left)
                && handle.contains(mouse)
                && !is_click_consumed()
            {
                self.resizing = true;
                self.resize_start_mouse = mouse;
                self.resize_start_size = vec2(panel_rect.w, panel_rect.h);
                consume_click();
            }
        }

        self.definition
            .draw(ctx, content_rect, editor, blocked || self.resizing);

        let border_inset = CONTENT_BORDER_WIDTH / 2.0;
        ctx.draw_rectangle_lines(
            content_rect.x + border_inset,
            content_rect.y + border_inset,
            content_rect.w - CONTENT_BORDER_WIDTH,
            content_rect.h - CONTENT_BORDER_WIDTH,
            CONTENT_BORDER_WIDTH,
            with_theme(|t| t.border),
        );

        if let Some(handle) = resize_handle {
            let grip_color = with_theme(|theme| theme.border);
            ctx.draw_line(
                handle.x + 3.0,
                handle.bottom() - 1.0,
                handle.right() - 1.0,
                handle.y + 3.0,
                RESIZE_GRIP_LINE_WIDTH,
                grip_color,
            );
            ctx.draw_line(
                handle.x + 9.0,
                handle.bottom() - 1.0,
                handle.right() - 1.0,
                handle.y + 9.0,
                RESIZE_GRIP_LINE_WIDTH,
                grip_color,
            );
            ctx.draw_line(
                handle.x + 15.0,
                handle.bottom() - 1.0,
                handle.right() - 1.0,
                handle.y + 15.0,
                RESIZE_GRIP_LINE_WIDTH,
                grip_color,
            );
        }
    }
}

fn clamp_panel_size(
    requested: (f32, f32),
    limits: PanelResizeLimits,
    available: (f32, f32),
) -> (f32, f32) {
    let min_width = limits.min_width.min(available.0);
    let min_height = limits.min_height.min(available.1);
    (
        requested.0.clamp(min_width, available.0),
        requested.1.clamp(min_height, available.1),
    )
}

#[cfg(test)]
mod tests {
    use super::{clamp_panel_size, PanelResizeLimits};

    const LIMITS: PanelResizeLimits = PanelResizeLimits {
        min_width: 180.0,
        min_height: 140.0,
    };

    #[test]
    fn panel_size_clamp_below_minimum_uses_policy() {
        assert_eq!(
            clamp_panel_size((80.0, 50.0), LIMITS, (900.0, 700.0)),
            (180.0, 140.0)
        );
    }

    #[test]
    fn panel_size_clamp_within_viewport_preserves_requested_size() {
        assert_eq!(
            clamp_panel_size((300.0, 250.0), LIMITS, (900.0, 700.0)),
            (300.0, 250.0)
        );
    }

    #[test]
    fn panel_size_clamp_above_viewport_uses_viewport_size() {
        assert_eq!(
            clamp_panel_size((1200.0, 900.0), LIMITS, (900.0, 700.0)),
            (900.0, 700.0)
        );
    }
}
