#version 300 es
// daynight_live.glsl — the complete outdoor day/night palette effect, live.
//
// REFERENCE ONLY: validated by the porting effort, not by this pipeline. The bit-exact ground
// truth is fade_page() (src/fmain2.c:377-419) as ported in experiment/shaders/fade_page.py; the
// renders in previews/ come from that port. The review app runs this shader verbatim in WebGL2
// and diffs it against those renders (see README.md).
//
// What the game does: every 4 ticks day_fade() (src/fmain2.c:1653-1660) calls
//     fade_page(lightlevel-80+ll, lightlevel-61, lightlevel-62, TRUE, pagecolors)
// with ll = 200 while the Green Jewel's light_timer runs, else 0. fade_page rewrites the 32-entry
// hardware palette; because every pixel of a prebaked RGBA sheet is palette nibble n expanded to
// n*17 (asset_common.rgb4_to_rgba8), the nibble is recoverable exactly and the same integer math
// can run per pixel. The only information baking loses is the palette INDEX, which the
// vegetation boost needs (indices 16..24); the 1-bit highlight mask shipped with every sheet
// and atlas restores exactly that bit.
//
// Inputs
//   uBase        RGBA sheet/atlas at full brightness (lightlevel >= 180 == original palette).
//                Sprites: index 31 is alpha 0; the alpha is passed through untouched.
//                Colour 31 on tile atlases is already the per-region value (fmain2.c:381-386).
//   uHighlight   1-bit highlight mask: .r >= 0.5 where the source index was 16..24.
//   uLightlevel  0..300, lightlevel = daynight/40 mirrored (src/fmain.c:2025-2026).
//                Indoors (regions 8-9) day_fade passes (100,100,100): use any value >= 180.
//   uLightTimer  true while the Green Jewel is active (src/fmain.c:3306, decrement :1380).
//
// Pseudocode (fade_page, limit = TRUE path):
//   r = clamp(lightlevel-80+ll, 10, 100); g = clamp(lightlevel-61, 25, 100); b = clamp(lightlevel-62, 60, 100)
//   g2 = (100-g)/3                                           // moonlight coefficient, 25 at night, 0 by day
//   for each pixel with nibbles rn,gn,bn (0..15):
//     r1 = rn*16; g1 = gn*16; b1 = bn
//     if light_timer and r1 < g1: r1 = g1                    // jewel: lift red to green
//     r1 = r*r1/1600; g1 = g*g1/1600; b1 = (b*b1 + g2*g1)/100
//     if index in 16..24 and g > 20: b1 += (g < 50 ? 2 : g < 75 ? 1 : 0)   // vegetation night boost
//     b1 = min(b1, 15)
//   out = (r1, g1, b1) / 15, alpha unchanged
precision highp float;
precision highp int;

in vec2 vUV;
out vec4 fragColor;

uniform sampler2D uBase;
uniform sampler2D uHighlight;
uniform int  uLightlevel;    // 0..300
uniform bool uLightTimer;

int nib(float c) { return int(floor(c * 15.0 + 0.5)); }   // n*17/255 -> n, exact

void main() {
    vec4 base = texture(uBase, vUV);
    bool highlight = texture(uHighlight, vUV).r >= 0.5;

    // day_fade() weights (fmain2.c:1655-1658), then fade_page() clamp + night floors (:388-395)
    int ll = uLightTimer ? 200 : 0;
    int r = min(uLightlevel - 80 + ll, 100);
    int g = min(uLightlevel - 61, 100);
    int b = min(uLightlevel - 62, 100);
    r = max(r, 10); g = max(g, 25); b = max(b, 60);
    int g2 = (100 - g) / 3;

    // per-colour loop body (fmain2.c:404-416), on the recovered nibbles
    int r1 = nib(base.r) * 16;      // (colors & 0x0f00) >> 4
    int g1 = nib(base.g) * 16;      // colors & 0x00f0
    int b1 = nib(base.b);           // colors & 0x000f
    if (uLightTimer && r1 < g1) r1 = g1;
    r1 = (r * r1) / 1600;
    g1 = (g * g1) / 1600;
    b1 = (b * b1 + g2 * g1) / 100;
    if (highlight && g > 20) {
        if (g < 50) b1 += 2; else if (g < 75) b1 += 1;
    }
    b1 = min(b1, 15);

    fragColor = vec4(float(r1) / 15.0, float(g1) / 15.0, float(b1) / 15.0, base.a);
}
