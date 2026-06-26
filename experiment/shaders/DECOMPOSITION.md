# Day/Night Cycle — Full Decomposition

The entire Faery Tale day/night cycle is produced by **one** function that rewrites
the 32-entry hardware palette every few frames: `fade_page()` in `src/fmain2.c:377-419`,
driven by `day_fade()` at `src/fmain2.c:1653-1660`. `fade_page.py` in this directory is a
verbatim integer port; it is the reference oracle for the whole experiment.

## 1. Driver: light level → params

`daynight` is a free-running counter; `lightlevel = daynight/40`, mirrored to a 0..300
triangle (`if (lightlevel >= 300) lightlevel = 600 - lightlevel`). Outdoors (`region_num < 8`)
`day_fade()` calls:

```
fade_page(lightlevel-80+ll,  lightlevel-61,  lightlevel-62,  TRUE,  pagecolors)
            r                  g               b              limit
```

`ll = 200` only when a torch/light spell is active (`light_timer`); **out of scope here**, so
`ll = 0` and `r = ll-80, g = ll-61, b = ll-62`.

## 2. `fade_page()` per-entry math (exact)

```c
clamp r,g,b <= 100
if (limit) { if(r<10)r=10; if(g<25)g=25; if(b<60)b=60; g2=(100-g)/3; }   // night floors
else       { clamp >=0; g2=0; }
for i in 0..31:
    r1 = (colors[i] & 0x0f00) >> 4;   // red nibble * 16
    g1 =  colors[i] & 0x00f0;          // green nibble * 16
    b1 =  colors[i] & 0x000f;          // blue nibble
    if (light_timer && r1 < g1) r1 = g1;        // green jewel (skipped)
    r1 = (r * r1) / 1600;              // -> 0..15   (integer truncation)
    g1 = (g * g1) / 1600;              // -> 0..15
    b1 = (b * b1 + g2*g1) / 100;       // moonlight blue injection
    if (limit) {
        if (16<=i<=24 && g>20) { if(g<50) b1+=2; else if(g<75) b1+=1; }  // VEG BOOST
        if (b1>15) b1=15;
    }
    fader[i] = (r1<<8)+(g1<<4)+b1;     // 12-bit Amiga color
```

Three distinct effects live in this one loop:

- **Brightness dim** — `(param * nibble*16)/1600` scales each channel by the light param.
- **Moonlight blue** — `g2 = (100-g)/3` injects blue derived from the (scaled) green channel.
  At night `g` floors to 25 → `g2 = 25`, a strong blue cast; by day `g→100` → `g2 = 0`.
- **Vegetation night boost** — for palette indices **16–24 only**, adds 1–2 to the blue nibble
  during twilight. This is the effect `assets/plan.md:119-121` claims cannot be done on RGBA.

## 3. Why the index *is* recoverable (the crux)

At full bright `r=g=b=100`, `limit` true, `g2=0`:
`r1=(100*nibble*16)/1600 = nibble`, `g1=nibble`, `b1=(100*nibble)/100=nibble`. So the
full-bright palette **equals the original nibbles**, and 8-bit baking uses
`expand4(n) = n*17`. Therefore `channel // 17 == original nibble`, exactly. The live shader
recovers every value `fade_page()` needs from the full-bright RGBA — **except** which pixels
were indices 16–24. That single bit is supplied by a precomputed `highlight_mask`. Nothing else is lost.

## 4. Light-level bands (and why these levels were chosen)

`g = lightlevel - 61`, floored to 25. The veg boost depends entirely on `g`:

| lightlevel | g (after floor) | veg boost on idx 16–24 | notes |
|-----------:|----------------:|-----------------------:|-------|
| 0 – 86     | 25              | **+2** (g in 20..50)   | deep-night plateau (all floors pinned; identical frame) |
| 95         | 34              | +2                     | twilight |
| 105        | 44              | +2                     | |
| **111**    | 50              | **+1** (g in 50..75)   | boost step-down |
| 120        | 59              | +1                     | |
| **136**    | 75              | **0** (g ≥ 75)         | boost ends |
| 150        | 89              | 0                      | |
| 165        | 104→100         | 0                      | |
| 180        | 119→100         | 0                      | full day (= original palette) |

`compare.py` confirms these bands empirically: the per-level "highlight px boosted / max Δ" column
shows Δ34 (=2 nibble steps ×17) for levels 0–105, Δ17 for 111–120, and 0 from 136 on.

The canonical set `[0, 86, 95, 105, 111, 120, 136, 150, 165, 180]` therefore captures full dark,
full bright, and every boundary where the cycle's behavior actually changes.

## 5. The vegetation palette (indices 16–24)

`pagecolors[16..24]` = `0x0040, 0x0070, 0x00B0, 0x06F6, 0x0005, 0x0009, 0x000D, 0x037F, 0x0C00` —
the foliage greens (16–19), water/shadow blues (20–22), cyan (23) and a dark red (24). These are
the colors the night boost nudges bluer, which is why the F8 "forest and wilderness" region
(dense vegetation) is the terrain subject for this experiment.
