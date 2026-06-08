#version 100
precision mediump float;
varying vec2 uv;

uniform sampler2D scene_tex;
uniform sampler2D glow_tex;
uniform sampler2D undarkened_tex;

void main() {
    vec4 scene = texture2D(scene_tex, uv);
    vec4 glow = texture2D(glow_tex, uv);
    vec4 existing = texture2D(undarkened_tex, uv);

    vec3 combinedRGB = mix(scene.rgb, glow.rgb, glow.a);
    float combinedA = max(scene.a, glow.a);
    
    vec4 combined = vec4(combinedRGB, combinedA);

    vec4 outCol = mix(existing, combined, combined.a);

    gl_FragColor = vec4(clamp(outCol.rgb, 0.0, 1.0), combined.a);
}