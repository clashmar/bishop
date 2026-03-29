//! Trait implementations for WgpuContext.

use std::cell::RefCell;

use super::context::WgpuContext;
use super::render::FontAtlas;
use super::render::WgpuTexture;
use crate::camera::{Camera, Camera2D};
use crate::draw::{Draw, DrawTextureParams};
use crate::input::{Input, KeyCode, MouseButton};
use crate::material::RenderOps;
use crate::text::{Text, TextDimensions};
use crate::time::Time;
use crate::types::{Color, Texture2D, Vec2};
use crate::window::Window;
use crate::TextureLoader;

thread_local! {
    static MEASURE_ATLAS: RefCell<Option<FontAtlas>> = const { RefCell::new(None) };
}

impl Input for WgpuContext {
    fn is_key_down(&self, key: KeyCode) -> bool {
        self.input.is_key_down(key)
    }

    fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.input.is_key_pressed(key)
    }

    fn is_key_released(&self, key: KeyCode) -> bool {
        self.input.is_key_released(key)
    }

    fn any_key_pressed(&self) -> bool {
        self.input.any_key_pressed()
    }

    fn is_mouse_button_down(&self, button: MouseButton) -> bool {
        self.input.is_mouse_button_down(button)
    }

    fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.input.is_mouse_button_pressed(button)
    }

    fn is_mouse_button_released(&self, button: MouseButton) -> bool {
        self.input.is_mouse_button_released(button)
    }

    fn mouse_position(&self) -> (f32, f32) {
        self.input.mouse_position()
    }

    fn mouse_delta_position(&self) -> (f32, f32) {
        self.input.mouse_delta_position()
    }

    fn mouse_wheel(&self) -> (f32, f32) {
        self.input.mouse_wheel()
    }

    fn chars_pressed(&self) -> Vec<char> {
        self.input.chars_pressed()
    }

    fn get_time(&self) -> f64 {
        self.time.elapsed()
    }
}

impl Draw for WgpuContext {
    fn draw_rectangle(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let prev = self.primitive_renderer.vertex_count() as u32;
        self.primitive_renderer.draw_rectangle(x, y, w, h, color);
        self.record_primitive_segment(prev);
    }

    fn draw_rectangle_lines(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        thickness: f32,
        color: Color,
    ) {
        let prev = self.primitive_renderer.vertex_count() as u32;
        self.primitive_renderer
            .draw_rectangle_lines(x, y, w, h, thickness, color);
        self.record_primitive_segment(prev);
    }

    fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color) {
        let prev = self.primitive_renderer.vertex_count() as u32;
        self.primitive_renderer
            .draw_line(x1, y1, x2, y2, thickness, color);
        self.record_primitive_segment(prev);
    }

    fn draw_circle(&mut self, x: f32, y: f32, radius: f32, color: Color) {
        let prev = self.primitive_renderer.vertex_count() as u32;
        self.primitive_renderer.draw_circle(x, y, radius, color);
        self.record_primitive_segment(prev);
    }

    fn draw_circle_lines(&mut self, x: f32, y: f32, radius: f32, thickness: f32, color: Color) {
        let prev = self.primitive_renderer.vertex_count() as u32;
        self.primitive_renderer
            .draw_circle_lines(x, y, radius, thickness, color);
        self.record_primitive_segment(prev);
    }

    fn draw_triangle(&mut self, v1: Vec2, v2: Vec2, v3: Vec2, color: Color) {
        let prev = self.primitive_renderer.vertex_count() as u32;
        self.primitive_renderer.draw_triangle(v1, v2, v3, color);
        self.record_primitive_segment(prev);
    }

    fn clear_background(&mut self, color: Color) {
        self.clear_color = Some(color);
    }

    fn draw_texture(&mut self, texture: &Texture2D, x: f32, y: f32, color: Color) {
        let prev = self.texture_renderer.batch_count();
        self.texture_renderer
            .draw_texture(texture.inner(), x, y, color);
        self.record_texture_segment(prev);
    }

    fn draw_texture_ex(
        &mut self,
        texture: &Texture2D,
        x: f32,
        y: f32,
        color: Color,
        params: DrawTextureParams,
    ) {
        let prev = self.texture_renderer.batch_count();
        self.texture_renderer
            .draw_texture_ex(texture.inner(), x, y, color, params);
        self.record_texture_segment(prev);
    }

    fn push_clip_rect(&mut self, rect: crate::types::Rect) {
        self.push_clip_rect(rect);
    }

    fn pop_clip_rect(&mut self) {
        self.pop_clip_rect();
    }
}

impl Text for WgpuContext {
    fn draw_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: Color,
    ) -> TextDimensions {
        let prev = self.text_renderer.vertex_count() as u32;
        let dims = self.text_renderer.draw_text(text, x, y, font_size, color);
        self.record_text_segment(prev);
        dims
    }

    fn draw_text_ex(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        params: crate::text::TextParams,
    ) -> TextDimensions {
        let prev = self.text_renderer.vertex_count() as u32;
        let dims = self.text_renderer.draw_text_ex(text, x, y, &params);
        self.record_text_segment(prev);
        dims
    }

    fn measure_text(&self, text: &str, font_size: f32) -> TextDimensions {
        MEASURE_ATLAS.with(|cell| {
            let mut atlas_opt = cell.borrow_mut();
            if atlas_opt.is_none() {
                *atlas_opt =
                    Some(FontAtlas::with_default_font().expect("Failed to create font atlas"));
            }
            atlas_opt.as_mut().unwrap().measure_text(text, font_size)
        })
    }
}

impl Camera for WgpuContext {
    fn set_camera(&mut self, camera: &Camera2D) {
        self.flush_if_needed();
        self.current_camera = Some(camera.clone());
    }

    fn set_default_camera(&mut self) {
        self.flush_if_needed();
        self.current_camera = None;
    }

    fn screen_to_world(&self, camera: &Camera2D, screen_pos: Vec2) -> Vec2 {
        camera.screen_to_world(screen_pos, self.screen_width(), self.screen_height())
    }

    fn create_render_target(&self, width: u32, height: u32) -> super::render::BishopRenderTarget {
        WgpuContext::create_render_target(self, width, height)
    }
}

impl RenderOps for WgpuContext {
    fn begin_render_to_target(&mut self, rt: &super::render::BishopRenderTarget) {
        WgpuContext::begin_render_to_target(self, rt);
    }

    fn end_render_to_target(&mut self) {
        WgpuContext::end_render_to_target(self);
    }

    fn draw_render_target(
        &mut self,
        rt: &super::render::BishopRenderTarget,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) {
        WgpuContext::draw_render_target(self, rt, x, y, w, h);
    }

    fn create_drawable_render_target(
        &self,
        width: u32,
        height: u32,
    ) -> super::render::BishopRenderTarget {
        WgpuContext::create_drawable_render_target(self, width, height)
    }
}

impl Window for WgpuContext {
    fn screen_width(&self) -> f32 {
        self.screen_width()
    }

    fn screen_height(&self) -> f32 {
        self.screen_height()
    }

    fn set_cursor_icon(&mut self, icon: crate::window::CursorIcon) {
        self.set_cursor_icon(icon);
    }

    fn toggle_fullscreen(&mut self) -> bool {
        self.toggle_fullscreen()
    }

    fn is_fullscreen(&self) -> bool {
        self.is_fullscreen()
    }

    fn scale_factor(&self) -> f32 {
        self.scale_factor()
    }
}

impl Time for WgpuContext {
    fn get_frame_time(&self) -> f32 {
        self.time.frame_time()
    }

    fn get_frame_spike_ms(&self) -> f32 {
        self.time.frame_spike_ms()
    }

    fn update(&mut self) {
        self.input.end_frame();
    }
}

impl TextureLoader for WgpuContext {
    fn load_texture_from_bytes(&self, data: &[u8]) -> Result<Texture2D, String> {
        let wgpu_texture = WgpuTexture::from_png(
            self.device(),
            self.queue(),
            self.texture_bind_group_layout(),
            data,
        )
        .map_err(|e| e.to_string())?;
        Ok(Texture2D::from_wgpu(wgpu_texture))
    }

    fn load_texture_from_path(&self, path: &str) -> Result<Texture2D, String> {
        let data = std::fs::read(path).map_err(|e| format!("Failed to read '{}': {}", path, e))?;
        self.load_texture_from_bytes(&data)
    }

    fn empty_texture(&self) -> Texture2D {
        let data: [u8; 4] = [0, 0, 0, 0];
        let wgpu_texture = WgpuTexture::from_rgba(
            self.device(),
            self.queue(),
            self.texture_bind_group_layout(),
            &data,
            1,
            1,
        );
        Texture2D::from_wgpu(wgpu_texture)
    }
}
