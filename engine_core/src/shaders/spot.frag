#version 100
precision mediump float;

varying vec2 uv;
uniform sampler2D tex;
uniform sampler2D light_mask;

#define MAX_LIGHTS 10

uniform int LightCount; 
uniform vec2 LightPos[MAX_LIGHTS];
uniform vec3 LightColor[MAX_LIGHTS];
uniform float LightIntensity[MAX_LIGHTS];
uniform float LightRadius[MAX_LIGHTS];
uniform float LightSpread[MAX_LIGHTS];
uniform float LightAlpha[MAX_LIGHTS];
uniform float LightBrightness[MAX_LIGHTS];
uniform float ScreenWidth;
uniform float ScreenHeight;
uniform float Darkness;

void main() {
    vec4 base = texture2D(tex, uv);
    vec3 scene = base.rgb;

    float maskVal = texture2D(light_mask, uv).r;
    if (maskVal < 0.01) {
        gl_FragColor = vec4(scene, 0.0);
        return;
    }

    vec2 fragPos = uv * vec2(ScreenWidth, ScreenHeight);
    vec3 result = vec3(0.0);
    float totalMask = 0.0;


    for (int i = 0; i < LightCount; ++i) {
        if (i >= MAX_LIGHTS) break;

        float dist = distance(fragPos, LightPos[i]);
        float mask = 1.0 - smoothstep(LightRadius[i],
                                      LightRadius[i] + LightSpread[i],
                                      dist);
        mask *= LightAlpha[i];

        vec3 tinted = mix(scene, LightColor[i], LightIntensity[i]);
        vec3 lit = mix(scene, tinted, mask);
        lit += LightBrightness[i] * LightColor[i] * mask;

        vec3 contrib = (lit - scene * (1.0 - Darkness)) * mask;

        result += contrib;
        totalMask += mask;
    }

    float normalizedMask = clamp(totalMask, 0.0, 1.0);
    gl_FragColor = vec4(clamp(scene + result, 0.0, 1.0), normalizedMask);
}