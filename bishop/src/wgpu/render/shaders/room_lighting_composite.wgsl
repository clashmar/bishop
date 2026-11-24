// Combines albedo and accumulated irradiance, then applies a soft highlight shoulder.

@group(2) @binding(0)
var t_albedo: texture_2d<f32>;

@group(2) @binding(1)
var s_albedo: sampler;

@group(2) @binding(2)
var t_illumination: texture_2d<f32>;

@group(2) @binding(3)
var s_illumination: sampler;

fn decode_radiance(encoded: vec3<f32>) -> vec3<f32> {
    let bounded = min(encoded, vec3<f32>(0.999));
    return bounded / max(vec3<f32>(1.0) - bounded, vec3<f32>(0.001));
}

fn soft_knee_tone_map(radiance: vec3<f32>) -> vec3<f32> {
    let knee = vec3<f32>(0.75);
    let shoulder = max(radiance - knee, vec3<f32>(0.0));
    return min(radiance, knee) + vec3<f32>(0.25) * (vec3<f32>(1.0) - exp(-shoulder / vec3<f32>(0.25)));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let albedo = textureSample(t_albedo, s_albedo, in.uv);
    let encoded_illumination = textureSample(t_illumination, s_illumination, in.uv).rgb;
    let illumination = decode_radiance(encoded_illumination);
    let mapped = soft_knee_tone_map(albedo.rgb * illumination);
    return vec4<f32>(mapped, albedo.a);
}
