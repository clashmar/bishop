#version 100
precision mediump float;

varying vec2 uv;

uniform sampler2D scene_comp_tex;
uniform sampler2D spot_tex;
uniform sampler2D final_comp_tex;

void main() {
    vec4 scene = texture2D(scene_comp_tex, uv);
    vec4 spot = texture2D(spot_tex, uv);
    vec4 existing = texture2D(final_comp_tex, uv);

    vec4 current = mix(scene, scene + spot, spot.a);

    vec4 outCol = mix(existing, current, current.a);

    gl_FragColor = vec4(clamp(outCol.rgb, 0.0, 1.0), 1.0);
}