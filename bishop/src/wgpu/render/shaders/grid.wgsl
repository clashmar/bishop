// Grid shader for editor grid overlay.
// Renders an infinite grid with anti-aliased lines that scale with zoom.

struct CameraUniforms {
    projection: mat4x4<f32>,
}

struct ModelUniforms {
    model: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniforms;

@group(0) @binding(1)
var<uniform> model: ModelUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.projection * model.model * vec4<f32>(in.position, 1.0);
    out.uv = in.tex_coord;
    return out;
}

struct GridUniforms {
    camera_pos: vec2<f32>,
    camera_zoom: f32,
    grid_size: f32,
    viewport_size: vec2<f32>,
    line_thickness: f32,
    _pad: f32,
    line_color: vec4<f32>,
}

@group(1) @binding(0)
var<uniform> params: GridUniforms;

fn srgb_to_linear(c: f32) -> f32 {
    if (c <= 0.04045) {
        return c / 12.92;
    }
    return pow((c + 0.055) / 1.055, 2.4);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let aspect = params.viewport_size.x / params.viewport_size.y;
    let visible_half_height = 1.0 / params.camera_zoom;
    let visible_half_width = aspect / params.camera_zoom;

    let world_offset = vec2<f32>(
        (in.uv.x - 0.5) * 2.0 * visible_half_width,
        (in.uv.y - 0.5) * 2.0 * visible_half_height
    );
    let world_pos = world_offset + params.camera_pos;

    let grid_coord = world_pos / params.grid_size;
    let frac_coord = fract(grid_coord + 0.5) - 0.5;
    let dist_to_line = abs(frac_coord) * params.grid_size;

    let min_dist = min(dist_to_line.x, dist_to_line.y);

    let world_thickness = params.line_thickness / params.camera_zoom;

    let half_thickness = world_thickness * 0.5;
    let alpha = 1.0 - smoothstep(0.0, half_thickness, min_dist);

    return vec4<f32>(
        srgb_to_linear(params.line_color.r),
        srgb_to_linear(params.line_color.g),
        srgb_to_linear(params.line_color.b),
        params.line_color.a * alpha,
    );
}