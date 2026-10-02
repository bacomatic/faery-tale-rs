# Light-level reference renders

Non-authoritative preview artifacts (`assets/tasks/_SHARED.md` → Previews) that double as golden
images for the port: every region tile atlas and every actor sheet as `fade_page()`
(`src/fmain2.c:377-419`) colours it at a spread of light levels. Rendered by
`tools/render_light_levels.py` with the **bit-exact Python port** `experiment/shaders/fade_page.py`
— **not** with the GLSL in the parent directory (the GLSL is checked against these).

## Light levels

`day_fade()` (`src/fmain2.c:1653-1660`) calls `fade_page(lightlevel-80, lightlevel-61,
lightlevel-62, TRUE, pagecolors)` outdoors. The levels rendered are the points where the
cycle's behaviour changes:

| lightlevel | what it is |
|---:|---|
| 0 | deep night — every weight on its floor (r 10, g 25, b 60); identical for lightlevel 0..86 |
| 95, 105 | twilight, vegetation boost +2 |
| 111, 120 | boost steps down to +1 |
| 136, 150, 165 | boost over; brightening |
| 180 | full day — weights 100, equals the original palette (and the shipped atlas/sheet) |
| jewel, 0 | deep night with the Green Jewel (`light_timer`): red weight 100, red lifted to green (`src/fmain2.c:1655, 407`) |

Regions 8 and 9 (indoors) get **one** render, `light180`: `day_fade` passes `(100,100,100)`
inside (`src/fmain2.c:1657-1659`), so there is no night and no jewel variant to show.

## Files

- `<subject>_light<level>.png` — one RGBA render per level; `<subject>_jewel_light000.png`.
- `<subject>_strip.png` — all renders of the subject side by side with labels (review aid).
- `previews.json` — index: per subject the source indexed PNG and highlight mask under `assets/`,
  the levels and the file per level. The review app's shader viewer uses it as the per-level
  bank and for diffs.

Subjects: `region_00` … `region_09` (from `tiles/region_NN/atlas_indexed.png`, colour 31 =
the region's own value) and the **17 actor sheets that appear outdoors** (index 31 transparent):
heroes `julian phillip kevin`; enemies `ogre orcs wraith skeleton snake spider dknight`; NPCs
`begger ranger spectre ghost`; carriers `bird raft turtle`. Loraii, the necromancer and the
woodcutter (astral plane), the dragon (his cave) and the other setfig NPCs (buildings) only
appear where `day_fade` passes `(100,100,100)`, so they are never faded and are not rendered
(T3.1 review decision).
