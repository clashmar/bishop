#version 100
precision mediump float;

#define MAX_LIGHTS 10

varying vec2 uv;

uniform sampler2D scene_tex;
uniform sampler2D tex_mask0;
uniform sampler2D tex_mask1;
uniform sampler2D tex_mask2;
uniform sampler2D tex_mask3;
uniform sampler2D tex_mask4;
uniform sampler2D tex_mask5;
uniform sampler2D tex_mask6;
uniform sampler2D tex_mask7;
uniform sampler2D tex_mask8;

uniform int GlowCount; 
uniform float Brightness[MAX_LIGHTS];
uniform float Intensity[MAX_LIGHTS];
uniform vec3 Color[MAX_LIGHTS];
uniform float Emission[MAX_LIGHTS];            
uniform vec2 maskPos[MAX_LIGHTS];         
uniform vec2 maskSize[MAX_LIGHTS]; 

uniform float screenWidth;
uniform float screenHeight;

float sampleMask(int i, vec2 uvMask)
{
    // Reject coordinates outside the mask
    if (uvMask.x < 0.0 || uvMask.x > 1.0 ||
        uvMask.y < 0.0 || uvMask.y > 1.0)
        return 0.0;

    if (i == 0) return texture2D(tex_mask0, uvMask).a;
    if (i == 1) return texture2D(tex_mask1, uvMask).a;
    if (i == 2) return texture2D(tex_mask2, uvMask).a;
    if (i == 3) return texture2D(tex_mask3, uvMask).a;
    if (i == 4) return texture2D(tex_mask4, uvMask).a;
    if (i == 5) return texture2D(tex_mask5, uvMask).a;
    if (i == 6) return texture2D(tex_mask6, uvMask).a;
    if (i == 7) return texture2D(tex_mask7, uvMask).a;
    if (i == 8) return texture2D(tex_mask8, uvMask).a;
    return 0.0;
}

void main()
{
    vec3 baseScene = texture2D(scene_tex, uv).rgb;

    vec2 fragScreen = uv * vec2(screenWidth, screenHeight);
    float finalMask = 0.0;
    vec3 glowAccum = vec3(0.0);

    for (int i = 0; i < GlowCount; ++i) {
        if (i >= MAX_LIGHTS) break;

        vec2 rel = (fragScreen - maskPos[i]) / maskSize[i];
        float c00 = sampleMask(i, rel);

        // 3×3 blur TODO: Improve range and clarity
        vec2 pixelSize = 1.0 / maskSize[i];
        float sum = 0.0;
        sum += sampleMask(i, rel + pixelSize * vec2(-Emission[i], -Emission[i]));
        sum += sampleMask(i, rel + pixelSize * vec2( 0.0, -Emission[i]));
        sum += sampleMask(i, rel + pixelSize * vec2( Emission[i], -Emission[i]));
        sum += sampleMask(i, rel + pixelSize * vec2( Emission[i],  0.0));
        sum += c00;
        sum += sampleMask(i, rel + pixelSize * vec2( Emission[i],  Emission[i]));
        sum += sampleMask(i, rel + pixelSize * vec2( 0.0,  Emission[i]));
        sum += sampleMask(i, rel + pixelSize * vec2(-Emission[i],  Emission[i]));
        sum += sampleMask(i, rel + pixelSize * vec2(-Emission[i],  0.0));
        float avg = sum / 9.0;

        float blurred = mix(c00, avg, clamp(Emission[i],0.0,1.0));
        finalMask = max(finalMask, blurred);

        vec3 glowColor = Color[i] * Brightness[i] * blurred;

        vec3 tinted = mix(baseScene, Color[i], Intensity[i] * blurred);
        vec3 tintContribution = (tinted - baseScene) * blurred;

        glowAccum += glowColor + tintContribution;
    }

    vec3 outRGB = glowAccum;               
    float outA = finalMask;

    gl_FragColor = vec4(clamp(outRGB,0.0,1.0), outA);
}