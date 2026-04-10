use bishop::material::BishopRenderTarget;
use bishop::prelude::*;

pub(super) struct WidgetTestContext {
    pub mouse_pos: (f32, f32),
    pub left_pressed: bool,
    pub left_down: bool,
    pub left_released: bool,
    pub right_pressed: bool,
    pub right_down: bool,
    pub right_released: bool,
    pub rectangle_fills: Vec<Color>,
    pub rectangle_lines: Vec<Color>,
    pub text_colors: Vec<Color>,
}

impl WidgetTestContext {
    pub(super) fn new() -> Self {
        Self {
            mouse_pos: (0.0, 0.0),
            left_pressed: false,
            left_down: false,
            left_released: false,
            right_pressed: false,
            right_down: false,
            right_released: false,
            rectangle_fills: Vec::new(),
            rectangle_lines: Vec::new(),
            text_colors: Vec::new(),
        }
    }
}

impl Input for WidgetTestContext {
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

impl Draw for WidgetTestContext {
    fn draw_rectangle(&mut self, _x: f32, _y: f32, _w: f32, _h: f32, color: Color) {
        self.rectangle_fills.push(color);
    }

    fn draw_rectangle_lines(
        &mut self,
        _x: f32,
        _y: f32,
        _w: f32,
        _h: f32,
        _thickness: f32,
        color: Color,
    ) {
        self.rectangle_lines.push(color);
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

impl Text for WidgetTestContext {
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

    fn draw_text_ex(&mut self, text: &str, _x: f32, _y: f32, params: TextParams) -> TextDimensions {
        self.text_colors.push(params.color);
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

impl Camera for WidgetTestContext {
    fn set_camera(&mut self, _camera: &Camera2D) {}

    fn set_default_camera(&mut self) {}

    fn screen_to_world(&self, _camera: &Camera2D, screen_pos: Vec2) -> Vec2 {
        screen_pos
    }

    fn create_render_target(&self, _width: u32, _height: u32) -> BishopRenderTarget {
        panic!("render targets are not used in widget tests")
    }
}

impl Window for WidgetTestContext {
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

impl Time for WidgetTestContext {
    fn get_frame_time(&self) -> f32 {
        1.0 / 60.0
    }

    fn get_frame_spike_ms(&self) -> f32 {
        0.0
    }

    fn update(&mut self) {}
}

impl RenderOps for WidgetTestContext {
    fn begin_render_to_target(&mut self, _rt: &BishopRenderTarget) {}

    fn end_render_to_target(&mut self) {}

    fn draw_render_target(&mut self, _rt: &BishopRenderTarget, _x: f32, _y: f32, _w: f32, _h: f32) {
    }

    fn create_drawable_render_target(&self, _width: u32, _height: u32) -> BishopRenderTarget {
        panic!("render targets are not used in widget tests")
    }
}

impl TextureLoader for WidgetTestContext {
    fn load_texture_from_bytes(&self, _data: &[u8]) -> Result<Texture2D, String> {
        panic!("textures are not used in widget tests")
    }

    fn load_texture_from_path(&self, _path: &str) -> Result<Texture2D, String> {
        panic!("textures are not used in widget tests")
    }

    fn empty_texture(&self) -> Texture2D {
        panic!("textures are not used in widget tests")
    }
}
