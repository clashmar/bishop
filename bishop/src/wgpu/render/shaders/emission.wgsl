// Adds sprite-alpha emission and a normalized halo to encoded irradiance.

struct EmissionUniforms {
    color: vec4<f32>,
    settings: vec4<f32>,
    resolution: vec4<f32>,
    pixel_grid_origin: vec4<f32>,
}

@group(1) @binding(0)
var<uniform> params: EmissionUniforms;

@group(2) @binding(0)
var t_mask: texture_2d<f32>;

@group(2) @binding(1)
var s_mask: sampler;

@group(2) @binding(2)
var t_illumination: texture_2d<f32>;

@group(2) @binding(3)
var s_illumination: sampler;

fn decode_radiance(encoded: vec3<f32>) -> vec3<f32> {
    let bounded = min(encoded, vec3<f32>(0.999));
    return bounded / max(vec3<f32>(1.0) - bounded, vec3<f32>(0.001));
}

fn encode_radiance(radiance: vec3<f32>) -> vec3<f32> {
    let nonnegative = max(radiance, vec3<f32>(0.0));
    return nonnegative / (vec3<f32>(1.0) + nonnegative);
}

fn quantize_mask_position(position: vec2<f32>) -> vec2<f32> {
    let pixel_size = max(params.settings.z, 1.0);
    let grid_position = position - params.pixel_grid_origin.xy;
    let cell = floor(grid_position / pixel_size);
    return cell * pixel_size + vec2<f32>(pixel_size * 0.5) + params.pixel_grid_origin.xy;
}

fn mask_alpha_at(position: vec2<f32>, resolution: vec2<f32>) -> f32 {
    if any(position < vec2<f32>(0.5)) || any(position > resolution - vec2<f32>(0.5)) {
        return 0.0;
    }
    let uv = position / resolution;
    return textureSampleLevel(t_mask, s_mask, uv, 0.0).a;
}

fn sampled_mask_alpha(position: vec2<f32>, resolution: vec2<f32>) -> f32 {
    var sample_position = position;
    if params.settings.w > 0.5 {
        sample_position = quantize_mask_position(sample_position);
    }
    return mask_alpha_at(sample_position, resolution);
}

fn halo_alpha_at(position: vec2<f32>, resolution: vec2<f32>) -> f32 {
    let spread = params.settings.y;
    if spread <= 0.0 {
        return 0.0;
    }

    var weighted_alpha = 0.0;
    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let weight_x = select(1.0, 2.0, x == 0);
            let weight_y = select(1.0, 2.0, y == 0);
            let offset = vec2<f32>(f32(x), f32(y)) * spread;
            weighted_alpha += sampled_mask_alpha(position + offset, resolution) * weight_x * weight_y;
        }
    }
    return weighted_alpha / 16.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let resolution = max(params.resolution.xy, vec2<f32>(1.0));
    let accumulated = textureSample(t_illumination, s_illumination, in.uv);
    let source_alpha = sampled_mask_alpha(in.position.xy, resolution);
    let halo_alpha = halo_alpha_at(in.position.xy, resolution);
    let emission = params.color.rgb * max(params.settings.x, 0.0) * (source_alpha + halo_alpha);
    let combined = decode_radiance(accumulated.rgb) + emission;
    return vec4<f32>(encode_radiance(combined), accumulated.a);
}
