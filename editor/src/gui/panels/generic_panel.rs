// editor/src/gui/generic_panel.rs
use crate::gui::gui_constants::*;
use crate::Editor;
use bishop::prelude::*;
use engine_core::prelude::*;

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
        }
    }

    pub fn update_and_draw(&mut self, ctx: &mut WgpuContext, editor: &mut Editor, blocked: bool) {
        if !self.visible {
            return;
        }

        const TITLE_BAR_H: f32 = 28.0;

        // Process drag logic first (before snapshot) so drawing uses current position
        let mouse: Vec2 = ctx.mouse_position().into();
        let title_bar_for_drag = Rect::new(self.rect.x, self.rect.y, self.rect.w, TITLE_BAR_H);
        if !blocked
            && ctx.is_mouse_button_pressed(MouseButton::Left)
            && title_bar_for_drag.contains(mouse)
        {
            self.dragging = true;
            self.drag_offset = mouse - vec2(self.rect.x, self.rect.y);
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
        let max_y = ctx.screen_height() - TITLE_BAR_H;
        if self.rect.y < min_y {
            self.rect.y = min_y;
        } else if self.rect.y > max_y {
            self.rect.y = max_y;
        }

        // Take snapshot after position updates so all drawing uses current frame's position
        let panel_rect = self.rect;
        let title_bar = Rect::new(panel_rect.x, panel_rect.y, panel_rect.w, TITLE_BAR_H);

        // Title bar
        ctx.draw_rectangle(
            title_bar.x,
            title_bar.y,
            title_bar.w,
            title_bar.h,
            PANEL_COLOR,
        );

        // Collapse button
        let collapse_rect = Rect::new(panel_rect.left() + 5., panel_rect.y + 4., 20., 20.);
        let collapse_clicked = Button::new(collapse_rect, if self.collapsed { "+" } else { "-" })
            .plain()
            .text_color(Color::BLACK)
            .blocked(blocked)
            .show(ctx);
        if !blocked && collapse_clicked {
            self.collapsed = !self.collapsed;
        }

        // Title
        ctx.draw_text(
            self.title,
            collapse_rect.x + 25.,
            title_bar.y + 20.,
            16.,
            Color::BLACK,
        );

        // Close button
        let close_rect = Rect::new(panel_rect.right() - 26., panel_rect.y + 4., 20., 20.);
        let close_clicked = Button::new(close_rect, "x")
            .plain()
            .text_color(Color::BLACK)
            .blocked(blocked)
            .show(ctx);
        if !blocked && close_clicked {
            self.visible = false;
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
            FIELD_BACKGROUND_COLOR,
        );
        ctx.draw_rectangle_lines(
            content_rect.x,
            content_rect.y,
            content_rect.w,
            content_rect.h,
            2.,
            Color::WHITE,
        );

        if !self.collapsed {
            self.definition.draw(ctx, content_rect, editor, blocked);
        }
    }
}
