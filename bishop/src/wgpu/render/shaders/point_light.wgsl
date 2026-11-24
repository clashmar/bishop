// Adds one smooth or pixel-quantized direct light to encoded irradiance.

struct PointLightUniforms {
    center_radius_intensity: vec4<f32>,
    color: vec4<f32>,
    settings: vec4<f32>,
    resolution: vec4<f32>,
    pixel_grid_origin: vec4<f32>,
}

@group(1) @binding(0)
var<uniform> params: PointLightUniforms;

@group(2) @binding(0)
var t_scene: texture_2d<f32>;

@group(2) @binding(1)
var s_scene: sampler;

fn decode_radiance(encoded: vec3<f32>) -> vec3<f32> {
    let bounded = min(encoded, vec3<f32>(0.999));
    return bounded / max(vec3<f32>(1.0) - bounded, vec3<f32>(0.001));
}

fn encode_radiance(radiance: vec3<f32>) -> vec3<f32> {
    let nonnegative = max(radiance, vec3<f32>(0.0));
    return nonnegative / (vec3<f32>(1.0) + nonnegative);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let resolution = max(params.resolution.xy, vec2<f32>(1.0));
    let accumulated = textureSample(t_scene, s_scene, in.uv);
    var sample_position = in.position.xy;
    if params.settings.z > 0.5 {
        let pixel_size = max(params.settings.y, 1.0);
        let grid_position = sample_position - params.pixel_grid_origin.xy;
        let pixel = floor(grid_position / pixel_size) * pixel_size
            + vec2<f32>(pixel_size * 0.5) + params.pixel_grid_origin.xy;
        sample_position = clamp(pixel, vec2<f32>(0.5), resolution - vec2<f32>(0.5));
    }
    let distance_to_light = distance(sample_position, params.center_radius_intensity.xy);
    let radius = params.center_radius_intensity.z;
    let normalized = clamp(1.0 - distance_to_light / max(radius, 0.0001), 0.0, 1.0);
    let falloff = pow(normalized, max(params.settings.x, 0.0));
    let direct_radiance = params.color.rgb * max(params.center_radius_intensity.w, 0.0) * falloff;
    let combined = decode_radiance(accumulated.rgb) + direct_radiance;
    return vec4<f32>(encode_radiance(combined), accumulated.a);
}
