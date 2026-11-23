// Applies room ambient color, exposure, and darkness to a scene layer.

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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = textureSample(t_scene, s_scene, in.uv);
    let darkness = clamp(params.settings.x, 0.0, 1.0);
    let exposure = max(params.settings.y, 0.0);
    let ambient = params.color.rgb * exposure * (1.0 - darkness);
    return vec4<f32>(base.rgb * ambient, base.a);
}
