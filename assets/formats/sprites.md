# Sprites

Directory: `sprites/`. Decoded from the `cfiles` shape files on the disk image (`src/fmain2.c:644-662`,
loader `read_shapes` `src/fmain2.c:690-698`): 5 bitplanes, frame-major, drawn with the
`pagecolors` palette; index 31 is the key colour (`transparent_index`).

```
sprites/
  heroes/<actor>/   julian phillip kevin
  enemies/<actor>/  ogre orcs wraith skeleton snake spider dknight loraii necromancer woodcutter
  npcs/<actor>/     wizard priest guard princess king noble sorceress bartender witch spectre ghost ranger begger
  carriers/<actor>/ bird dragon raft turtle
  objects/          the OBJECTS sheet: items, weapon overlays, effects
    items/<item>.png              one PNG per pick-up item (40)
    weapons/<weapon>/             dirk mace sword bow wand — overlay frame sets
    effects/<effect>/             arrow fireball bubbles — missile / effect frame sets
  raw/              the 17 cfiles as stored, in original frame order
```

## Actor set — `<actor>/<actor>.json`

One set per actor the game can draw, with the frames resolved from whichever cfile the game
really reads them from (hero fall frames come from the necromancer file, the landed swan from the
raft file — see `frames[].origin`).

| Field | Meaning |
|---|---|
| `actor`, `group`, `title` | id, directory group, display title |
| `cfile` | `{index, sheet, source}` — the primary cfile this set was cut from |
| `palette` | `"palettes/pagecolors.json"` |
| `transparent_index` | `31` |
| `frame_size` | `[w, h]` of every frame cell: 16×32 for people, 64×64 swan, 48×40 dragon, 32×32 raft/turtle |
| `frame_count`, `frames[]` | `{index, file, rect, origin}`: `file` is `frame_NNN.png` (indexed PNG, `tRNS` 31); `rect` `[x, y, w, h]` locates the frame in the sheet; `origin` `{cfile, sheet, frame}` is where it came from, plus `padded_from: [w, h]` when a smaller source frame was padded top-left into the cell |
| `sheet` | `{file, columns, highlight_mask, silhouette_mask, silhouette_source}` — `<actor>_sheet.png` indexed, `<actor>_highlightmask.png` (index 16–24, [palettes.md](palettes.md)), `<actor>_silhouettemask.png` (set where the pixel is not index 31 = the collision/occlusion silhouette the game computes with `make_mask`, `src/fsubs.asm:1619-1653`) |
| `overlay_sheet` | heroes/enemies: `{file: "sprites/objects/objects_sheet.png", frame_size: [16,16], columns: 16, source}` — where `overlays` frames are read from |
| `weapons[]` | heroes/enemies: `{id, name}` the actor can hold; `0` = none. Enemies carry `weapons_source` (`weapon_probs` row) |
| `modes[]` | the animations (below) |
| `notes[]` | `{text, source}` — render rules and source bugs applied (see also `reference/PROBLEMS.md` P25) |
| heroes: `brother`, `brother_source` · enemies: `race`, `race_source` · NPCs: `setfig`, `setfig_source`, `image_base`, `can_talk` | identity in the game's tables |

### Modes

```json
{"id": "walk", "label": "Walk", "playback": "loop", "directional": true,
 "source": "src/fmain.c:1629-1632", "weapons": [0,1,2,3,4,5],
 "facings": {"N": [ {step}, ... ], "NE": [...], "E": [...], "SE": [...], "S": [...], "SW": [...], "W": [...], "NW": [...]}}
```

| Field | Meaning |
|---|---|
| `id`, `label`, `source`, `note` | id, display label, source citation, optional explanation |
| `directional` | `true`: `facings` maps the 8 compass directions to step lists; `false`: a single `steps` list |
| `playback` | `loop` (cycle), `once` (hold last step), `transitions` (fight: next step = `transitions[step][rand4()]`, the 9×4 table is included), `random` (pick any step each tick) |
| `weapons` | which weapon ids this mode can be drawn with (omitted when the actor has none) |
| step `frame` | index into `frames[]` |
| step `ticks` | how many game ticks the step is held (omitted = 1 / until the state changes) |
| step `state_index` | the `statelist` row the game used (`src/fmain2.c`), for cross-reference |
| step `overlays` | map weapon id → `{frame, rows: [r0, r1], dx, dy, behind}`: draw rows `r0..r1-1` of 16×16 frame `frame` of `overlay_sheet` at offset `(dx, dy)` from the actor frame's top-left, before (`behind: true`) or after the body |

Mode ids seen: heroes `still walk fight shoot_bow cast_wand dying frustrated sink sleep fall`;
enemies `still walk fight dying` (+ race-specific); NPCs `still`/`still_front`/`still_back`,
`dying`, `dead`, `other`; carriers `fly`/`grounded` (swan), `swim`/`ridden_idle`… (turtle), etc.
The step lists already apply the game's per-race drawing rules (even/odd frame parity, wraith
glide, snake offset, DKnight slot, fight-state remaps, dying order), so a consumer plays them as
written.

## Objects — `objects/objects.json`

The OBJECTS cfile (116 frames of 16×16; frame 115 is only partly loaded by the game, `cfile.load_shortfall`).

| Field | Meaning |
|---|---|
| `frames[]` | `{index, file, rect}` into `objects_sheet.png` (16 columns) |
| `items[]` | one entry per pick-up item (40): `{name, file: "items/<item>.png", frame, rows, icon_rows, stuff_index: [...]}` — `rows` are the sheet rows that hold the item (frames holding two items in rows 0–7 / 8–15 are split, `src/fmain.c:2524`), `icon_rows` the rows the inventory page draws, `stuff_index` the inventory slot(s) |
| `inventory_icons` | `{items[], note, source}` — per inventory slot: `{stuff_index, name, icon: {frame, rows}, items_page: {x, y, ydelta, maxshown}, source}` — how the Items page lays the icons out (`src/fmain.c:3128-3139`) |
| `world_object_ids` | `{objects[], note, source}` — ground objects: `{ob_id, frame, rows, item, enum}` — a placed object's `ob_id` is its frame; bit 7 selects rows 8–15 of frame `ob_id & 0x7f` |
| `sheet` | sheet + highlight/silhouette masks as for actors |

`items/<item>.png` are 16×16 (or 16×8 padded) indexed PNGs with `tRNS` 31.

## Weapon and effect sets — `objects/weapons/<w>/<w>.json`, `objects/effects/<e>/<e>.json`

Same shape as an actor set without modes: `frames[]` are `{index, file, rect, origin, rows, used_by[]}`
where `origin` is the OBJECTS frame, `rows` the row slice drawn, and `used_by` lists the actor
modes (weapons: `"<mode> (state N)"`; effects: `"flight <dir>"`) that draw it. `source`/`note`
cite the draw code (`src/fmain.c:2400-2447` overlays, `2319-2322`/`2492-2497` effects). Weapons:
`dirk mace sword bow wand`; effects: `arrow` (8 flight directions), `fireball`, `bubbles`.

## Raw sheets — `raw/raw.json`, `raw/cfile_NN_<name>.png`

Each cfile decoded exactly as stored (frame `n` at `file_id*512 + n*5*h*w/8`), 16 columns,
original frame order, indexed with `tRNS` 31. `sheets[]`: `{cfile, name, file, file_id, numblocks,
frame_count, frame_size, columns, loaded_frames?, source}` — `loaded_frames` is present where the
game's `numblocks` loads fewer frames than the file holds. cfile 12 (a duplicate of Julian's
block that is never loaded) is skipped.
