use crate::*;

/// The visual style of a button.
#[derive(Clone, Copy, PartialEq)]
pub enum ButtonStyle {
    /// Standard button with background and border.
    Default,
    /// Minimal button with no background, only shows hover state.
    Plain,
}

/// A clickable button widget using the builder pattern.
pub struct Button<'a> {
    rect: Rect,
    label: &'a str,
    style: ButtonStyle,
    font_size: f32,
    text_color: Color,
    hover_color: Color,
    text_offset: Vec2,
    blocked: bool,
    focused: bool,
    mouse_position: Option<Vec2>,
}

impl<'a> Button<'a> {
    /// Creates a new button with the given rect and label.
    pub fn new(rect: impl Into<Rect>, label: &'a str) -> Self {
        Self {
            rect: rect.into(),
            label,
            style: ButtonStyle::Default,
            font_size: FIELD_TEXT_SIZE_16,
            text_color: FIELD_TEXT_COLOR,
            hover_color: HOVER_COLOR,
            text_offset: Vec2::ZERO,
            blocked: false,
            focused: false,
            mouse_position: None,
        }
    }

    /// Sets the button to use the plain style (no background).
    pub fn plain(mut self) -> Self {
        self.style = ButtonStyle::Plain;
        self.hover_color = HOVER_COLOR_PLAIN;
        self
    }

    /// Sets the text color.
    pub fn text_color(mut self, color: impl Into<Color>) -> Self {
        self.text_color = color.into();
        self
    }

    /// Sets the hover background color.
    pub fn hover_color(mut self, color: impl Into<Color>) -> Self {
        self.hover_color = color.into();
        self
    }

    /// Sets an offset for the text position.
    pub fn text_offset(mut self, offset: impl Into<Vec2>) -> Self {
        self.text_offset = offset.into();
        self
    }

    /// Sets the font size for the button label.
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Sets whether the button is blocked from interaction.
    pub fn blocked(mut self, blocked: bool) -> Self {
        self.blocked = blocked;
        self
    }

    /// Sets whether the button is visually focused (shows hover highlight without mouse).
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Overrides the mouse position used for hover detection (e.g. world-space coords when a camera is active).
    pub fn mouse_position(mut self, pos: Vec2) -> Self {
        self.mouse_position = Some(pos);
        self
    }

    /// Draws the button and returns true if clicked.
    pub fn show<C: BishopContext>(self, ctx: &mut C) -> bool {
        let mouse = self
            .mouse_position
            .unwrap_or_else(|| ctx.mouse_position().into());
        let hovered = self.rect.contains(mouse);

        let txt_dims = measure_text_ui(ctx, self.label, self.font_size);
        let txt_y = self.rect.y + (self.rect.h - txt_dims.height) / 2.0 + txt_dims.offset_y;
        let txt_x = self.rect.x + (self.rect.w - txt_dims.width) / 2.;

        match self.style {
            ButtonStyle::Default => {
                let highlight = (hovered || self.focused)
                    && !is_dropdown_open()
                    && !self.blocked
                    && !(hovered && ctx.is_mouse_button_down(MouseButton::Left));
                let background = if highlight {
                    self.hover_color
                } else {
                    FIELD_BACKGROUND_COLOR
                };
                ctx.draw_rectangle(
                    self.rect.x,
                    self.rect.y,
                    self.rect.w,
                    self.rect.h,
                    background,
                );
                ctx.draw_rectangle_lines(
                    self.rect.x,
                    self.rect.y,
                    self.rect.w,
                    self.rect.h,
                    2.,
                    OUTLINE_COLOR,
                );
            }
            ButtonStyle::Plain => {
                let highlight = (hovered || self.focused)
                    && !is_dropdown_open()
                    && !self.blocked
                    && !(hovered && ctx.is_mouse_button_down(MouseButton::Left));
                if highlight {
                    ctx.draw_rectangle(
                        self.rect.x,
                        self.rect.y,
                        self.rect.w,
                        self.rect.h,
                        self.hover_color,
                    );
                }
            }
        }

        draw_text_ui(
            ctx,
            self.label,
            txt_x + self.text_offset.x,
            txt_y + self.text_offset.y,
            self.font_size,
            self.text_color,
        );

        let clicked = ctx.is_mouse_button_pressed(MouseButton::Left)
            && hovered
            && !self.blocked
            && !is_dropdown_open()
            && !is_click_consumed();

        if clicked {
            consume_click();
        }

        clicked
    }
}
