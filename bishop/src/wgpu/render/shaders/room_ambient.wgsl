// Initializes a reversible bounded encoding of room ambient irradiance.

struct AmbientUniforms {
    color: vec4<f32>,
    settings: vec4<f32>,
}

@group(1) @binding(0)
var<uniform> params: AmbientUniforms;

@group(2) @binding(0)
var t_scene: texture_2d<f32>;

@group(2) @binding(1)
var s_scene: sampler;

fn encode_radiance(radiance: vec3<f32>) -> vec3<f32> {
    let nonnegative = max(radiance, vec3<f32>(0.0));
    return nonnegative / (vec3<f32>(1.0) + nonnegative);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = textureSample(t_scene, s_scene, in.uv);
    let darkness = clamp(params.settings.x, 0.0, 1.0);
    let exposure = max(params.settings.y, 0.0);
    let ambient = params.color.rgb * exposure * (1.0 - darkness);
    return vec4<f32>(encode_radiance(ambient), base.a);
}
