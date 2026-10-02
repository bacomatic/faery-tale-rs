#version 300 es
// fade_to_black.glsl — the cinematic fade (fade_down / fade_normal / intro zoom).
//
// REFERENCE ONLY: validated by the porting effort, not by this pipeline.
//
// What the game does: fade_down() steps fade_page(i,i,i,FALSE,pagecolors) for i = 100,95,..,0
// and fade_normal() for i = 0,5,..,100, one tick apart (src/fmain2.c:623-629) — 21 steps each.
// The intro zoom calls fade_page(y*2-40, y*2-70, y*2-100, 0, introcolors) with per-channel
// weights (src/fmain.c:2930). With limit = FALSE there are no night floors, no moonlight term
// and no vegetation boost: each channel is simply scaled by its weight, truncated to a 4-bit
// nibble (src/fmain2.c:397-410) — so the fade is quantised, not a smooth multiply.
// The Green Jewel lift (fmain2.c:407) still applies if light_timer happens to be running.
//
// Inputs
//   uBase        RGBA sheet/atlas at full brightness; alpha passed through.
//   uWeight      per-channel weights 0..100 (fade_down/normal: all three equal to i).
//   uLightTimer  Green Jewel active (normally false during fades).
//
// Pseudocode (fade_page, limit = FALSE path):
//   r,g,b = clamp(weights, 0, 100); g2 = 0
//   r1 = rn*16; g1 = gn*16; b1 = bn;  if light_timer and r1 < g1: r1 = g1
//   r1 = r*r1/1600; g1 = g*g1/1600; b1 = b*b1/100
precision highp float;
precision highp int;

in vec2 vUV;
out vec4 fragColor;

uniform sampler2D uBase;
uniform ivec3 uWeight;        // (r, g, b) 0..100
uniform bool  uLightTimer;

int nib(float c) { return int(floor(c * 15.0 + 0.5)); }

void main() {
    vec4 base = texture(uBase, vUV);
    ivec3 w = clamp(uWeight, 0, 100);                       // fmain2.c:388-390, 398-400

    int r1 = nib(base.r) * 16;
    int g1 = nib(base.g) * 16;
    int b1 = nib(base.b);
    if (uLightTimer && r1 < g1) r1 = g1;                    // fmain2.c:407
    r1 = (w.r * r1) / 1600;                                 // fmain2.c:408
    g1 = (w.g * g1) / 1600;                                 // fmain2.c:409
    b1 = (w.b * b1) / 100;                                  // fmain2.c:410 with g2 = 0

    fragColor = vec4(float(r1) / 15.0, float(g1) / 15.0, float(b1) / 15.0, base.a);
}
