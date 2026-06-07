# Actor Sheets Atlas (PHIL + ENEMY)

> PHIL sheet: `seq_list` slot 0 (`PHIL`). Source: `fmain2.c:646-648` (`cfiles[0-2]`).
> Size: 16×32 px per frame. Frame count: 67 (all three brothers share the same layout).
>
> ENEMY sheet: `seq_list` slot 2 (`ENEMY`). Source: `fmain2.c:653-660` (`cfiles[6-9,12]`).
> Size: 16×32 px per frame. Frame count: 64 for all ENEMY files.
>
> Both sheets share the same animation indexing scheme: the game selects a body frame
> via `statelist[inum].figure` (PHIL) or `inum + snake_offset` (ENEMY), not by `inum`
> directly. See `logic/sprite-rendering.md § select_atype_inum`.

## PHIL Sheet — Frame Table

`figure` values from `statelist[87]` (`fmain.c:154-204`). Walk and fight base frames
selected by `diroffs[16]` (`fmain.c:1010`). Full `statelist` table in
`logic/sprite-rendering.md § statelist[87]`.

| Frame | Name / description | Size (px) | Source ref |
|-------|--------------------|-----------|------------|
| 0 | Walk S — step 0 | 16×32 | `fmain.c:154` `statelist[0].figure` |
| 1 | Walk S — step 1 | 16×32 | `fmain.c:154` `statelist[1].figure` |
| 2 | Walk S — step 2 | 16×32 | `fmain.c:154` `statelist[2].figure` |
| 3 | Walk S — step 3 | 16×32 | `fmain.c:154` `statelist[3].figure` |
| 4 | Walk S — step 4 | 16×32 | `fmain.c:154` `statelist[4].figure` |
| 5 | Walk S — step 5 | 16×32 | `fmain.c:154` `statelist[5].figure` |
| 6 | Walk S — step 6 | 16×32 | `fmain.c:154` `statelist[6].figure` |
| 7 | Walk S — step 7 | 16×32 | `fmain.c:154` `statelist[7].figure` |
| 8 | Walk W — step 0 | 16×32 | `fmain.c:154` `statelist[8].figure` |
| 9 | Walk W — step 1 | 16×32 | `fmain.c:154` `statelist[9].figure` |
| 10 | Walk W — step 2 | 16×32 | `fmain.c:154` `statelist[10].figure` |
| 11 | Walk W — step 3 | 16×32 | `fmain.c:154` `statelist[11].figure` |
| 12 | Walk W — step 4 | 16×32 | `fmain.c:154` `statelist[12].figure` |
| 13 | Walk W — step 5 | 16×32 | `fmain.c:154` `statelist[13].figure` |
| 14 | Walk W — step 6 | 16×32 | `fmain.c:154` `statelist[14].figure` |
| 15 | Walk W — step 7 | 16×32 | `fmain.c:154` `statelist[15].figure` |
| 16 | Walk N — step 0 | 16×32 | `fmain.c:154` `statelist[16].figure` |
| 17 | Walk N — step 1 | 16×32 | `fmain.c:154` `statelist[17].figure` |
| 18 | Walk N — step 2 | 16×32 | `fmain.c:154` `statelist[18].figure` |
| 19 | Walk N — step 3 | 16×32 | `fmain.c:154` `statelist[19].figure` |
| 20 | Walk N — step 4 | 16×32 | `fmain.c:154` `statelist[20].figure` |
| 21 | Walk N — step 5 | 16×32 | `fmain.c:154` `statelist[21].figure` |
| 22 | Walk N — step 6 | 16×32 | `fmain.c:154` `statelist[22].figure` |
| 23 | Walk N — step 7 | 16×32 | `fmain.c:154` `statelist[23].figure` |
| 24 | Walk E — step 0 | 16×32 | `fmain.c:154` `statelist[24].figure` |
| 25 | Walk E — step 1 | 16×32 | `fmain.c:154` `statelist[25].figure` |
| 26 | Walk E — step 2 | 16×32 | `fmain.c:154` `statelist[26].figure` |
| 27 | Walk E — step 3 | 16×32 | `fmain.c:154` `statelist[27].figure` |
| 28 | Walk E — step 4 | 16×32 | `fmain.c:154` `statelist[28].figure` |
| 29 | Walk E — step 5 | 16×32 | `fmain.c:154` `statelist[29].figure` |
| 30 | Walk E — step 6 | 16×32 | `fmain.c:154` `statelist[30].figure` |
| 31 | Walk E — step 7 | 16×32 | `fmain.c:154` `statelist[31].figure` |
| 32 | Fight S — swing 0 | 16×32 | `fmain.c:154` `statelist[32].figure` |
| 33 | Fight S — swing 2 | 16×32 | `fmain.c:154` `statelist[34].figure` |
| 34 | Fight S — swing 3–5 | 16×32 | `fmain.c:154` `statelist[34].figure` |
| 35 | Fight S — weapon raised (frustration pose) | 16×32 | `fmain.c:154` `statelist[40].figure`; `fmain.c:1657` |
| 36 | Fight S — swing 6–7,9 | 16×32 | `fmain.c:154` `statelist[36].figure` |
| 37 | Fight S — ranged frame B | 16×32 | `fmain.c:154` `statelist[43].figure` |
| 38 | Fight S — ranged frame A | 16×32 | `fmain.c:154` `statelist[42].figure` |
| 39 | Death sequence frame C | 16×32 | `fmain.c:154` `statelist[82].figure` |
| 40 | Fight W — swing 0–1 | 16×32 | `fmain.c:154` `statelist[44].figure` |
| 41 | Fight W — swing 2 | 16×32 | `fmain.c:154` `statelist[46].figure` |
| 42 | Fight W — swing 3–5,9 | 16×32 | `fmain.c:154` `statelist[47].figure` |
| 43 | Fight W — swing 8 | 16×32 | `fmain.c:154` `statelist[51].figure` |
| 44 | Fight W — swing 6–7 | 16×32 | `fmain.c:154` `statelist[49].figure` |
| 45 | Fight W — ranged frame B | 16×32 | `fmain.c:154` `statelist[55].figure` |
| 46 | Fight W — ranged frame A | 16×32 | `fmain.c:154` `statelist[54].figure` |
| 47 | Death sequence frame A | 16×32 | `fmain.c:154` `statelist[80].figure` |
| 48 | Fight N — swing 0–1 | 16×32 | `fmain.c:154` `statelist[56].figure` |
| 49 | Fight N — swing 2 | 16×32 | `fmain.c:154` `statelist[58].figure` |
| 50 | Fight N — swing 3–5,9 | 16×32 | `fmain.c:154` `statelist[59].figure` |
| 51 | Fight N — swing 8 | 16×32 | `fmain.c:154` `statelist[67].figure` |
| 52 | Fight N — swing 6–7 | 16×32 | `fmain.c:154` `statelist[62].figure` |
| 53 | Fight N — ranged frame B | 16×32 | `fmain.c:154` `statelist[67].figure` |
| 54 | Fight N — ranged frame A | 16×32 | `fmain.c:154` `statelist[66].figure` |
| 55 | Sinking (drowning/deep water) | 16×32 | `fmain.c:154` `statelist[83].figure` |
| 56 | Fight E — swing 0–1 | 16×32 | `fmain.c:154` `statelist[68].figure` |
| 57 | Fight E — swing 2 | 16×32 | `fmain.c:154` `statelist[70].figure` |
| 58 | Fight E — swing 3–5,9 | 16×32 | `fmain.c:154` `statelist[71].figure` |
| 59 | Fight E — swing 8 | 16×32 | `fmain.c:154` `statelist[76].figure` |
| 60 | Fight E — swing 6–7 | 16×32 | `fmain.c:154` `statelist[74].figure` |
| 61 | Fight E — ranged frame B | 16×32 | `fmain.c:154` `statelist[79].figure` |
| 62 | Fight E — ranged frame A | 16×32 | `fmain.c:154` `statelist[78].figure` |
| 63 | Death sequence frame B | 16×32 | `fmain.c:154` `statelist[81].figure` |
| 64 | Frustration / sword-at-side A | 16×32 | `fmain.c:154` `statelist[84].figure` |
| 65 | Frustration / sword-at-side B | 16×32 | `fmain.c:154` `statelist[85].figure` |
| 66 | Asleep | 16×32 | `fmain.c:154` `statelist[86].figure` |

## ENEMY Sheet — Frame Table

All ENEMY files (`cfiles[6-9, 12]`) share the same 64-frame layout. The snake/salamander
(`cfiles[12]`) is special: its frames are accessed as `inum + 0x24` (= inum + 36) when
`an->race == 4` (RACE_SNAKE) and `an->state < 14` (`fmain.c:2459`). The snake walk-S
base is therefore at physical frame 36, walk-W at 44, etc. — they reuse the same
8-frame-per-direction layout but starting 36 frames into the sheet.

| Frame | Name / description | Size (px) | Source ref |
|-------|--------------------|-----------|------------|
| 0–7 | Walk S animation (8 frames) | 16×32 | `fmain.c:1010` `diroffs[4]=0` |
| 8–15 | Walk W animation (8 frames) | 16×32 | `fmain.c:1010` `diroffs[6]=8` |
| 16–23 | Walk N animation (8 frames) | 16×32 | `fmain.c:1010` `diroffs[0]=16` |
| 24–31 | Walk E animation (8 frames) | 16×32 | `fmain.c:1010` `diroffs[2]=24` |
| 32–43 | Fight S animation (12 frames) | 16×32 | `fmain.c:1010` `diroffs[12]=32` |
| 44–55 | Fight W animation (12 frames) | 16×32 | `fmain.c:1010` `diroffs[14]=44` |
| 56–67 | Fight N animation (12 frames) | 16×32 | `fmain.c:1010` `diroffs[8]=56` |
| 68–79 | Fight E animation (12 frames) | 16×32 | `fmain.c:1010` `diroffs[10]=68` |
| 80–82 | Death / corpse sequence (3 frames) | 16×32 | `fmain.c:154` `statelist[80-82]` |
| 83–87 | *(unknown — beyond 64-frame count, may be unused padding)* | 16×32 | — |

### Snake / Salamander offset

For `cfiles[12]` (snake/salamander, `actor_file=12`), frames are accessed as
`inum + 0x24` (= inum + 36) when `an->race == 4` (RACE_SNAKE) and `an->state < 14`
(`fmain.c:2459`). The snake walk-S base is therefore at physical frame 36, walk-W at 44,
etc. — they reuse the same 8-frame-per-direction layout but starting 36 frames into the
sheet.

### Per-enemy file summary

| `actor_file` | Identity | `cfiles` index |
|---|---|---|
| 6 | Ogre | 6 |
| 7 | Ghost / Wraith / Skeleton | 7 |
| 8 | Dark Knight / Spider | 8 |
| 9 | Necromancer / Farmer / Loraii | 9 |
| 12 | Snake / Salamander | 12 |

## Notes

- **`statelist` indirection:** The renderer never uses a PHIL frame index directly.
  Body frames go through `figure = statelist[inum].figure` (`fmain.c:2458`). The table
  above lists frames by their physical position in the PHIL sheet.

- **`fallstates` frames:** Fall animation uses frames from `fallstates[24]`
  (`fmain2.c:871-874`). Julian: 0x20, 0x22, 0x3a, 0x6f, 0x70, 0x71. Phillip: 0x24,
  0x27, 0x3c, 0x6f, 0x70, 0x71. Kevin: 0x37, 0x38, 0x3d, 0x6f, 0x70, 0x71. Frames
  0x6f (111), 0x70 (112), 0x71 (113) are shared by all brothers (late fall / puff
  frames); those high-index values exceed the PHIL 67-frame count and may reference
  OBJECTS frames via an atype switch at `fmain.c:2457`.

- **ENEMY frame layout:** All ENEMY files share identical layout (walk×4 + fight×4 + death).
  Visual content differs per enemy type; the layout structure is invariant.
