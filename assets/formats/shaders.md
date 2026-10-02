# Reference shaders and light-level renders

Directory: `shaders/`. Not game data: reference material for a port that ships the RGBA assets
and wants the original palette effects. **The GLSL is reference-only and is validated by the
porting effort, not by this bundle's pipeline.** Full description: [`shaders/README.md`](../shaders/README.md).

```
shaders/
  daynight_live.glsl   complete outdoor day/night (brightness, night floors, moonlight, vegetation boost, Green Jewel)
  daynight_bank.glsl   the same from a prebaked per-level texture array (the previews)
  fade_to_black.glsl   fade_down / fade_normal / intro per-channel fade
  shaders.json         textures + uniforms per shader (machine-readable)
  README.md            effect list, driving values, indexed-vs-RGBA matrix
  previews/            light-level reference renders + previews.json + README.md
```

- Shaders are GLSL ES 3.00 fragment shaders (`#version 300 es`), runnable verbatim in WebGL2;
  they expect a full-screen quad with `vUV` (0,0) at the top-left texel, NEAREST sampling, no
  premultiplied alpha. Alpha passes through (sprite index 31 is alpha 0).
- `shaders.json`: `{note, vertex, textures: {indexed, highlight_mask, bank}, shaders: {"<file>": {title,
  source[], textures: {uniform: kind}, uniforms: [{name, type, label, min, max, step?, default}],
  modes?: [{id, label, rule?, param?: {label, min, max, step, default}}], compare?}}, compare: {...}}` —
  `type` ∈ `int | float | bool | ivec3`; `modes` are the game's ways of driving the uniforms (for
  `fade_to_black`: `fade` = `(i,i,i)`, `intro_zoom` = `screen_size(x)` weights, `free`); `compare`
  names which preview render the shader must reproduce exactly for given uniform values.
- `previews/<subject>_light<level>.png`, `<subject>_jewel_light000.png`, `<subject>_strip.png`:
  RGBA renders of every region atlas and every actor sheet that appears outdoors (17) at light
  levels 0, 95, 105, 111, 120, 136, 150, 165, 180 and with the Green Jewel at night, computed with the bit-exact Python port of
  `fade_page()` (`experiment/shaders/fade_page.py` in the repo) — golden images, not the GLSL.
  Indoor regions 8/9 have only `light180`. `previews/previews.json`: `{source, renderer, levels[],
  jewel_level, indoor_regions[], subjects: [{id, kind, region, source: {indexed, highlight_mask},
  levels[], files: {"<level>": file, "jewel": file}, strip}]}`.

What each effect needs on the RGBA path, and why the vegetation boost needs the 1-bit
`highlight_mask`, is the palette-effects matrix in [palettes.md](palettes.md).
