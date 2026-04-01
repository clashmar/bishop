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
    text_color: Color,
    hover_color: Color,
    text_offset: Vec2,
    blocked: bool,
    focused: bool,
    mouse_position: Option<Vec2>,
    allow_secondary_click: bool,
    interaction_id: Option<ClickTargetId>,
    icon_padding: f32,
}

impl<'a> Button<'a> {
    /// Creates a new button with the given rect and label.
    pub fn new(rect: impl Into<Rect>, label: &'a str) -> Self {
        Self {
            rect: rect.into(),
            content: ButtonContent::Text(label),
            style: ButtonStyle::Default,
            font_size: FIELD_TEXT_SIZE_16,
            text_color: FIELD_TEXT_COLOR,
            hover_color: HOVER_COLOR,
            text_offset: Vec2::ZERO,
            blocked: false,
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
            font_size: FIELD_TEXT_SIZE_16,
            text_color: FIELD_TEXT_COLOR,
            hover_color: HOVER_COLOR,
            text_offset: Vec2::ZERO,
            blocked: false,
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
        self.show_clicks(ctx).primary
    }

    /// Draws the button and returns primary and secondary click results.
    pub fn show_clicks<C: BishopContext>(self, ctx: &mut C) -> ButtonClicks {
        let interaction_id = self.interaction_id.unwrap_or_else(|| self.default_interaction_id());
        let mouse = self
            .mouse_position
            .unwrap_or_else(|| ctx.mouse_position().into());
        let hovered = self.rect.contains(mouse);
        let primary_held = hovered && ctx.is_mouse_button_down(MouseButton::Left);
        let secondary_held =
            self.allow_secondary_click && hovered && ctx.is_mouse_button_down(MouseButton::Right);

        match self.style {
            ButtonStyle::Default => {
                let highlight = (hovered || self.focused)
                    && !is_dropdown_open()
                    && !self.blocked
                    && !primary_held
                    && !secondary_held;
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
                    && !primary_held
                    && !secondary_held;
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

        match &self.content {
            ButtonContent::Text(label) => {
                let txt_dims = measure_text_ui(ctx, label, self.font_size);
                let txt_y =
                    self.rect.y + (self.rect.h - txt_dims.height) / 2.0 + txt_dims.offset_y;
                let txt_x = self.rect.x + (self.rect.w - txt_dims.width) / 2.0;
                draw_text_ui(
                    ctx,
                    label,
                    txt_x + self.text_offset.x,
                    txt_y + self.text_offset.y,
                    self.font_size,
                    self.text_color,
                );
            }
            ButtonContent::Icon { texture, .. } => {
                let p = self.icon_padding;
                ctx.draw_texture_ex(
                    texture,
                    self.rect.x + p,
                    self.rect.y + p,
                    self.text_color,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(self.rect.w - 2.0 * p, self.rect.h - 2.0 * p)),
                        ..Default::default()
                    },
                );
            }
        }

        let interactive = !self.blocked && !is_dropdown_open();
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

#[cfg(test)]
mod tests {
    use super::*;
    use bishop::material::BishopRenderTarget;
    use bishop::prelude::*;

    struct TestContext {
        mouse_pos: (f32, f32),
        left_pressed: bool,
        left_down: bool,
        left_released: bool,
        right_pressed: bool,
        right_down: bool,
        right_released: bool,
    }

    impl TestContext {
        fn new() -> Self {
            Self {
                mouse_pos: (0.0, 0.0),
                left_pressed: false,
                left_down: false,
                left_released: false,
                right_pressed: false,
                right_down: false,
                right_released: false,
            }
        }
    }

    impl Input for TestContext {
        fn is_key_down(&self, _key: KeyCode) -> bool {
            false
        }

        fn is_key_pressed(&self, _key: KeyCode) -> bool {
            false
        }

        fn is_key_released(&self, _key: KeyCode) -> bool {
            false
        }

        fn any_key_pressed(&self) -> bool {
            false
        }

        fn is_mouse_button_down(&self, button: MouseButton) -> bool {
            match button {
                MouseButton::Left => self.left_down,
                MouseButton::Right => self.right_down,
                _ => false,
            }
        }

        fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
            match button {
                MouseButton::Left => self.left_pressed,
                MouseButton::Right => self.right_pressed,
                _ => false,
            }
        }

        fn is_mouse_button_released(&self, button: MouseButton) -> bool {
            match button {
                MouseButton::Left => self.left_released,
                MouseButton::Right => self.right_released,
                _ => false,
            }
        }

        fn mouse_position(&self) -> (f32, f32) {
            self.mouse_pos
        }

        fn mouse_delta_position(&self) -> (f32, f32) {
            (0.0, 0.0)
        }

        fn mouse_wheel(&self) -> (f32, f32) {
            (0.0, 0.0)
        }

        fn chars_pressed(&self) -> Vec<char> {
            Vec::new()
        }

        fn get_time(&self) -> f64 {
            0.0
        }
    }

    impl Draw for TestContext {
        fn draw_rectangle(&mut self, _x: f32, _y: f32, _w: f32, _h: f32, _color: Color) {}

        fn draw_rectangle_lines(
            &mut self,
            _x: f32,
            _y: f32,
            _w: f32,
            _h: f32,
            _thickness: f32,
            _color: Color,
        ) {
        }

        fn draw_line(
            &mut self,
            _x1: f32,
            _y1: f32,
            _x2: f32,
            _y2: f32,
            _thickness: f32,
            _color: Color,
        ) {
        }

        fn draw_circle(&mut self, _x: f32, _y: f32, _radius: f32, _color: Color) {}

        fn draw_circle_lines(
            &mut self,
            _x: f32,
            _y: f32,
            _radius: f32,
            _thickness: f32,
            _color: Color,
        ) {
        }

        fn draw_triangle(&mut self, _v1: Vec2, _v2: Vec2, _v3: Vec2, _color: Color) {}

        fn clear_background(&mut self, _color: Color) {}

        fn draw_texture(&mut self, _texture: &Texture2D, _x: f32, _y: f32, _color: Color) {}

        fn draw_texture_ex(
            &mut self,
            _texture: &Texture2D,
            _x: f32,
            _y: f32,
            _color: Color,
            _params: DrawTextureParams,
        ) {
        }

        fn push_clip_rect(&mut self, _rect: Rect) {}

        fn pop_clip_rect(&mut self) {}
    }

    impl Text for TestContext {
        fn draw_text(
            &mut self,
            text: &str,
            x: f32,
            y: f32,
            font_size: f32,
            color: Color,
        ) -> TextDimensions {
            self.draw_text_ex(
                text,
                x,
                y,
                TextParams {
                    font_size: font_size as u16,
                    color,
                    ..TextParams::default()
                },
            )
        }

        fn draw_text_ex(
            &mut self,
            text: &str,
            _x: f32,
            _y: f32,
            params: TextParams,
        ) -> TextDimensions {
            self.measure_text(text, params.font_size as f32)
        }

        fn measure_text(&self, text: &str, font_size: f32) -> TextDimensions {
            TextDimensions {
                width: text.len() as f32 * font_size * 0.5,
                height: font_size,
                offset_y: 0.0,
            }
        }
    }

    impl Camera for TestContext {
        fn set_camera(&mut self, _camera: &Camera2D) {}

        fn set_default_camera(&mut self) {}

        fn screen_to_world(&self, _camera: &Camera2D, screen_pos: Vec2) -> Vec2 {
            screen_pos
        }

        fn create_render_target(&self, _width: u32, _height: u32) -> BishopRenderTarget {
            panic!("render targets are not used in button widget tests")
        }
    }

    impl Window for TestContext {
        fn screen_width(&self) -> f32 {
            320.0
        }

        fn screen_height(&self) -> f32 {
            240.0
        }

        fn set_cursor_icon(&mut self, _icon: CursorIcon) {}

        fn toggle_fullscreen(&mut self) -> bool {
            false
        }

        fn is_fullscreen(&self) -> bool {
            false
        }

        fn scale_factor(&self) -> f32 {
            1.0
        }
    }

    impl Time for TestContext {
        fn get_frame_time(&self) -> f32 {
            1.0 / 60.0
        }

        fn get_frame_spike_ms(&self) -> f32 {
            0.0
        }

        fn update(&mut self) {}
    }

    impl RenderOps for TestContext {
        fn begin_render_to_target(&mut self, _rt: &BishopRenderTarget) {}

        fn end_render_to_target(&mut self) {}

        fn draw_render_target(
            &mut self,
            _rt: &BishopRenderTarget,
            _x: f32,
            _y: f32,
            _w: f32,
            _h: f32,
        ) {
        }

        fn create_drawable_render_target(&self, _width: u32, _height: u32) -> BishopRenderTarget {
            panic!("render targets are not used in button widget tests")
        }
    }

    impl TextureLoader for TestContext {
        fn load_texture_from_bytes(&self, _data: &[u8]) -> Result<Texture2D, String> {
            panic!("textures are not used in button widget tests")
        }

        fn load_texture_from_path(&self, _path: &str) -> Result<Texture2D, String> {
            panic!("textures are not used in button widget tests")
        }

        fn empty_texture(&self) -> Texture2D {
            panic!("textures are not used in button widget tests")
        }
    }

    #[test]
    fn primary_click_requires_matching_press_and_release() {
        reset_click_consumed();

        let button = Rect::new(0.0, 0.0, 80.0, 30.0);
        let mut ctx = TestContext::new();
        ctx.mouse_pos = (40.0, 20.0);
        ctx.left_pressed = true;
        ctx.left_down = true;

        assert!(!Button::new(button, "Play").show(&mut ctx));

        reset_click_consumed();
        ctx.left_pressed = false;
        ctx.left_down = false;
        ctx.left_released = true;

        assert!(Button::new(button, "Play").show(&mut ctx));
    }

    #[test]
    fn primary_click_does_not_activate_from_another_controls_press() {
        reset_click_consumed();

        let button = Rect::new(0.0, 0.0, 80.0, 30.0);
        let mut ctx = TestContext::new();
        ctx.mouse_pos = (120.0, 20.0);
        ctx.left_pressed = true;
        ctx.left_down = true;

        assert!(!Button::new(button, "Play").show(&mut ctx));

        reset_click_consumed();
        ctx.left_pressed = false;
        ctx.left_down = false;
        ctx.left_released = true;
        ctx.mouse_pos = (40.0, 20.0);

        assert!(!Button::new(button, "Play").show(&mut ctx));
    }

    #[test]
    fn secondary_clicks_are_reported_when_opted_in() {
        reset_click_consumed();

        let button = Rect::new(0.0, 0.0, 80.0, 30.0);
        let mut ctx = TestContext::new();
        ctx.mouse_pos = (40.0, 20.0);
        ctx.right_pressed = true;
        ctx.right_down = true;

        let clicks = Button::new(button, "Play")
            .allow_secondary_click()
            .show_clicks(&mut ctx);

        assert!(!clicks.primary);
        assert!(!clicks.secondary);

        reset_click_consumed();
        ctx.right_pressed = false;
        ctx.right_down = false;
        ctx.right_released = true;

        let clicks = Button::new(button, "Play")
            .allow_secondary_click()
            .show_clicks(&mut ctx);

        assert!(!clicks.primary);
        assert!(clicks.secondary);
    }
}
