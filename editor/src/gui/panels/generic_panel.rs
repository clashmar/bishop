// editor/src/gui/generic_panel.rs
use crate::gui::gui_constants::MENU_PANEL_HEIGHT;
use crate::Editor;
use bishop::prelude::*;
use engine_core::prelude::*;
use widgets::{focused_panel, is_context_menu_open, set_focused_panel};

/// Must be globally unique.
pub type PanelId = &'static str;

/// Defines the features and content of the panel.
pub trait PanelDefinition {
    /// Unique title (also used as id).
    fn title(&self) -> &'static str;
    /// Default rect when first created.
    fn default_rect(&self, ctx: &WgpuContext) -> Rect;
    /// Draws panel contents. When `blocked` is true, the panel should not respond to mouse input.
    fn draw(&mut self, ctx: &mut WgpuContext, rect: Rect, editor: &mut Editor, blocked: bool);
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
    drag_offset: Vec2,
    definition: Box<dyn PanelDefinition>,
    had_focus_last_frame: bool,
}

impl GenericPanel {
    pub fn new(definition: impl PanelDefinition + 'static, ctx: &WgpuContext) -> Self {
        let title = definition.title();
        let rect = definition.default_rect(ctx);

        Self {
            title,
            rect,
            visible: false,
            in_current_mode: false,
            collapsed: false,
            dragging: false,
            drag_offset: Vec2::ZERO,
            definition: Box::new(definition),
            had_focus_last_frame: false,
        }
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

        // Process ongoing drag (before snapshot) so drawing uses current position
        let mouse: Vec2 = ctx.mouse_position().into();
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
        let max_y = ctx.screen_height() - TITLE_BAR_H;
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
            .text_color(with_theme(|t| t.panel_text))
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
                with_theme(|t| t.panel_text),
            );
        }

        // Close button
        let close_rect = Rect::new(panel_rect.right() - 26., panel_rect.y + 4., 20., 20.);
        let close_clicked = Button::new(close_rect, "x")
            .plain()
            .text_color(with_theme(|t| t.panel_text))
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
        ctx.draw_rectangle_lines(
            content_rect.x,
            content_rect.y,
            content_rect.w,
            content_rect.h,
            2.,
            with_theme(|t| t.border),
        );

        self.definition.draw(ctx, content_rect, editor, blocked);
    }
}
