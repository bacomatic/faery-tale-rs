#version 300 es
// daynight_bank.glsl — day/night from a prebaked per-light-level bank.
//
// REFERENCE ONLY: validated by the porting effort, not by this pipeline.
//
// The alternative to daynight_live.glsl: bake one RGBA frame per light level (the files in
// previews/ are exactly that bank, produced by the bit-exact fade_page() port) and sample the
// layer at run time. The vegetation boost and every other fade_page() term are already in the
// texels, so no highlight mask and no palette index are needed. The lerp between neighbouring
// layers is an optional smoothing the original integer palette never produced; with uLevel01
// exactly on a layer (t == 0) the output is the baked texel verbatim.
//
// Inputs
//   uBank        sampler2DArray, one layer per baked light level in ascending order
//                (previews.json "levels": 0, 95, 105, 111, 120, 136, 150, 165, 180).
//   uLevel01     0..1 position across the layers (0 = darkest, 1 = brightest).
//   uLayerCount  number of layers in uBank.
//
// Pseudocode:
//   f = uLevel01 * (layers-1); lo = floor(f); hi = min(lo+1, layers-1); t = f - lo
//   out = mix(bank[lo], bank[hi], t)
precision highp float;
precision highp int;
precision highp sampler2DArray;

in vec2 vUV;
out vec4 fragColor;

uniform sampler2DArray uBank;
uniform float uLevel01;
uniform int   uLayerCount;

void main() {
    float f  = clamp(uLevel01, 0.0, 1.0) * float(uLayerCount - 1);
    int   lo = int(floor(f));
    int   hi = min(lo + 1, uLayerCount - 1);
    float t  = f - float(lo);
    vec4 a = texture(uBank, vec3(vUV, float(lo)));
    vec4 b = texture(uBank, vec3(vUV, float(hi)));
    fragColor = mix(a, b, t);
}
