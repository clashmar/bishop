use std::cell::RefCell;
use std::ffi::{CString, c_char, c_void};
use std::rc::Rc;

use glow::HasContext;

use crate::input::KeyboardMouseState;
use crate::platform::CloseState;
use crate::time::web_clock::WebClock;
use crate::types::Texture2D;
use super::draw::WebRenderer;
use super::text::WebText;

unsafe extern "C" {
    fn bishop_open_webgl2(selector: *const c_char) -> i32;
    fn bishop_make_webgl2_current(handle: i32) -> i32;
    fn bishop_close_webgl2(handle: i32);
    fn bishop_canvas_size(selector: *const c_char, width: *mut i32, height: *mut i32) -> i32;
    fn bishop_canvas_scale(selector: *const c_char) -> f64;
    fn bishop_now_ms() -> f64;
    fn bishop_set_canvas_cursor(selector: *const c_char, css_cursor: *const c_char) -> i32;
    fn bishop_fullscreen_status() -> i32;
    fn bishop_toggle_fullscreen(selector: *const c_char) -> i32;
    fn emscripten_webgl_get_proc_address(name: *const c_char) -> *const c_void;
}

const SPRITE_VERTEX_SHADER: &str = "#version 300 es
layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_uv;
uniform vec2 u_resolution;
out vec2 v_uv;
void main() {
    vec2 clip = a_position / u_resolution * 2.0 - 1.0;
    gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
    v_uv = a_uv;
}
";

const SPRITE_FRAGMENT_SHADER: &str = "#version 300 es
precision mediump float;
in vec2 v_uv;
uniform sampler2D u_texture;
uniform vec4 u_tint;
out vec4 out_color;
void main() {
    out_color = texture(u_texture, v_uv) * u_tint;
}
";

pub(crate) struct GlContext {
    pub(crate) gl: glow::Context,
    pub(crate) program: glow::Program,
    handle: i32,
    selector: CString,
    error: RefCell<Option<String>>,
}

impl GlContext {
    fn new(selector: &str) -> Result<Self, String> {
        let selector = CString::new(selector).map_err(|error| error.to_string())?;
        let handle = unsafe { bishop_open_webgl2(selector.as_ptr()) };
        if handle <= 0 {
            return Err(format!("WebGL2 context creation failed: {handle}"));
        }
        let gl = unsafe {
            glow::Context::from_loader_function_cstr(|name| {
                emscripten_webgl_get_proc_address(name.as_ptr())
            })
        };
        let program = match unsafe { create_program(&gl) } {
            Ok(program) => program,
            Err(error) => {
                unsafe { bishop_close_webgl2(handle) };
                return Err(error);
            }
        };
        Ok(Self {
            gl,
            program,
            handle,
            selector,
            error: RefCell::new(None),
        })
    }

    pub(crate) fn make_current(&self) -> Result<(), String> {
        let status = unsafe { bishop_make_webgl2_current(self.handle) };
        if status == 0 {
            Ok(())
        } else {
            Err(format!("WebGL2 context is not current: {status}"))
        }
    }

    pub(crate) fn canvas_size(&self) -> Result<(i32, i32), String> {
        let mut width = 0;
        let mut height = 0;
        let status = unsafe {
            bishop_canvas_size(self.selector.as_ptr(), &mut width, &mut height)
        };
        if status != 0 || width <= 0 || height <= 0 {
            Err(format!("WebGL2 canvas dimensions unavailable: status {status}, {width}x{height}"))
        } else {
            Ok((width, height))
        }
    }

    pub(crate) fn now_ms() -> f64 {
        unsafe { bishop_now_ms() }
    }

    pub(crate) fn set_canvas_cursor(&self, css_cursor: &str) -> Result<(), String> {
        let css_cursor = CString::new(css_cursor).map_err(|error| error.to_string())?;
        let status = unsafe { bishop_set_canvas_cursor(self.selector.as_ptr(), css_cursor.as_ptr()) };
        if status == 0 {
            Ok(())
        } else {
            Err(format!("browser canvas cursor update failed: {status}"))
        }
    }

    pub(crate) fn canvas_scale(&self) -> Result<f32, String> {
        let scale = unsafe { bishop_canvas_scale(self.selector.as_ptr()) };
        if scale.is_finite() && scale > 0.0 {
            Ok(scale as f32)
        } else {
            Err(format!("browser canvas scale unavailable: {scale}"))
        }
    }

    pub(crate) fn fullscreen_status(&self) -> Result<bool, String> {
        match unsafe { bishop_fullscreen_status() } {
            0 => Ok(false),
            1 => Ok(true),
            status => Err(format!("browser fullscreen status unavailable: {status}")),
        }
    }

    pub(crate) fn toggle_fullscreen(&self) -> Result<bool, String> {
        match unsafe { bishop_toggle_fullscreen(self.selector.as_ptr()) } {
            0 => Ok(false),
            1 => Ok(true),
            status => Err(format!("browser fullscreen request failed: {status}")),
        }
    }

    pub(crate) fn record_error(&self, error: String) {
        let mut recorded = self.error.borrow_mut();
        if recorded.is_none() {
            eprintln!("BISHOP_WEBGL_RENDER_ERROR {error}");
            *recorded = Some(error);
        }
    }

    pub(crate) fn render_error(&self) -> Option<String> {
        self.error.borrow().clone()
    }
}

impl Drop for GlContext {
    fn drop(&mut self) {
        if self.make_current().is_ok() {
            unsafe { self.gl.delete_program(self.program) };
        }
        unsafe { bishop_close_webgl2(self.handle) };
    }
}

/// The WebGL2 context backing Bishop's browser platform.
pub struct WebContext {
    pub(crate) owner: Rc<GlContext>,
    pub(crate) empty_texture: Texture2D,
    pub(crate) renderer: WebRenderer,
    pub(crate) text: WebText,
    pub(crate) input: KeyboardMouseState,
    pub(crate) clock: WebClock,
    pub(crate) close: CloseState,
}

impl WebContext {
    /// Opens a WebGL2 context on an Emscripten canvas selector.
    pub fn new(selector: &str) -> Result<Self, String> {
        let owner = Rc::new(GlContext::new(selector)?);
        let empty_texture = super::texture::upload_rgba(&owner, &[0, 0, 0, 0], 1, 1)?;
        let white = super::texture::upload_rgba(&owner, &[255, 255, 255, 255], 1, 1)?;
        let renderer = WebRenderer::new(Rc::clone(&owner), white)?;
        let text = WebText::new()?;
        let clock = WebClock::new(GlContext::now_ms());
        Ok(Self {
            owner,
            empty_texture,
            renderer,
            text,
            input: KeyboardMouseState::new(),
            clock,
            close: CloseState::default(),
        })
    }

    /// Starts a new GL frame and refreshes the canvas dimensions.
    pub fn begin_frame(&mut self) -> Result<(), String> {
        if let Some(error) = self.owner.render_error() {
            return Err(error);
        }
        self.renderer.begin_frame()?;
        self.clock.begin_frame(GlContext::now_ms());
        self.input.begin_frame();
        Ok(())
    }

    /// Grants the browser runner access to Bishop's shared input state.
    pub fn input_state(&mut self) -> &mut KeyboardMouseState {
        &mut self.input
    }

    /// Returns whether a close request was also confirmed by the application.
    pub fn should_exit(&self) -> bool {
        self.close.should_exit()
    }

    /// Submits GL draws and reports any retained render error.
    pub fn end_frame(&mut self) -> Result<(), String> {
        self.owner.make_current()?;
        if self.renderer.active_target.is_some() {
            self.owner.record_error("WebGL2 offscreen target was not ended".to_owned());
        }
        unsafe {
            self.owner.gl.flush();
            let error = self.owner.gl.get_error();
            if error != glow::NO_ERROR {
                self.owner.record_error(format!("WebGL2 frame failed: GL error {error}"));
            }
        }
        match self.owner.render_error() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

unsafe fn create_program(gl: &glow::Context) -> Result<glow::Program, String> {
    let vertex = unsafe { compile_shader(gl, glow::VERTEX_SHADER, SPRITE_VERTEX_SHADER)? };
    let fragment = match unsafe { compile_shader(gl, glow::FRAGMENT_SHADER, SPRITE_FRAGMENT_SHADER) }
    {
        Ok(shader) => shader,
        Err(error) => {
            unsafe { gl.delete_shader(vertex) };
            return Err(error);
        }
    };
    let result = unsafe {
        match gl.create_program() {
            Ok(program) => {
                gl.attach_shader(program, vertex);
                gl.attach_shader(program, fragment);
                gl.link_program(program);
                if gl.get_program_link_status(program) {
                    Ok(program)
                } else {
                    let error = gl.get_program_info_log(program);
                    gl.delete_program(program);
                    Err(format!("WebGL2 sprite program link failed: {error}"))
                }
            }
            Err(error) => Err(error),
        }
    };
    unsafe {
        gl.delete_shader(vertex);
        gl.delete_shader(fragment);
    }
    result
}

unsafe fn compile_shader(gl: &glow::Context, kind: u32, source: &str) -> Result<glow::Shader, String> {
    let shader = unsafe { gl.create_shader(kind)? };
    unsafe {
        gl.shader_source(shader, source);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            let error = gl.get_shader_info_log(shader);
            gl.delete_shader(shader);
            return Err(format!("WebGL2 shader compile failed: {error}"));
        }
    }
    Ok(shader)
}
