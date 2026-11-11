use std::rc::Rc;

use glow::HasContext;

use super::clip::{ClipRegion, ClipStack, scissor_y};
use super::gl_context::{GlContext, WebContext};
use super::target::WebRenderTarget;
use crate::camera::{Camera, Camera2D};
use crate::draw::texture_geometry::sprite_quad;
use crate::draw::{Draw, DrawTextureParams};
use crate::material::{BishopRenderTarget, RenderOps};
use crate::types::{Color, Rect, Texture2D, Vec2};

#[derive(Clone, Copy)]
pub(super) struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
}

impl Vertex {
    pub(super) fn new(position: [f32; 2], uv: [f32; 2]) -> Self {
        Self { position, uv }
    }

    pub(super) fn solid(position: [f32; 2]) -> Self {
        Self::new(position, [0.0, 0.0])
    }
}

pub(crate) struct WebRenderer {
    pub(crate) owner: Rc<GlContext>,
    vao: glow::VertexArray,
    buffer: glow::Buffer,
    tint_uniform: glow::UniformLocation,
    resolution_uniform: glow::UniformLocation,
    pub(crate) white: Texture2D,
    pub(crate) camera: Option<Camera2D>,
    pub(crate) clips: ClipStack,
    pub(crate) active_target: Option<WebRenderTarget>,
    pub(crate) saved_camera: Option<Camera2D>,
    pub(crate) saved_clips: Option<ClipStack>,
    pub(crate) canvas_size: (i32, i32),
}

impl WebRenderer {
    pub(crate) fn new(owner: Rc<GlContext>, white: Texture2D) -> Result<Self, String> {
        owner.make_current()?;
        let canvas_size = owner.canvas_size()?;
        let gl = &owner.gl;
        let vao = unsafe { gl.create_vertex_array()? };
        let buffer = match unsafe { gl.create_buffer() } {
            Ok(buffer) => buffer,
            Err(error) => {
                unsafe { gl.delete_vertex_array(vao) };
                return Err(error);
            }
        };
        let result = unsafe {
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 16, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, 16, 8);
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            let tint = gl.get_uniform_location(owner.program, "u_tint");
            let resolution = gl.get_uniform_location(owner.program, "u_resolution");
            let error = gl.get_error();
            match (tint, resolution, error) {
                (Some(tint), Some(resolution), glow::NO_ERROR) => Ok((tint, resolution)),
                _ => Err(format!("WebGL2 vertex setup failed: GL error {error}")),
            }
        };
        let (tint_uniform, resolution_uniform) = match result {
            Ok(uniforms) => uniforms,
            Err(error) => {
                unsafe {
                    gl.delete_buffer(buffer);
                    gl.delete_vertex_array(vao);
                }
                return Err(error);
            }
        };
        Ok(Self {
            owner,
            vao,
            buffer,
            tint_uniform,
            resolution_uniform,
            white,
            camera: None,
            clips: ClipStack::default(),
            active_target: None,
            saved_camera: None,
            saved_clips: None,
            canvas_size,
        })
    }

    pub(crate) fn begin_frame(&mut self) -> Result<(), String> {
        if self.active_target.is_some() {
            return Err("WebGL2 offscreen target was not ended before the next frame".to_owned());
        }
        self.owner.make_current()?;
        self.canvas_size = self.owner.canvas_size()?;
        self.camera = None;
        self.clips = ClipStack::default();
        unsafe {
            self.owner.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            self.owner.gl.viewport(0, 0, self.canvas_size.0, self.canvas_size.1);
            self.owner.gl.disable(glow::SCISSOR_TEST);
        }
        Ok(())
    }

    pub(crate) fn size(&self) -> (i32, i32) {
        self.active_target
            .as_ref()
            .map(|target| (target.width() as i32, target.height() as i32))
            .unwrap_or(self.canvas_size)
    }

    pub(crate) fn clear(&mut self, color: Color) {
        if !self.can_render() {
            return;
        }
        let rgba: [f32; 4] = color.into();
        let gl = &self.owner.gl;
        unsafe {
            gl.disable(glow::SCISSOR_TEST);
            gl.clear_color(rgba[0], rgba[1], rgba[2], rgba[3]);
            gl.clear(glow::COLOR_BUFFER_BIT);
            let error = gl.get_error();
            if error != glow::NO_ERROR {
                self.owner.record_error(format!("WebGL2 clear failed: GL error {error}"));
            }
        }
    }

    pub(crate) fn draw_quad(
        &self,
        positions: [[f32; 2]; 4],
        uv: [[f32; 2]; 4],
        texture: glow::Texture,
        color: Color,
    ) {
        let vertices = [
            Vertex::new(positions[0], uv[0]),
            Vertex::new(positions[1], uv[1]),
            Vertex::new(positions[2], uv[2]),
            Vertex::new(positions[0], uv[0]),
            Vertex::new(positions[2], uv[2]),
            Vertex::new(positions[3], uv[3]),
        ];
        self.draw_triangles(&vertices, texture, color);
    }

    pub(super) fn draw_triangles(&self, vertices: &[Vertex], texture: glow::Texture, color: Color) {
        if !self.can_render() || vertices.is_empty() {
            return;
        }
        if self.active_target.as_ref().is_some_and(|target| target.texture() == Some(texture)) {
            self.owner.record_error("WebGL2 cannot sample the active framebuffer texture".to_owned());
            return;
        }
        let (width, height) = self.size();
        let mut bytes = Vec::with_capacity(vertices.len() * 16);
        for vertex in vertices {
            let position = self.camera.as_ref().map_or(
                Vec2::from(vertex.position),
                |camera| {
                    camera.world_to_screen(
                        Vec2::from(vertex.position), width as f32, height as f32,
                    )
                },
            );
            for value in [position.x, position.y, vertex.uv[0], vertex.uv[1]] {
                bytes.extend_from_slice(&value.to_ne_bytes());
            }
        }
        let rgba: [f32; 4] = color.into();
        let gl = &self.owner.gl;
        unsafe {
            self.apply_scissor();
            gl.use_program(Some(self.owner.program));
            gl.uniform_2_f32(Some(&self.resolution_uniform), width as f32, height as f32);
            gl.uniform_4_f32(Some(&self.tint_uniform), rgba[0], rgba[1], rgba[2], rgba[3]);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.buffer));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STREAM_DRAW);
            gl.draw_arrays(glow::TRIANGLES, 0, vertices.len() as i32);
            let error = gl.get_error();
            if error != glow::NO_ERROR {
                self.owner.record_error(format!("WebGL2 draw failed: GL error {error}"));
            }
            gl.bind_vertex_array(None);
            gl.bind_texture(glow::TEXTURE_2D, None);
        }
    }

    fn can_render(&self) -> bool {
        if self.owner.render_error().is_some() {
            return false;
        }
        match self.owner.make_current() {
            Ok(()) => true,
            Err(error) => {
                self.owner.record_error(error);
                false
            }
        }
    }

    unsafe fn apply_scissor(&self) {
        let gl = &self.owner.gl;
        let (width, height) = self.size();
        let camera_rect = self.camera.as_ref().and_then(|camera| camera.viewport).map(
            |(x, y, w, h)| Rect::new(x as f32, scissor_y(height, y, h) as f32, w as f32, h as f32),
        );
        let mut clip = match self.clips.current() {
            ClipRegion::Unclipped => camera_rect,
            ClipRegion::Empty => Some(Rect::new(0.0, 0.0, 0.0, 0.0)),
            ClipRegion::Rect(rect) => match camera_rect {
                Some(viewport) => viewport.intersection(&rect),
                None => Some(rect),
            },
        };
        if clip.is_none() && self.clips.current() == ClipRegion::Unclipped {
            unsafe { gl.disable(glow::SCISSOR_TEST) };
            return;
        }
        clip = clip.and_then(|rect| {
            rect.intersection(&Rect::new(0.0, 0.0, width as f32, height as f32))
        });
        let (x, y, w, h) = clip.map_or((0, 0, 0, 0), |rect| {
            let left = rect.x.ceil() as i32;
            let top = rect.y.ceil() as i32;
            let right = rect.right().floor() as i32;
            let bottom = rect.bottom().floor() as i32;
            (left, scissor_y(height, top, (bottom - top).max(0)),
                (right - left).max(0), (bottom - top).max(0))
        });
        unsafe {
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(x, y, w, h);
        }
    }
}

impl Drop for WebRenderer {
    fn drop(&mut self) {
        if self.owner.make_current().is_ok() {
            unsafe {
                self.owner.gl.delete_buffer(self.buffer);
                self.owner.gl.delete_vertex_array(self.vao);
            }
        }
    }
}

impl Draw for WebContext {
    fn draw_rectangle(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        self.renderer.draw_colored_rectangle(x, y, w, h, color);
    }

    fn draw_rectangle_lines(&mut self, x: f32, y: f32, w: f32, h: f32, thickness: f32, color: Color) {
        self.renderer.draw_rectangle_outline(x, y, w, h, thickness, color);
    }

    fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color) {
        self.renderer.draw_colored_line(x1, y1, x2, y2, thickness, color);
    }

    fn draw_circle(&mut self, x: f32, y: f32, radius: f32, color: Color) {
        self.renderer.draw_colored_circle(x, y, radius, color);
    }

    fn draw_circle_lines(&mut self, x: f32, y: f32, radius: f32, thickness: f32, color: Color) {
        self.renderer.draw_circle_outline(x, y, radius, thickness, color);
    }

    fn draw_arc_lines(
        &mut self, center: Vec2, radius: f32, start_angle: f32,
        end_angle: f32, thickness: f32, color: Color,
    ) {
        self.renderer.draw_arc(center, radius, start_angle, end_angle, thickness, color);
    }

    fn draw_triangle(&mut self, v1: Vec2, v2: Vec2, v3: Vec2, color: Color) {
        self.renderer.draw_colored_triangle(v1, v2, v3, color);
    }

    fn clear_background(&mut self, color: Color) {
        self.renderer.clear(color);
    }

    fn draw_texture(&mut self, texture: &Texture2D, x: f32, y: f32, color: Color) {
        self.draw_texture_ex(texture, x, y, color, DrawTextureParams::default());
    }

    fn draw_texture_ex(
        &mut self, texture: &Texture2D, x: f32, y: f32,
        color: Color, params: DrawTextureParams,
    ) {
        let web = texture.inner_web();
        if !Rc::ptr_eq(&web.owner, &self.owner) {
            self.owner.record_error("WebGL2 texture belongs to a different context".to_owned());
            return;
        }
        let quad = sprite_quad(
            Vec2::new(texture.width(), texture.height()), Vec2::new(x, y), &params,
        );
        self.renderer.draw_quad(quad.positions, quad.uv, web.handle(), color);
    }

    fn push_clip_rect(&mut self, rect: Rect) {
        self.renderer.clips.push(rect);
    }

    fn pop_clip_rect(&mut self) {
        self.renderer.clips.pop();
    }
}

impl Camera for WebContext {
    fn set_camera(&mut self, camera: &Camera2D) {
        self.renderer.camera = Some(camera.clone());
    }

    fn set_default_camera(&mut self) {
        self.renderer.camera = None;
    }

    fn screen_to_world(&self, camera: &Camera2D, screen_pos: Vec2) -> Vec2 {
        let (width, height) = self.renderer.size();
        camera.screen_to_world(screen_pos, width as f32, height as f32)
    }

    fn create_render_target(&self, width: u32, height: u32) -> BishopRenderTarget {
        self.create_drawable_render_target(width, height)
    }
}
