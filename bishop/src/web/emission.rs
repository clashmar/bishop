//! Sprite-shaped emission and halo rendering for the WebGL2 backend.

use glow::HasContext;
use std::rc::Rc;

use super::gl_context::{GlContext, WebContext};
use super::lighting::{VERTEX_SHADER, WebLightingRenderer, link_program};
use super::target::WebRenderTarget;
use crate::material::EmissionPassParams;

const EMISSION_FRAGMENT_SHADER: &str = "#version 300 es
precision highp float;
in vec2 v_uv;
uniform sampler2D u_mask;
uniform sampler2D u_illumination;
uniform vec2 u_resolution;
uniform vec3 u_color;
uniform float u_intensity;
uniform float u_halo_spread;
uniform float u_pixel_size;
uniform bool u_pixelated;
uniform vec2 u_pixel_grid_origin;
out vec4 out_color;
vec3 decode_radiance(vec3 encoded) {
    vec3 bounded = min(encoded, vec3(0.999));
    return bounded / max(vec3(1.0) - bounded, vec3(0.001));
}
vec3 encode_radiance(vec3 radiance) {
    vec3 nonnegative = max(radiance, vec3(0.0));
    return nonnegative / (vec3(1.0) + nonnegative);
}
vec2 quantize_mask_position(vec2 position) {
    float pixel_size = max(u_pixel_size, 1.0);
    vec2 grid_position = position - u_pixel_grid_origin;
    return floor(grid_position / pixel_size) * pixel_size
        + vec2(pixel_size * 0.5) + u_pixel_grid_origin;
}
float mask_alpha_at(vec2 position) {
    if (any(lessThan(position, vec2(0.5)))
        || any(greaterThan(position, u_resolution - vec2(0.5)))) {
        return 0.0;
    }
    vec2 uv = vec2(position.x / u_resolution.x, 1.0 - position.y / u_resolution.y);
    return textureLod(u_mask, uv, 0.0).a;
}
float sampled_mask_alpha(vec2 position) {
    if (u_pixelated) {
        position = quantize_mask_position(position);
    }
    return mask_alpha_at(position);
}
float halo_alpha_at(vec2 position) {
    if (u_halo_spread <= 0.0) {
        return 0.0;
    }
    float weighted_alpha = 0.0;
    for (int y = -1; y <= 1; ++y) {
        for (int x = -1; x <= 1; ++x) {
            float weight_x = x == 0 ? 2.0 : 1.0;
            float weight_y = y == 0 ? 2.0 : 1.0;
            vec2 offset = vec2(float(x), float(y)) * u_halo_spread;
            weighted_alpha += sampled_mask_alpha(position + offset) * weight_x * weight_y;
        }
    }
    return weighted_alpha / 16.0;
}
void main() {
    vec2 pixel_position = vec2(gl_FragCoord.x, u_resolution.y - gl_FragCoord.y);
    vec4 accumulated = texture(u_illumination, v_uv);
    float source_alpha = sampled_mask_alpha(pixel_position);
    float halo_alpha = halo_alpha_at(pixel_position);
    vec3 emission = u_color * max(u_intensity, 0.0) * (source_alpha + halo_alpha);
    out_color = vec4(encode_radiance(decode_radiance(accumulated.rgb) + emission), accumulated.a);
}
";

pub(crate) struct WebEmissionRenderer {
    owner: Rc<GlContext>,
    program: glow::Program,
    mask: glow::UniformLocation,
    illumination: glow::UniformLocation,
    resolution: glow::UniformLocation,
    color: glow::UniformLocation,
    intensity: glow::UniformLocation,
    halo_spread: glow::UniformLocation,
    pixel_size: glow::UniformLocation,
    pixelated: glow::UniformLocation,
    pixel_grid_origin: glow::UniformLocation,
}

impl WebEmissionRenderer {
    pub(crate) fn new(owner: Rc<GlContext>) -> Result<Self, String> {
        owner.make_current()?;
        let program = unsafe { link_program(&owner.gl, VERTEX_SHADER, EMISSION_FRAGMENT_SHADER)? };
        let locations = unsafe {
            (
                owner.gl.get_uniform_location(program, "u_mask"),
                owner.gl.get_uniform_location(program, "u_illumination"),
                owner.gl.get_uniform_location(program, "u_resolution"),
                owner.gl.get_uniform_location(program, "u_color"),
                owner.gl.get_uniform_location(program, "u_intensity"),
                owner.gl.get_uniform_location(program, "u_halo_spread"),
                owner.gl.get_uniform_location(program, "u_pixel_size"),
                owner.gl.get_uniform_location(program, "u_pixelated"),
                owner.gl.get_uniform_location(program, "u_pixel_grid_origin"),
            )
        };
        let (
            Some(mask),
            Some(illumination),
            Some(resolution),
            Some(color),
            Some(intensity),
            Some(halo_spread),
            Some(pixel_size),
            Some(pixelated),
            Some(pixel_grid_origin),
        ) = locations
        else {
            unsafe { owner.gl.delete_program(program) };
            return Err("WebGL2 emission shader is missing required uniforms".to_owned());
        };

        Ok(Self {
            owner,
            program,
            mask,
            illumination,
            resolution,
            color,
            intensity,
            halo_spread,
            pixel_size,
            pixelated,
            pixel_grid_origin,
        })
    }

    pub(crate) fn apply(
        &self,
        lighting_renderer: &WebLightingRenderer,
        source_mask: &WebRenderTarget,
        illumination: &WebRenderTarget,
        destination: &WebRenderTarget,
        params: EmissionPassParams,
    ) -> Result<(), String> {
        lighting_renderer.draw_custom_pass(
            source_mask,
            Some(illumination),
            destination,
            self.program,
            "emission",
            &self.mask,
            Some(&self.illumination),
            |gl, width, height| unsafe {
                gl.uniform_2_f32(Some(&self.resolution), width as f32, height as f32);
                gl.uniform_3_f32(Some(&self.color), params.color[0], params.color[1], params.color[2]);
                gl.uniform_1_f32(Some(&self.intensity), params.intensity.max(0.0));
                gl.uniform_1_f32(Some(&self.halo_spread), params.halo_spread.max(0.0));
                gl.uniform_1_f32(Some(&self.pixel_size), params.pixel_size.max(1.0));
                gl.uniform_1_i32(Some(&self.pixelated), i32::from(params.pixelated));
                gl.uniform_2_f32(
                    Some(&self.pixel_grid_origin),
                    params.pixel_grid_origin[0],
                    params.pixel_grid_origin[1],
                );
            },
        )
    }
}

impl Drop for WebEmissionRenderer {
    fn drop(&mut self) {
        if self.owner.make_current().is_ok() {
            unsafe {
                self.owner.gl.delete_program(self.program);
            }
        }
    }
}

impl WebContext {
    pub(crate) fn apply_emission_pass(
        &mut self,
        source_mask: &WebRenderTarget,
        illumination: &WebRenderTarget,
        destination: &WebRenderTarget,
        params: EmissionPassParams,
    ) -> Result<(), String> {
        if self.renderer.active_target.is_some() {
            return Err("WebGL2 emission pass cannot run inside an active target".to_owned());
        }
        self.emission_renderer.apply(
            &self.lighting_renderer,
            source_mask,
            illumination,
            destination,
            params,
        )
    }
}
