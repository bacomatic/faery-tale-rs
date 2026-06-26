// daynight_bank.glsl
// ---------------------------------------------------------------------------
// The SECOND, even simpler rebuttal to assets/plan.md:119-121.
//
// Instead of computing the cycle live, we prebake one RGBA frame per light level
// (extract_experiment.py -> frames/<subject>/L<level>/). The vegetation night
// boost -- and every other part of fade_page() -- is already in those texels, so
// at runtime there is NOTHING to compute and NO palette index is needed at all.
// The shader just samples the bank and (optionally) cross-fades between the two
// nearest baked levels for smooth time-of-day.
//
// This is the texture-array / 3D-LUT interpretation of the day/night cycle:
// the original palette LUT lives in the bank, the "indexing" already happened
// at bake time. The claim that the boost "must use the indexed atlas + a palette
// LUT" at runtime is exactly what this sidesteps.
//
// Inputs:
//   bank       : sampler2DArray, one baked layer per CANONICAL level (see
//                fade_page.py CANONICAL_LEVELS). uLayerCount layers.
//   uLevel01   : 0..1 normalized position across the baked levels (0 = darkest
//                layer, 1 = brightest). Drives selection + interpolation.
//   uLayerCount: number of baked levels in the bank.
// ---------------------------------------------------------------------------
#version 330 core
in  vec2 vUV;
out vec4 fragColor;

uniform sampler2DArray bank;
uniform float          uLevel01;     // 0..1 across baked levels
uniform int            uLayerCount;

void main() {
    float f  = clamp(uLevel01, 0.0, 1.0) * float(uLayerCount - 1);
    int   lo = int(floor(f));
    int   hi = min(lo + 1, uLayerCount - 1);
    float t  = f - float(lo);

    vec4 a = texture(bank, vec3(vUV, float(lo)));
    vec4 b = texture(bank, vec3(vUV, float(hi)));

    // For pixel-exact reproduction of a specific baked level, set uLevel01 to that
    // layer (t == 0) and this returns the bank texel verbatim. The lerp only adds
    // smooth in-between frames the original integer palette never produced.
    fragColor = mix(a, b, t);
}
