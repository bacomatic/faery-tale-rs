// daynight_live.glsl
// ---------------------------------------------------------------------------
// THE HEAD-ON REBUTTAL to assets/plan.md:119-121 ("the vegetation night boost
// on palette indices 16-24 is NOT shader-doable on RGBA").
//
// It IS shader-doable. This single fragment shader reproduces the ENTIRE original
// fade_page() day/night cycle (src/fmain2.c:377-419) bit-exactly, at any continuous
// light level, from prebaked RGBA -- given one extra 1-bit "is-highlight" channel.
//
// Why it works:
//   * The original effect is a deterministic integer function of the light level
//     and the source palette nibbles. Nothing is lost that the math needs...
//   * ...except the palette INDEX, which the claim correctly notes is gone after
//     baking. We restore exactly the one bit the boost needs (index in 16..24) via
//     a precomputed highlightMask. That is the whole trick.
//
// Inputs:
//   baseFullBright : RGBA at full brightness (level 180). Its 8-bit channels are
//                    n*17, so the original 4-bit Amiga nibbles are recoverable
//                    exactly as round(channel*15).
//   highlightMask        : .r >= 0.5 where the source pixel was palette index 16..24.
//   uLightlevel    : 0..300 day/night phase (see day_fade(), fmain2.c:1653).
//                    Outdoor params: r=ll-80, g=ll-61, b=ll-62.
//
// All math below is integer and mirrors fade_page() line-for-line. (The torch /
// green-jewel branch is intentionally omitted -- out of scope for this experiment.)
// ---------------------------------------------------------------------------
#version 330 core
in  vec2 vUV;
out vec4 fragColor;

uniform sampler2D baseFullBright;
uniform sampler2D highlightMask;
uniform int       uLightlevel;   // 0..300

int nib(float c) { return int(floor(c * 15.0 + 0.5)); }   // n*17/255 -> n, exact

void main() {
    vec4 base = texture(baseFullBright, vUV);
    bool highlight = texture(highlightMask, vUV).r >= 0.5;

    // --- day_fade() outdoor params, then fade_page() clamps/floors (limit=TRUE) ---
    int r = uLightlevel - 80;
    int g = uLightlevel - 61;
    int b = uLightlevel - 62;
    r = min(r, 100); g = min(g, 100); b = min(b, 100);
    r = max(r, 10);  g = max(g, 25);  b = max(b, 60);     // night limits
    int g2 = (100 - g) / 3;                                // moonlight coefficient

    // --- recover original palette nibbles from the full-bright RGBA ---
    int rn = nib(base.r);
    int gn = nib(base.g);
    int bn = nib(base.b);

    int r1 = rn * 16;     // (colors & 0x0f00) >> 4
    int g1 = gn * 16;     // (colors & 0x00f0)
    int b1 = bn;          // (colors & 0x000f)

    r1 = (r * r1) / 1600;                 // -> 0..15
    g1 = (g * g1) / 1600;                 // -> 0..15
    b1 = (b * b1 + (g2 * g1)) / 100;      // moonlight blue injection

    // --- the contested vegetation night boost, gated by the mask ---
    if (highlight && g > 20) {
        if (g < 50)      b1 += 2;
        else if (g < 75) b1 += 1;
    }
    b1 = min(b1, 15);

    // 4-bit channels -> normalized float (n/15 == expand4(n)/255), preserve source alpha.
    fragColor = vec4(float(r1) / 15.0, float(g1) / 15.0, float(b1) / 15.0, base.a);
}
