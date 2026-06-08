#version 100
precision mediump float;

varying vec2 uv;
uniform sampler2D tex;
uniform float Darkness;

void main() {
    vec4 base = texture2D(tex, uv);
    vec3 scene = base.rgb;

    vec3 darkened = mix(scene, vec3(0.0), Darkness);

    gl_FragColor = vec4(clamp(darkened, 0.0, 1.0), base.a);
}