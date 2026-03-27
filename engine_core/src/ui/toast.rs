// engine_core/src/ui/toast.rs
use crate::ui::widgets::*;
use crate::ui::text::*;
use std::time::Instant;

const PADDING: f32 = 20.0;

/// A simple toast that disappears after a short delay.
pub struct Toast {
    /// Text that will be shown.
    pub msg: String,
    /// When the toast was created.
    start: Instant,
    /// How long the toast stays visible (seconds).
    pub duration: f32,
    /// Whether the toast is currently visible.
    pub active: bool,
}

impl Toast {
    /// Create a new toast that lives for `duration` seconds.
    pub fn new<S: Into<String>>(msg: S, duration: f32) -> Self {
        Self {
            msg: msg.into(),
            start: Instant::now(),
            duration,
            active: true,
        }
    }

    /// Call each frame. Draws the toast if it is still alive.
    pub fn update<C: BishopContext>(&mut self, ctx: &mut C) {
        if !self.active {
            return;
        }
        // Hide after the elapsed time.
        if self.start.elapsed().as_secs_f32() >= self.duration {
            self.active = false;
            return;
        }

        let txt = measure_text(ctx, &self.msg, DEFAULT_FONT_SIZE_16);

        // Bottom left
        let bg_rect = Rect::new(
            PADDING,
            ctx.screen_height() - PADDING - (txt.height + PADDING),
            txt.width + PADDING * 2.0,
            txt.height + PADDING,
        );

        // Background
        ctx.draw_rectangle(
            bg_rect.x,
            bg_rect.y,
            bg_rect.w,
            bg_rect.h,
            Color::new(0.0, 0.0, 0.0, 0.7),
        );

        // Text
        draw_text_ui(
            ctx,
            &self.msg,
            bg_rect.x + PADDING,
            bg_rect.y + (bg_rect.h - txt.height) / 2.0 + txt.offset_y,
            DEFAULT_FONT_SIZE_16,
            Color::WHITE,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bishop::prelude::*;

    struct DrawTextCall {
        text: String,
        x: f32,
        y: f32,
        font_size: f32,
        color: Color,
    }

    struct TestContext {
        screen_height: f32,
        text_dims: TextDimensions,
        draw_text_calls: Vec<DrawTextCall>,
        rect_calls: Vec<Rect>,
    }

    impl TestContext {
        fn new(screen_height: f32, text_dims: TextDimensions) -> Self {
            Self {
                screen_height,
                text_dims,
                draw_text_calls: Vec::new(),
                rect_calls: Vec::new(),
            }
        }
    }

    impl Input for TestContext {
        fn is_key_down(&self, _key: KeyCode) -> bool { false }
        fn is_key_pressed(&self, _key: KeyCode) -> bool { false }
        fn is_key_released(&self, _key: KeyCode) -> bool { false }
        fn any_key_pressed(&self) -> bool { false }
        fn is_mouse_button_down(&self, _button: MouseButton) -> bool { false }
        fn is_mouse_button_pressed(&self, _button: MouseButton) -> bool { false }
        fn is_mouse_button_released(&self, _button: MouseButton) -> bool { false }
        fn mouse_position(&self) -> (f32, f32) { (0.0, 0.0) }
        fn mouse_delta_position(&self) -> (f32, f32) { (0.0, 0.0) }
        fn mouse_wheel(&self) -> (f32, f32) { (0.0, 0.0) }
        fn chars_pressed(&self) -> Vec<char> { Vec::new() }
        fn get_time(&self) -> f64 { 0.0 }
    }

    impl Draw for TestContext {
        fn draw_rectangle(&mut self, x: f32, y: f32, w: f32, h: f32, _color: Color) {
            self.rect_calls.push(Rect::new(x, y, w, h));
        }

        fn draw_rectangle_lines(&mut self, _x: f32, _y: f32, _w: f32, _h: f32, _thickness: f32, _color: Color) {}

        fn draw_line(&mut self, _x1: f32, _y1: f32, _x2: f32, _y2: f32, _thickness: f32, _color: Color) {}

        fn draw_circle(&mut self, _x: f32, _y: f32, _radius: f32, _color: Color) {}

        fn draw_circle_lines(&mut self, _x: f32, _y: f32, _radius: f32, _thickness: f32, _color: Color) {}

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
        ) {}

        fn push_clip_rect(&mut self, _rect: Rect) {}

        fn pop_clip_rect(&mut self) {}
    }

    impl Text for TestContext {
        fn draw_text(&mut self, text: &str, x: f32, y: f32, font_size: f32, color: Color) -> TextDimensions {
            self.draw_text_calls.push(DrawTextCall {
                text: text.to_string(),
                x,
                y,
                font_size,
                color,
            });
            self.text_dims
        }

        fn draw_text_ex(&mut self, _text: &str, _x: f32, _y: f32, _params: TextParams) -> TextDimensions {
            self.text_dims
        }

        fn measure_text(&self, _text: &str, _font_size: f32) -> TextDimensions {
            self.text_dims
        }
    }

    impl Camera for TestContext {
        fn set_camera(&mut self, _camera: &Camera2D) {}
        fn set_default_camera(&mut self) {}
        fn screen_to_world(&self, _camera: &Camera2D, screen_pos: Vec2) -> Vec2 { screen_pos }
        fn create_render_target(&self, _width: u32, _height: u32) -> BishopRenderTarget {
            panic!("not used in toast tests")
        }
    }

    impl Window for TestContext {
        fn screen_width(&self) -> f32 { 800.0 }
        fn screen_height(&self) -> f32 { self.screen_height }
        fn set_cursor_icon(&mut self, _icon: CursorIcon) {}
        fn toggle_fullscreen(&mut self) -> bool { false }
        fn is_fullscreen(&self) -> bool { false }
        fn scale_factor(&self) -> f32 { 1.0 }
    }

    impl Time for TestContext {
        fn get_frame_time(&self) -> f32 { 0.016 }
        fn get_frame_spike_ms(&self) -> f32 { 0.0 }
        fn update(&mut self) {}
    }

    impl RenderOps for TestContext {
        fn begin_render_to_target(&mut self, _rt: &BishopRenderTarget) {}
        fn end_render_to_target(&mut self) {}
        fn draw_render_target(&mut self, _rt: &BishopRenderTarget, _x: f32, _y: f32, _w: f32, _h: f32) {}
        fn create_drawable_render_target(&self, _width: u32, _height: u32) -> BishopRenderTarget {
            panic!("not used in toast tests")
        }
    }

    impl TextureLoader for TestContext {
        fn load_texture_from_bytes(&self, _data: &[u8]) -> Result<Texture2D, String> {
            panic!("not used in toast tests")
        }

        fn load_texture_from_path(&self, _path: &str) -> Result<Texture2D, String> {
            panic!("not used in toast tests")
        }

        fn empty_texture(&self) -> Texture2D {
            panic!("not used in toast tests")
        }
    }

    #[test]
    fn toast_centers_text_using_baseline_offset() {
        let text_dims = TextDimensions {
            width: 80.0,
            height: 16.0,
            offset_y: 12.0,
        };
        let mut ctx = TestContext::new(200.0, text_dims);
        let mut toast = Toast::new("Saved", 5.0);

        toast.update(&mut ctx);

        let bg = ctx.rect_calls[0];
        let text_call = &ctx.draw_text_calls[0];
        let expected_baseline_y = bg.y + (bg.h - text_dims.height) / 2.0 + text_dims.offset_y;

        assert_eq!(text_call.text, "Saved");
        assert_eq!(text_call.font_size, DEFAULT_FONT_SIZE_16);
        assert_eq!(text_call.color, Color::WHITE);
        assert!((text_call.x - (bg.x + PADDING)).abs() < f32::EPSILON);
        assert!((text_call.y - expected_baseline_y).abs() < f32::EPSILON);
    }
}
