use crate::constants::{colors, layout};
use crate::theme::WidgetThemeMapper;
use crate::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// The visual style of a button.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonStyle {
    /// Standard button with background and border.
    Default,
    /// Minimal button with no background, only shows hover state.
    Plain,
}

/// Click results reported by [`Button::show_clicks`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ButtonClicks {
    pub primary: bool,
    pub secondary: bool,
}

enum ButtonContent<'a> {
    Text(&'a str),
    Icon { texture: &'a Texture2D, id: &'a str },
}

/// A clickable button widget using the builder pattern.
pub struct Button<'a> {
    rect: Rect,
    content: ButtonContent<'a>,
    style: ButtonStyle,
    font_size: f32,
    visuals: WidgetVisuals,
    text_offset: Vec2,
    blocked: bool,
    suppressed: bool,
    focused: bool,
    mouse_position: Option<Vec2>,
    allow_secondary_click: bool,
    interaction_id: Option<ClickTargetId>,
    icon_padding: f32,
}

const BLOCKED_BACKGROUND_COLOR: Color = Color::new(0.08, 0.08, 0.08, 0.9);
const BLOCKED_OUTLINE_COLOR: Color = Color::new(0.45, 0.45, 0.45, 0.7);
const BLOCKED_TEXT_COLOR: Color = Color::new(0.65, 0.65, 0.65, 0.9);
const PLAIN_BLOCKED_OVERLAY: Color = Color::new(0.2, 0.2, 0.2, 0.25);

impl<'a> Button<'a> {
    /// Creates a new button with the given rect and label.
    pub fn new(rect: impl Into<Rect>, label: &'a str) -> Self {
        Self {
            rect: rect.into(),
            content: ButtonContent::Text(label),
            style: ButtonStyle::Default,
            font_size: layout::FIELD_TEXT_SIZE_16,
            visuals: WidgetVisuals::default(),
            text_offset: Vec2::ZERO,
            blocked: false,
            suppressed: false,
            focused: false,
            mouse_position: None,
            allow_secondary_click: false,
            interaction_id: None,
            icon_padding: 2.0,
        }
    }

    /// Creates a new icon button with the given rect and texture.
    ///
    /// The `id` string is used for interaction tracking and is not displayed.
    /// The texture is scaled to fill the button rect minus padding.
    pub fn icon(rect: impl Into<Rect>, texture: &'a Texture2D, id: &'a str) -> Self {
        Self {
            rect: rect.into(),
            content: ButtonContent::Icon { texture, id },
            style: ButtonStyle::Default,
            font_size: layout::FIELD_TEXT_SIZE_16,
            visuals: WidgetVisuals::default(),
            text_offset: Vec2::ZERO,
            blocked: false,
            suppressed: false,
            focused: false,
            mouse_position: None,
            allow_secondary_click: false,
            interaction_id: None,
            icon_padding: 2.0,
        }
    }

    /// Sets the button to use the plain style (no background).
    pub fn plain(mut self) -> Self {
        self.style = ButtonStyle::Plain;
        self
    }

    /// Sets visual overrides for the button.
    pub fn visuals(mut self, visuals: WidgetVisuals) -> Self {
        self.visuals = visuals;
        self
    }

    /// Sets the text color. Delegates to [`visuals`].
    pub fn text_color(mut self, color: impl Into<Color>) -> Self {
        self.visuals.text = Some(color.into());
        self
    }

    /// Sets the hover background color. Delegates to [`visuals`].
    pub fn hover_color(mut self, color: impl Into<Color>) -> Self {
        self.visuals.hover = Some(color.into());
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

    /// Sets whether the button is transiently suppressed from interaction without blocked visuals.
    pub fn suppressed(mut self, suppressed: bool) -> Self {
        self.suppressed = suppressed;
        self
    }

    /// Enables secondary click reporting for [`Button::show_clicks`].
    pub fn allow_secondary_click(mut self) -> Self {
        self.allow_secondary_click = true;
        self
    }

    /// Overrides the interaction id used to match press and release to the same control.
    pub fn interaction_id(mut self, id: WidgetId) -> Self {
        self.interaction_id = Some(ClickTargetId(id.0 as u64));
        self
    }

    /// Sets whether the button is visually focused (shows hover highlight without mouse).
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Sets the padding between the button border and the icon texture. Only applies to icon buttons. Default is 2.0.
    pub fn icon_padding(mut self, padding: f32) -> Self {
        self.icon_padding = padding;
        self
    }

    /// Overrides the mouse position used for hover detection (e.g. world-space coords when a camera is active).
    pub fn mouse_position(mut self, pos: Vec2) -> Self {
        self.mouse_position = Some(pos);
        self
    }

    /// Draws the button and returns true if clicked.
    pub fn show<C: BishopContext>(self, ctx: &mut C) -> bool {
        let theme_vs = with_theme(Self::theme_visuals);
        self.show_clicks(ctx, theme_vs).primary
    }

    pub fn show_native_dialog<C: BishopContext>(self, ctx: &mut C) -> bool {
        let interaction_id = self
            .interaction_id
            .unwrap_or_else(|| self.default_interaction_id());
        let theme_vs = with_theme(Self::theme_visuals);
        let clicks = self.show_clicks(ctx, theme_vs);

        if clicks.primary {
            queue_deferred_click_target(interaction_id);
            return false;
        }

        let ready = !ctx.is_mouse_button_down(MouseButton::Left)
            && !ctx.is_mouse_button_pressed(MouseButton::Left)
            && !ctx.is_mouse_button_released(MouseButton::Left);
        take_deferred_click_target(interaction_id, ready)
    }

    /// Draws the button and returns primary and secondary click results.
    pub fn show_clicks<C: BishopContext>(
        self,
        ctx: &mut C,
        theme_vs: WidgetVisuals,
    ) -> ButtonClicks {
        let interaction_id = self
            .interaction_id
            .unwrap_or_else(|| self.default_interaction_id());
        let mouse = self
            .mouse_position
            .unwrap_or_else(|| ctx.mouse_position().into());
        let hovered = self.rect.contains(mouse);
        let primary_held = hovered && ctx.is_mouse_button_down(MouseButton::Left);
        let secondary_held =
            self.allow_secondary_click && hovered && ctx.is_mouse_button_down(MouseButton::Right);
        let visually_blocked = self.blocked;
        let interactive_blocked = self.blocked || self.suppressed;

        let highlight = (hovered || self.focused)
            && !is_dropdown_open()
            && !is_context_menu_open()
            && !interactive_blocked
            && !primary_held
            && !secondary_held;

        match self.style {
            ButtonStyle::Default => {
                let background = if visually_blocked {
                    resolve_with_theme(
                        self.visuals.surface,
                        theme_vs.surface,
                        BLOCKED_BACKGROUND_COLOR,
                    )
                } else if highlight {
                    resolve_with_theme(
                        self.visuals.hover,
                        theme_vs.hover,
                        colors::DEFAULT_HOVER_COLOR,
                    )
                } else {
                    resolve_with_theme(
                        self.visuals.background,
                        theme_vs.background,
                        colors::DEFAULT_BACKGROUND_COLOR,
                    )
                };
                let outline_color = if visually_blocked {
                    BLOCKED_OUTLINE_COLOR
                } else {
                    resolve_with_theme(
                        self.visuals.border,
                        theme_vs.border,
                        colors::DEFAULT_BORDER_COLOR,
                    )
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
                    outline_color,
                );
            }
            ButtonStyle::Plain => {
                if visually_blocked {
                    ctx.draw_rectangle(
                        self.rect.x,
                        self.rect.y,
                        self.rect.w,
                        self.rect.h,
                        resolve_with_theme(
                            self.visuals.surface,
                            theme_vs.surface,
                            PLAIN_BLOCKED_OVERLAY,
                        ),
                    );
                } else if highlight {
                    ctx.draw_rectangle(
                        self.rect.x,
                        self.rect.y,
                        self.rect.w,
                        self.rect.h,
                        resolve_with_theme(
                            self.visuals.hover,
                            theme_vs.hover,
                            colors::DEFAULT_HOVER_COLOR,
                        ),
                    );
                }
            }
        }

        match &self.content {
            ButtonContent::Text(label) => {
                let text_color = if visually_blocked {
                    resolve_with_theme(
                        self.visuals.text_muted,
                        theme_vs.text_muted,
                        BLOCKED_TEXT_COLOR,
                    )
                } else {
                    resolve_with_theme(self.visuals.text, theme_vs.text, colors::DEFAULT_TEXT_COLOR)
                };
                let txt_dims = measure_text_ui(ctx, label, self.font_size);
                let txt_y = self.rect.y + (self.rect.h - txt_dims.height) / 2.0 + txt_dims.offset_y;
                let txt_x = self.rect.x + (self.rect.w - txt_dims.width) / 2.0;
                draw_text_ui(
                    ctx,
                    label,
                    txt_x + self.text_offset.x,
                    txt_y + self.text_offset.y,
                    self.font_size,
                    text_color,
                );
            }
            ButtonContent::Icon { texture, .. } => {
                let icon_color = if visually_blocked {
                    resolve_with_theme(
                        self.visuals.text_muted,
                        theme_vs.text_muted,
                        BLOCKED_TEXT_COLOR,
                    )
                } else {
                    resolve_with_theme(self.visuals.text, theme_vs.text, colors::DEFAULT_TEXT_COLOR)
                };
                let p = self.icon_padding;
                ctx.draw_texture_ex(
                    texture,
                    self.rect.x + p,
                    self.rect.y + p,
                    icon_color,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(self.rect.w - 2.0 * p, self.rect.h - 2.0 * p)),
                        ..Default::default()
                    },
                );
            }
        }

        let interactive = !interactive_blocked && !is_dropdown_open() && !is_context_menu_open();
        let primary = activate_on_release(
            MouseButton::Left,
            interaction_id,
            hovered,
            interactive,
            ctx.is_mouse_button_pressed(MouseButton::Left),
            ctx.is_mouse_button_released(MouseButton::Left),
        );
        let secondary = self.allow_secondary_click
            && activate_on_release(
                MouseButton::Right,
                interaction_id,
                hovered,
                interactive,
                ctx.is_mouse_button_pressed(MouseButton::Right),
                ctx.is_mouse_button_released(MouseButton::Right),
            );

        ButtonClicks { primary, secondary }
    }

    fn default_interaction_id(&self) -> ClickTargetId {
        let mut hasher = DefaultHasher::new();
        match &self.content {
            ButtonContent::Text(label) => label.hash(&mut hasher),
            ButtonContent::Icon { id, .. } => id.hash(&mut hasher),
        }
        self.rect.x.to_bits().hash(&mut hasher);
        self.rect.y.to_bits().hash(&mut hasher);
        self.rect.w.to_bits().hash(&mut hasher);
        self.rect.h.to_bits().hash(&mut hasher);
        self.style.hash(&mut hasher);
        ClickTargetId(hasher.finish())
    }
}

impl WidgetThemeMapper for Button<'_> {
    fn theme_visuals(theme: &Theme) -> WidgetVisuals {
        WidgetVisuals {
            background: Some(theme.background),
            text: Some(theme.text),
            text_muted: Some(theme.text_muted),
            border: Some(theme.border),
            hover: Some(theme.hover),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests;
