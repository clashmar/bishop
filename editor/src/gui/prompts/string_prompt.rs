// editor/src/gui/prompts/string_prompt.rs
use crate::app::escape::modal_escape_requested;
use crate::gui::prompts::constants::*;
use crate::gui::prompts::helpers::*;
use bishop::prelude::*;
use engine_core::prelude::*;
use widgets::{input_is_focused, request_focus};

/// Result of a string prompt.
#[derive(Debug, PartialEq, Eq)]
pub enum StringPromptResult {
    Confirmed(String),
    Cancelled,
}

/// A prompt that draws:
///   * Message line,
///   * Text field,
///   * Confirm / Cancel buttons.
pub struct StringPrompt {
    /// Unique id for the text field.
    input_id: WidgetId,
    /// Rectangle that contains the whole widget.
    rect: Rect,
    /// Message shown above the text field.
    message: String,
    /// Current contents of the text field.
    current: String,
    /// Whether the current text should be selected when the prompt opens.
    select_all_on_open: bool,
}

impl StringPrompt {
    /// Create a new prompt centred inside the supplied rect.
    pub fn new(modal_rect: Rect, message: impl Into<String>) -> Self {
        let total_h = PROMPT_TOP_PADDING
            + DEFAULT_FONT_SIZE_16
            + PROMPT_TEXT_GAP
            + FIELD_H
            + PROMPT_SECTION_GAP
            + BUTTON_H
            + PROMPT_BOTTOM_PADDING;
        let rect = prompt_content_rect(modal_rect, total_h);

        Self {
            input_id: WidgetId::default(),
            rect,
            message: message.into(),
            current: String::new(),
            select_all_on_open: false,
        }
    }

    /// Sets the initial text shown in the prompt input.
    pub fn with_initial_value(mut self, value: impl Into<String>) -> Self {
        self.current = value.into();
        self
    }

    /// Selects the initial text when the prompt first opens.
    pub fn select_all_on_open(mut self) -> Self {
        self.select_all_on_open = true;
        self
    }

    /// Draws the widget and, return the result if confirmed/cancelled or None.
    pub fn draw(&mut self, ctx: &mut WgpuContext) -> Option<StringPromptResult> {
        self.draw_with_ctx(ctx, Controls::enter(ctx), modal_escape_requested())
    }

    fn draw_with_ctx<C: BishopContext>(
        &mut self,
        ctx: &mut C,
        enter_pressed: bool,
        escape_pressed: bool,
    ) -> Option<StringPromptResult> {
        draw_prompt_label(
            ctx,
            &self.message,
            self.rect.x,
            self.rect.y + PROMPT_TOP_PADDING,
        );

        let field_rect = Rect::new(
            self.rect.x,
            self.rect.y + PROMPT_TOP_PADDING + DEFAULT_FONT_SIZE_16 + PROMPT_TEXT_GAP,
            self.rect.w,
            FIELD_H,
        );

        let mut input = TextInput::new(self.input_id, field_rect, &self.current)
            .focused(true)
            .live();
        if self.select_all_on_open {
            input = input.select_all_on_focus();
        }
        let (new_text, _) = input.show(ctx);
        self.current = new_text;

        let btn_y = field_rect.y + field_rect.h + PROMPT_SECTION_GAP;
        let (confirm_rect, cancel_rect) = confirm_cancel_rects(self.rect, btn_y);
        let confirm_clicked = Button::new(confirm_rect, "Confirm").show(ctx);
        let cancel_clicked = Button::new(cancel_rect, "Cancel").show(ctx);

        // Handle result
        if (confirm_clicked || enter_pressed) && !self.current.trim().is_empty() {
            return Some(StringPromptResult::Confirmed(self.current.clone()));
        }

        if cancel_clicked || escape_pressed {
            return Some(StringPromptResult::Cancelled);
        }

        if !input_is_focused() {
            request_focus(self.input_id, true);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::prompts::helpers::confirm_cancel_rects;
    use bishop::material::BishopRenderTarget;
    use widgets::{clear_click_target, reset_click_consumed};

    struct TestContext {
        mouse_pos: (f32, f32),
        left_pressed: bool,
        left_down: bool,
        left_released: bool,
        chars: Vec<char>,
    }

    impl TestContext {
        fn new() -> Self {
            Self {
                mouse_pos: (0.0, 0.0),
                left_pressed: false,
                left_down: false,
                left_released: false,
                chars: Vec::new(),
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
            matches!(button, MouseButton::Left) && self.left_down
        }

        fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
            matches!(button, MouseButton::Left) && self.left_pressed
        }

        fn is_mouse_button_released(&self, button: MouseButton) -> bool {
            matches!(button, MouseButton::Left) && self.left_released
        }

        fn is_mouse_button_double_clicked(&self, _button: MouseButton) -> bool {
            false
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
            self.chars.clone()
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
            panic!("render targets are not used in string prompt tests")
        }
    }

    impl Window for TestContext {
        fn screen_width(&self) -> f32 {
            800.0
        }

        fn screen_height(&self) -> f32 {
            600.0
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
            panic!("render targets are not used in string prompt tests")
        }
    }

    impl TextureLoader for TestContext {
        fn load_texture_from_bytes(&self, _data: &[u8]) -> Result<Texture2D, String> {
            panic!("textures are not used in string prompt tests")
        }

        fn load_texture_from_path(&self, _path: &str) -> Result<Texture2D, String> {
            panic!("textures are not used in string prompt tests")
        }

        fn empty_texture(&self) -> Texture2D {
            panic!("textures are not used in string prompt tests")
        }
    }

    fn reset_widget_state() {
        reset_click_consumed();
        clear_click_target(MouseButton::Left);
        text_input_reset(WidgetId::default());
    }

    #[test]
    fn clicking_confirm_submits_the_typed_value() {
        reset_widget_state();

        let modal_rect = Rect::new(100.0, 60.0, 400.0, 180.0);
        let mut prompt = StringPrompt::new(modal_rect, "Enter prefab name:");
        let (confirm_rect, _) = {
            let field_rect = Rect::new(
                prompt.rect.x,
                prompt.rect.y + PROMPT_TOP_PADDING + DEFAULT_FONT_SIZE_16 + PROMPT_TEXT_GAP,
                prompt.rect.w,
                FIELD_H,
            );
            let btn_y = field_rect.y + field_rect.h + PROMPT_SECTION_GAP;
            confirm_cancel_rects(prompt.rect, btn_y)
        };

        let mut ctx = TestContext::new();
        ctx.chars = vec!['C', 'r', 'a', 't', 'e'];
        assert!(prompt.draw_with_ctx(&mut ctx, false, false).is_none());

        reset_click_consumed();
        ctx.chars.clear();
        ctx.mouse_pos = (
            confirm_rect.x + confirm_rect.w / 2.0,
            confirm_rect.y + confirm_rect.h / 2.0,
        );
        ctx.left_pressed = true;
        ctx.left_down = true;
        assert!(prompt.draw_with_ctx(&mut ctx, false, false).is_none());

        reset_click_consumed();
        ctx.left_pressed = false;
        ctx.left_down = false;
        ctx.left_released = true;
        assert_eq!(
            prompt.draw_with_ctx(&mut ctx, false, false),
            Some(StringPromptResult::Confirmed("Crate".to_string()))
        );
    }

    #[test]
    fn confirming_prefilled_prompt_without_typing_returns_initial_value() {
        reset_widget_state();

        let modal_rect = Rect::new(100.0, 60.0, 400.0, 180.0);
        let mut prompt =
            StringPrompt::new(modal_rect, "Rename room:").with_initial_value("Entry Hall");

        let (confirm_rect, _) = {
            let field_rect = Rect::new(
                prompt.rect.x,
                prompt.rect.y + PROMPT_TOP_PADDING + DEFAULT_FONT_SIZE_16 + PROMPT_TEXT_GAP,
                prompt.rect.w,
                FIELD_H,
            );
            let btn_y = field_rect.y + field_rect.h + PROMPT_SECTION_GAP;
            confirm_cancel_rects(prompt.rect, btn_y)
        };

        let mut ctx = TestContext::new();
        assert!(prompt.draw_with_ctx(&mut ctx, false, false).is_none());

        reset_click_consumed();
        ctx.mouse_pos = (
            confirm_rect.x + confirm_rect.w / 2.0,
            confirm_rect.y + confirm_rect.h / 2.0,
        );
        ctx.left_pressed = true;
        ctx.left_down = true;
        assert!(prompt.draw_with_ctx(&mut ctx, false, false).is_none());

        reset_click_consumed();
        ctx.left_pressed = false;
        ctx.left_down = false;
        ctx.left_released = true;
        assert_eq!(
            prompt.draw_with_ctx(&mut ctx, false, false),
            Some(StringPromptResult::Confirmed("Entry Hall".to_string()))
        );
    }

    #[test]
    fn select_all_on_open_replaces_prefilled_value_when_typing() {
        reset_widget_state();

        let modal_rect = Rect::new(100.0, 60.0, 400.0, 180.0);
        let mut prompt = StringPrompt::new(modal_rect, "Rename prefab:")
            .with_initial_value("Crate")
            .select_all_on_open();

        let (confirm_rect, _) = {
            let field_rect = Rect::new(
                prompt.rect.x,
                prompt.rect.y + PROMPT_TOP_PADDING + DEFAULT_FONT_SIZE_16 + PROMPT_TEXT_GAP,
                prompt.rect.w,
                FIELD_H,
            );
            let btn_y = field_rect.y + field_rect.h + PROMPT_SECTION_GAP;
            confirm_cancel_rects(prompt.rect, btn_y)
        };

        let mut ctx = TestContext::new();
        ctx.chars = vec!['N'];
        assert!(prompt.draw_with_ctx(&mut ctx, false, false).is_none());

        reset_click_consumed();
        ctx.chars.clear();
        ctx.mouse_pos = (
            confirm_rect.x + confirm_rect.w / 2.0,
            confirm_rect.y + confirm_rect.h / 2.0,
        );
        ctx.left_pressed = true;
        ctx.left_down = true;
        assert!(prompt.draw_with_ctx(&mut ctx, false, false).is_none());

        reset_click_consumed();
        ctx.left_pressed = false;
        ctx.left_down = false;
        ctx.left_released = true;
        assert_eq!(
            prompt.draw_with_ctx(&mut ctx, false, false),
            Some(StringPromptResult::Confirmed("N".to_string()))
        );
    }
}
