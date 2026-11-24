//! Fixed fullscreen lighting shaders for the WebGL2 backend.

use glow::HasContext;
use std::rc::Rc;

use super::gl_context::{GlContext, WebContext};
use super::target::WebRenderTarget;
use crate::material::{AmbientPassParams, PointLightPassParams};

const VERTEX_SHADER: &str = "#version 300 es
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

const AMBIENT_FRAGMENT_SHADER: &str = "#version 300 es
precision highp float;
in vec2 v_uv;
uniform sampler2D u_scene;
uniform vec3 u_color;
uniform float u_exposure;
uniform float u_darkness;
out vec4 out_color;
void main() {
    vec4 base = texture(u_scene, v_uv);
    vec3 ambient = u_color * max(u_exposure, 0.0) * (1.0 - clamp(u_darkness, 0.0, 1.0));
    out_color = vec4(base.rgb * ambient, base.a);
}
";

const POINT_FRAGMENT_SHADER: &str = "#version 300 es
precision highp float;
in vec2 v_uv;
uniform sampler2D u_scene;
uniform vec2 u_center;
uniform vec2 u_resolution;
uniform vec3 u_color;
uniform float u_intensity;
uniform float u_radius;
uniform float u_attenuation;
uniform float u_pixel_size;
uniform bool u_pixelated;
uniform vec2 u_pixel_grid_origin;
out vec4 out_color;
void main() {
    vec4 base = texture(u_scene, v_uv);
    float radius = max(u_radius, 0.0);
    float falloff = 0.0;
    if (radius > 0.0) {
        vec2 pixel_position = vec2(gl_FragCoord.x, u_resolution.y - gl_FragCoord.y);
        if (u_pixelated) {
            float pixel_size = max(u_pixel_size, 1.0);
            vec2 grid_position = pixel_position - u_pixel_grid_origin;
            vec2 pixel = floor(grid_position / pixel_size) * pixel_size
                + vec2(pixel_size * 0.5) + u_pixel_grid_origin;
            pixel_position = clamp(pixel, vec2(0.5), u_resolution - vec2(0.5));
        }
        float normalized = clamp(1.0 - distance(pixel_position, u_center) / radius, 0.0, 1.0);
        falloff = pow(normalized, max(u_attenuation, 0.0));
    }
    vec3 contribution = u_color * max(u_intensity, 0.0) * falloff;
    out_color = vec4(base.rgb * (vec3(1.0) + contribution), base.a);
}
";

struct PassProgram {
    program: glow::Program,
    scene: glow::UniformLocation,
    resolution: glow::UniformLocation,
    color: glow::UniformLocation,
    exposure: Option<glow::UniformLocation>,
    darkness: Option<glow::UniformLocation>,
    center: Option<glow::UniformLocation>,
    intensity: Option<glow::UniformLocation>,
    radius: Option<glow::UniformLocation>,
    attenuation: Option<glow::UniformLocation>,
    pixel_size: Option<glow::UniformLocation>,
    pixelated: Option<glow::UniformLocation>,
    pixel_grid_origin: Option<glow::UniformLocation>,
}

impl PassProgram {
    fn new(owner: &GlContext, fragment_source: &str, ambient: bool) -> Result<Self, String> {
        let program = unsafe { link_program(&owner.gl, VERTEX_SHADER, fragment_source)? };
        let gl = &owner.gl;
        let locations = unsafe {
            (
                gl.get_uniform_location(program, "u_scene"),
                gl.get_uniform_location(program, "u_resolution"),
                gl.get_uniform_location(program, "u_color"),
                gl.get_uniform_location(program, "u_exposure"),
                gl.get_uniform_location(program, "u_darkness"),
                gl.get_uniform_location(program, "u_center"),
                gl.get_uniform_location(program, "u_intensity"),
                gl.get_uniform_location(program, "u_radius"),
                gl.get_uniform_location(program, "u_attenuation"),
                gl.get_uniform_location(program, "u_pixel_size"),
                gl.get_uniform_location(program, "u_pixelated"),
                gl.get_uniform_location(program, "u_pixel_grid_origin"),
            )
        };
        let (Some(scene), Some(resolution), Some(color)) =
            (locations.0, locations.1, locations.2)
        else {
            unsafe { gl.delete_program(program) };
            return Err("WebGL2 lighting shader is missing required uniforms".to_owned());
        };
        if ambient && (locations.3.is_none() || locations.4.is_none()) {
            unsafe { gl.delete_program(program) };
            return Err("WebGL2 ambient shader is missing ambient uniforms".to_owned());
        }
        if !ambient
            && [locations.5.as_ref(), locations.6.as_ref(), locations.7.as_ref(), locations.8.as_ref(), locations.9.as_ref(), locations.10.as_ref(), locations.11.as_ref()]
                .into_iter()
                .any(|location| location.is_none())
        {
            unsafe { gl.delete_program(program) };
            return Err("WebGL2 point-light shader is missing point uniforms".to_owned());
        }
        Ok(Self {
            program,
            scene,
            resolution,
            color,
            exposure: locations.3,
            darkness: locations.4,
            center: locations.5,
            intensity: locations.6,
            radius: locations.7,
            attenuation: locations.8,
            pixel_size: locations.9,
            pixelated: locations.10,
            pixel_grid_origin: locations.11,
        })
    }
}

pub(crate) struct WebLightingRenderer {
    owner: Rc<GlContext>,
    vao: glow::VertexArray,
    buffer: glow::Buffer,
    ambient: PassProgram,
    point: PassProgram,
}

impl WebLightingRenderer {
    pub(crate) fn new(owner: Rc<GlContext>) -> Result<Self, String> {
        owner.make_current()?;
        let ambient = PassProgram::new(&owner, AMBIENT_FRAGMENT_SHADER, true)?;
        let point = match PassProgram::new(&owner, POINT_FRAGMENT_SHADER, false) {
            Ok(point) => point,
            Err(error) => {
                unsafe { owner.gl.delete_program(ambient.program) };
                return Err(error);
            }
        };
        let vao = match unsafe { owner.gl.create_vertex_array() } {
            Ok(vao) => vao,
            Err(error) => {
                unsafe {
                    owner.gl.delete_program(ambient.program);
                    owner.gl.delete_program(point.program);
                }
                return Err(error);
            }
        };
        let buffer = match unsafe { owner.gl.create_buffer() } {
            Ok(buffer) => buffer,
            Err(error) => {
                unsafe {
                    owner.gl.delete_program(ambient.program);
                    owner.gl.delete_program(point.program);
                    owner.gl.delete_vertex_array(vao);
                }
                return Err(error);
            }
        };
        unsafe {
            owner.gl.bind_vertex_array(Some(vao));
            owner.gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer));
            owner.gl.enable_vertex_attrib_array(0);
            owner.gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 16, 0);
            owner.gl.enable_vertex_attrib_array(1);
            owner.gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, 16, 8);
            owner.gl.bind_vertex_array(None);
            owner.gl.bind_buffer(glow::ARRAY_BUFFER, None);
        }
        Ok(Self { owner, vao, buffer, ambient, point })
    }

    fn ambient(&self, source: &WebRenderTarget, destination: &WebRenderTarget, params: AmbientPassParams) -> Result<(), String> {
        self.draw(source, destination, &self.ambient, |gl, uniforms, width, height| unsafe {
            gl.uniform_3_f32(Some(&uniforms.color), params.color[0], params.color[1], params.color[2]);
            gl.uniform_1_f32(uniforms.exposure.as_ref(), params.exposure.max(0.0));
            gl.uniform_1_f32(uniforms.darkness.as_ref(), params.darkness.clamp(0.0, 1.0));
            gl.uniform_2_f32(Some(&uniforms.resolution), width as f32, height as f32);
        })
    }

    fn point(&self, source: &WebRenderTarget, destination: &WebRenderTarget, params: PointLightPassParams) -> Result<(), String> {
        self.draw(source, destination, &self.point, |gl, uniforms, width, height| unsafe {
            gl.uniform_2_f32(Some(&uniforms.resolution), width as f32, height as f32);
            gl.uniform_3_f32(Some(&uniforms.color), params.color[0], params.color[1], params.color[2]);
            gl.uniform_2_f32(uniforms.center.as_ref(), params.center[0], params.center[1]);
            gl.uniform_1_f32(uniforms.intensity.as_ref(), params.intensity.max(0.0));
            gl.uniform_1_f32(uniforms.radius.as_ref(), params.radius.max(0.0));
            gl.uniform_1_f32(uniforms.attenuation.as_ref(), params.attenuation.max(0.0));
            gl.uniform_1_f32(uniforms.pixel_size.as_ref(), params.pixel_size.max(1.0));
            gl.uniform_1_i32(uniforms.pixelated.as_ref(), i32::from(params.pixelated));
            gl.uniform_2_f32(
                uniforms.pixel_grid_origin.as_ref(),
                params.pixel_grid_origin[0],
                params.pixel_grid_origin[1],
            );
        })
    }

    fn draw<F>(&self, source: &WebRenderTarget, destination: &WebRenderTarget, pass: &PassProgram, set_uniforms: F) -> Result<(), String>
    where
        F: FnOnce(&glow::Context, &PassProgram, u32, u32),
    {
        if let Some(error) = source.failure().or_else(|| destination.failure()) {
            return Err(format!("cannot run WebGL2 lighting pass: {error}"));
        }
        if !source.belongs_to(&self.owner) || !destination.belongs_to(&self.owner) {
            return Err("WebGL2 lighting target belongs to another context".to_owned());
        }
        if source.texture() == destination.texture() {
            return Err("WebGL2 lighting source and destination must differ".to_owned());
        }
        if source.width() != destination.width() || source.height() != destination.height() {
            return Err("WebGL2 lighting targets must have matching dimensions".to_owned());
        }
        let Some(framebuffer) = destination.framebuffer() else {
            return Err("WebGL2 lighting destination has no framebuffer".to_owned());
        };
        let Some(texture) = source.texture() else {
            return Err("WebGL2 lighting source has no texture".to_owned());
        };
        self.owner.make_current()?;
        let gl = &self.owner.gl;
        let width = destination.width();
        let height = destination.height();
        let vertices: [f32; 24] = [
            0.0, 0.0, 0.0, 1.0,
            width as f32, 0.0, 1.0, 1.0,
            width as f32, height as f32, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
            width as f32, height as f32, 1.0, 0.0,
            0.0, height as f32, 0.0, 0.0,
        ];
        let mut bytes = Vec::with_capacity(vertices.len() * 4);
        for value in vertices {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            gl.viewport(0, 0, width as i32, height as i32);
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::BLEND);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.use_program(Some(pass.program));
            gl.uniform_1_i32(Some(&pass.scene), 0);
            set_uniforms(gl, pass, width, height);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.buffer));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STREAM_DRAW);
            gl.draw_arrays(glow::TRIANGLES, 0, 6);
            let error = gl.get_error();
            gl.bind_vertex_array(None);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            let canvas_size = match self.owner.canvas_size() {
                Ok(size) => size,
                Err(_) => (width as i32, height as i32),
            };
            gl.viewport(0, 0, canvas_size.0, canvas_size.1);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            if error != glow::NO_ERROR {
                return Err(format!("WebGL2 lighting pass failed: GL error {error}"));
            }
        }
        Ok(())
    }
}

impl Drop for WebLightingRenderer {
    fn drop(&mut self) {
        if self.owner.make_current().is_ok() {
            unsafe {
                self.owner.gl.delete_program(self.ambient.program);
                self.owner.gl.delete_program(self.point.program);
                self.owner.gl.delete_buffer(self.buffer);
                self.owner.gl.delete_vertex_array(self.vao);
            }
        }
    }
}

impl WebContext {
    pub(crate) fn apply_ambient_pass(&mut self, source: &WebRenderTarget, destination: &WebRenderTarget, params: AmbientPassParams) -> Result<(), String> {
        if self.renderer.active_target.is_some() {
            return Err("WebGL2 lighting pass cannot run inside an active target".to_owned());
        }
        self.lighting_renderer.ambient(source, destination, params)
    }

    pub(crate) fn apply_point_light_pass(&mut self, source: &WebRenderTarget, destination: &WebRenderTarget, params: PointLightPassParams) -> Result<(), String> {
        if self.renderer.active_target.is_some() {
            return Err("WebGL2 lighting pass cannot run inside an active target".to_owned());
        }
        self.lighting_renderer.point(source, destination, params)
    }
}

unsafe fn link_program(gl: &glow::Context, vertex_source: &str, fragment_source: &str) -> Result<glow::Program, String> {
    let vertex = unsafe { compile_shader(gl, glow::VERTEX_SHADER, vertex_source)? };
    let fragment = match unsafe { compile_shader(gl, glow::FRAGMENT_SHADER, fragment_source) } {
        Ok(fragment) => fragment,
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
                    Err(format!("WebGL2 lighting program link failed: {error}"))
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
            return Err(format!("WebGL2 lighting shader compile failed: {error}"));
        }
    }
    Ok(shader)
}
