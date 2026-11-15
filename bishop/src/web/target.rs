pub(crate) struct TargetQuad {
    pub positions: [[f32; 2]; 4],
    pub uv: [[f32; 2]; 4],
}

pub(crate) fn target_quad(x: f32, y: f32, w: f32, h: f32) -> TargetQuad {
    TargetQuad {
        positions: [[x, y], [x + w, y], [x + w, y + h], [x, y + h]],
        uv: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
    }
}

#[cfg(target_os = "emscripten")]
use std::rc::Rc;

#[cfg(target_os = "emscripten")]
use glow::HasContext;

#[cfg(target_os = "emscripten")]
use super::gl_context::{GlContext, WebContext};
#[cfg(target_os = "emscripten")]
use crate::material::RenderOps;
#[cfg(target_os = "emscripten")]
use crate::types::Color;

#[cfg(target_os = "emscripten")]
struct TargetStorage {
    owner: Rc<GlContext>,
    framebuffer: glow::Framebuffer,
    texture: glow::Texture,
    width: u32,
    height: u32,
}

#[cfg(target_os = "emscripten")]
impl Drop for TargetStorage {
    fn drop(&mut self) {
        if self.owner.make_current().is_ok() {
            unsafe {
                self.owner.gl.delete_framebuffer(self.framebuffer);
                self.owner.gl.delete_texture(self.texture);
            }
        }
    }
}

/// A cloneable WebGL2 framebuffer and color texture.
#[cfg(target_os = "emscripten")]
#[derive(Clone)]
pub struct WebRenderTarget {
    state: TargetState,
}

#[cfg(target_os = "emscripten")]
#[derive(Clone)]
enum TargetState {
    Ready(Rc<TargetStorage>),
    Failed { width: u32, height: u32, error: String },
}

#[cfg(target_os = "emscripten")]
impl std::fmt::Debug for WebRenderTarget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WebRenderTarget")
            .field("width", &self.width())
            .field("height", &self.height())
            .field("error", &self.failure())
            .finish()
    }
}

#[cfg(target_os = "emscripten")]
impl WebRenderTarget {
    pub(crate) fn new(
        owner: Rc<GlContext>,
        width: u32,
        height: u32,
        restore: Option<glow::Framebuffer>,
    ) -> Result<Self, String> {
        owner.make_current()?;
        let width_gl = i32::try_from(width).map_err(|error| error.to_string())?;
        let height_gl = i32::try_from(height).map_err(|error| error.to_string())?;
        if width == 0 || height == 0 {
            return Err("WebGL2 render target dimensions must be nonzero".to_owned());
        }
        let gl = &owner.gl;
        let texture = unsafe { gl.create_texture()? };
        let framebuffer = match unsafe { gl.create_framebuffer() } {
            Ok(framebuffer) => framebuffer,
            Err(error) => {
                unsafe { gl.delete_texture(texture) };
                return Err(error);
            }
        };
        let result = unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
            gl.tex_image_2d(
                glow::TEXTURE_2D, 0, glow::RGBA as i32, width_gl, height_gl, 0,
                glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(None),
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(texture), 0,
            );
            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            let error = gl.get_error();
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.bind_framebuffer(glow::FRAMEBUFFER, restore);
            if status != glow::FRAMEBUFFER_COMPLETE || error != glow::NO_ERROR {
                Err(format!("WebGL2 render target incomplete: status {status}, GL error {error}"))
            } else {
                Ok(())
            }
        };
        if let Err(error) = result {
            unsafe {
                gl.delete_framebuffer(framebuffer);
                gl.delete_texture(texture);
            }
            return Err(error);
        }
        Ok(Self {
            state: TargetState::Ready(Rc::new(TargetStorage {
                owner, framebuffer, texture, width, height,
            })),
        })
    }

    pub(crate) fn failed(width: u32, height: u32, error: String) -> Self {
        Self {
            state: TargetState::Failed { width, height, error },
        }
    }

    pub(crate) fn failure(&self) -> Option<&str> {
        match &self.state {
            TargetState::Ready(_) => None,
            TargetState::Failed { error, .. } => Some(error),
        }
    }

    pub(crate) fn belongs_to(&self, owner: &Rc<GlContext>) -> bool {
        match &self.state {
            TargetState::Ready(inner) => Rc::ptr_eq(&inner.owner, owner),
            TargetState::Failed { .. } => false,
        }
    }

    pub(crate) fn framebuffer(&self) -> Option<glow::Framebuffer> {
        match &self.state {
            TargetState::Ready(inner) => Some(inner.framebuffer),
            TargetState::Failed { .. } => None,
        }
    }

    pub(crate) fn texture(&self) -> Option<glow::Texture> {
        match &self.state {
            TargetState::Ready(inner) => Some(inner.texture),
            TargetState::Failed { .. } => None,
        }
    }

    /// Returns the framebuffer width in pixels.
    pub fn width(&self) -> u32 {
        match &self.state {
            TargetState::Ready(inner) => inner.width,
            TargetState::Failed { width, .. } => *width,
        }
    }

    /// Returns the framebuffer height in pixels.
    pub fn height(&self) -> u32 {
        match &self.state {
            TargetState::Ready(inner) => inner.height,
            TargetState::Failed { height, .. } => *height,
        }
    }
}

#[cfg(target_os = "emscripten")]
impl RenderOps for WebContext {
    fn begin_render_to_target(&mut self, rt: &WebRenderTarget) {
        if self.renderer.active_target.is_some() {
            self.owner.record_error("nested WebGL2 offscreen targets are unsupported".to_owned());
            return;
        }
        if let Some(error) = rt.failure() {
            self.owner.record_error(format!("cannot bind failed WebGL2 target: {error}"));
            return;
        }
        if !rt.belongs_to(&self.owner) {
            self.owner.record_error("WebGL2 target belongs to another context".to_owned());
            return;
        }
        let Some(framebuffer) = rt.framebuffer() else {
            self.owner.record_error("WebGL2 target has no framebuffer".to_owned());
            return;
        };
        if let Err(error) = self.owner.make_current() {
            self.owner.record_error(error);
            return;
        }
        self.renderer.saved_camera = self.renderer.camera.clone();
        self.renderer.saved_clips = Some(self.renderer.clips.clone());
        self.renderer.active_target = Some(rt.clone());
        let gl = &self.owner.gl;
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            gl.viewport(0, 0, rt.width() as i32, rt.height() as i32);
            gl.disable(glow::SCISSOR_TEST);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            let error = gl.get_error();
            if error != glow::NO_ERROR {
                self.owner.record_error(format!("WebGL2 target bind failed: GL error {error}"));
            }
        }
    }

    fn end_render_to_target(&mut self) {
        if self.renderer.active_target.is_none() {
            self.owner.record_error("WebGL2 offscreen target was not begun".to_owned());
            return;
        }
        if let Err(error) = self.owner.make_current() {
            self.owner.record_error(error);
            return;
        }
        self.renderer.active_target = None;
        self.renderer.camera = self.renderer.saved_camera.take();
        if let Some(clips) = self.renderer.saved_clips.take() {
            self.renderer.clips = clips;
        }
        let gl = &self.owner.gl;
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(0, 0, self.renderer.canvas_size.0, self.renderer.canvas_size.1);
            let error = gl.get_error();
            if error != glow::NO_ERROR {
                self.owner.record_error(format!("WebGL2 target restore failed: GL error {error}"));
            }
        }
    }

    fn draw_render_target(&mut self, rt: &WebRenderTarget, x: f32, y: f32, w: f32, h: f32) {
        if let Some(error) = rt.failure() {
            self.owner.record_error(format!("cannot draw failed WebGL2 target: {error}"));
            return;
        }
        if !rt.belongs_to(&self.owner) {
            self.owner.record_error("WebGL2 target belongs to another context".to_owned());
            return;
        }
        let Some(texture) = rt.texture() else {
            self.owner.record_error("WebGL2 target has no color texture".to_owned());
            return;
        };
        let quad = target_quad(x, y, w, h);
        self.renderer.draw_quad(quad.positions, quad.uv, texture, Color::WHITE);
    }

    fn create_drawable_render_target(&self, width: u32, height: u32) -> WebRenderTarget {
        let restore = self.renderer.active_target.as_ref().and_then(WebRenderTarget::framebuffer);
        match WebRenderTarget::new(Rc::clone(&self.owner), width, height, restore) {
            Ok(target) => target,
            Err(error) => {
                self.owner.record_error(error.clone());
                WebRenderTarget::failed(width, height, error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::target_quad;

    #[test]
    fn offscreen_target_scales_equally_in_both_axes() {
        let quad = target_quad(30.0, 0.0, 240.0, 180.0);
        let target_width = 120.0;
        let target_height = 90.0;

        assert_eq!(quad.positions[0], [30.0, 0.0]);
        assert_eq!(quad.positions[2], [270.0, 180.0]);
        assert_eq!(
            (quad.positions[2][0] - quad.positions[0][0]) / target_width,
            2.0
        );
        assert_eq!(
            (quad.positions[2][1] - quad.positions[0][1]) / target_height,
            2.0
        );
    }

    #[test]
    fn framebuffer_target_uses_uvs_opposite_to_uploaded_png() {
        let quad = target_quad(0.0, 0.0, 240.0, 180.0);

        assert_eq!(quad.uv[0], [0.0, 1.0]);
        assert_eq!(quad.uv[2], [1.0, 0.0]);
    }
}
