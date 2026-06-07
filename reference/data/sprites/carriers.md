# Carrier, Raft, Dragon, and SETFIG Sheet Atlases

This file covers four sprite sheets loaded into non-actor/non-OBJECTS slots:
RAFT (`seq_list[3]`), SETFIG (`seq_list[4]`), CARRIER (`seq_list[5]`), DRAGON (`seq_list[6]`).

---

## RAFT Sheet

> `seq_list` slot: 3 (`RAFT`)  
> Source: `fmain2.c:650` (`cfiles[4]`), disk block 1348  
> Size: 32×32 px per frame  
> Frame count: 2

| Frame | Name / description | Size (px) | Source ref |
|-------|--------------------|-----------|------------|
| 0 | Raft on water (only frame displayed during normal raft use) | 32×32 | `fmain.c:1455`; `_discovery/carriers.md` |
| 1 | Grounded swan (swan on land, not mounted) | 32×32 | `fmain.c:2464` `inum=1` — swan override: `atype=RAFT, inum=1` when `actor_file==11 && riding==0` |

### Notes

The RAFT handler in `actor_tick` jumps to `statc` without writing `an->index`, so
frame 0 is the only frame ever displayed for an actual raft (`_discovery/carriers.md`).
Frame 1 is the grounded-swan reskin: when `atype==CARRIER && riding==0 &&
actor_file==11`, `select_atype_inum` overrides to `{atype=RAFT, inum=1}`
(`fmain.c:2463-2464`).

---

## CARRIER Sheet — Turtle

> `seq_list` slot: 5 (`CARRIER`)  
> Source: `fmain2.c:652` (`cfiles[5]`), disk block 1351  
> Size: 32×32 px per frame  
> Frame count: 16

| Frame | Name / description | Size (px) | Source ref |
|-------|--------------------|-----------|------------|
| 0 | Turtle idle / still | 32×32 | `fmain.c:2784` `load_carrier(5)` — `an->index=0` on load |
| 1–7 | Turtle movement animation frames | 32×32 | `_discovery/carriers.md` (walk cycle) |
| 8–15 | *(unknown — additional animation frames)* | 32×32 | — |

---

## CARRIER Sheet — Bird / Swan

> `seq_list` slot: 5 (`CARRIER`) — same slot, loaded when `actor_file=11`  
> Source: `fmain2.c:659` (`cfiles[11]`), disk block 1120  
> Size: 64×64 px per frame  
> Frame count: 8

| Frame | Name / description | Size (px) | Source ref |
|-------|--------------------|-----------|------------|
| 0 | Swan idle / still | 64×64 | `fmain.c:2784` `load_carrier(11)` — `an->index=0` on load |
| 1–7 | Swan flight animation frames | 64×64 | `_discovery/carriers.md` |

### Notes

Turtle and Bird/Swan share the CARRIER seq slot. They are never loaded simultaneously —
`load_carrier(n)` overwrites the CARRIER location in `shape_mem` when `actor_file != n`
(`fmain.c:2784-2801`). The swan on the ground uses RAFT frame 1 (see RAFT sheet above).

---

## DRAGON Sheet

> `seq_list` slot: 6 (`DRAGON`)  
> Source: `fmain2.c:658` (`cfiles[10]`), disk block 1160  
> Size: 48×40 px per frame  
> Frame count: 5

| Frame | Name / description | Size (px) | Source ref |
|-------|--------------------|-----------|------------|
| 0 | Dragon idle / still | 48×40 | `fmain.c:2784` `load_carrier(10)` — `an->index=0` on load |
| 1–4 | Dragon animation frames | 48×40 | `_discovery/carriers.md` |

### Notes

Dragon is `atype=DRAGON` (`ftale.h:88`). `load_carrier` sets `an->type=DRAGON`
only when `n==10` (`fmain.c:2784`). Frame selection follows the same `an->index`
mechanism as other carriers.

---

## SETFIG Sheets

> `seq_list` slot: 4 (`SETFIG`)  
> Source: `fmain2.c:661-665` (`cfiles[13-17]`)  
> Size: 16×32 px per frame  
> Frame count: 8 per set file

Five SETFIG files are loaded on demand as the hero enters regions containing NPCs.
`set_objects` selects the correct file via `setfig_table[ob_id].cfile_entry`
(`fmain2.c:1250`) and calls `read_shapes(cf); prep(SETFIG)` when the file changes.

`an->index = setfig_table[ob_id].image_base` — the SETFIG frame index is the
`image_base` value for each NPC type, not the ob_id directly (`fmain2.c:1272`).

### `setfig_table[]` — `fmain.c:24-39`

| `ob_id` | NPC | `cfile` | `image_base` | SETFIG frame | `can_talk` |
|---------|-----|---------|--------------|--------------|------------|
| 0 | Wizard | 13 | 0 | 0 | yes |
| 1 | Priest | 13 | 4 | 4 | yes |
| 2 | Guard (front) | 14 | 0 | 0 | no |
| 3 | Guard (back) | 14 | 1 | 1 | no |
| 4 | Princess | 14 | 2 | 2 | no |
| 5 | King | 14 | 4 | 4 | yes |
| 6 | Noble | 14 | 6 | 6 | no |
| 7 | Sorceress | 14 | 7 | 7 | no |
| 8 | Bartender | 15 | 0 | 0 | no |
| 9 | Witch | 16 | 0 | 0 | no |
| 10 | Spectre | 16 | 6 | 6 | no |
| 11 | Ghost | 16 | 7 | 7 | no |
| 12 | Ranger | 17 | 0 | 0 | yes |
| 13 | Beggar | 17 | 4 | 4 | yes |

### Per-file frame table

#### `cfiles[13]` — Wizard/Priest set (disk block 936)

| Frame | Description | Size (px) |
|-------|-------------|-----------|
| 0 | Wizard — still pose | 16×32 |
| 1–3 | *(unknown)* | 16×32 |
| 4 | Priest — still pose | 16×32 |
| 5–7 | *(unknown)* | 16×32 |

#### `cfiles[14]` — Royal set (disk block 931)

| Frame | Description | Size (px) |
|-------|-------------|-----------|
| 0 | Guard (front) | 16×32 |
| 1 | Guard (back) | 16×32 |
| 2 | Princess | 16×32 |
| 3 | *(unknown)* | 16×32 |
| 4 | King | 16×32 |
| 5 | *(unknown)* | 16×32 |
| 6 | Noble | 16×32 |
| 7 | Sorceress | 16×32 |

#### `cfiles[15]` — Bartender set (disk block 941)

| Frame | Description | Size (px) |
|-------|-------------|-----------|
| 0 | Bartender | 16×32 |
| 1–7 | *(unknown)* | 16×32 |

#### `cfiles[16]` — Witch set (disk block 946)

| Frame | Description | Size (px) |
|-------|-------------|-----------|
| 0 | Witch | 16×32 |
| 1–5 | *(unknown)* | 16×32 |
| 6 | Spectre | 16×32 |
| 7 | Ghost | 16×32 |

#### `cfiles[17]` — Ranger/Beggar set (disk block 951)

| Frame | Description | Size (px) |
|-------|-------------|-----------|
| 0 | Ranger | 16×32 |
| 1–3 | *(unknown)* | 16×32 |
| 4 | Beggar | 16×32 |
| 5–7 | *(unknown)* | 16×32 |

### Unknown SETFIG frames

All SETFIG NPCs are rendered as `an->state = STILL` (or `DEAD`). The STILL state
maps to a single idle frame via `statelist[]`. It is likely that each SETFIG set only
ever displays one or two frames (the `image_base` still pose and possibly a dead pose),
but frames at other positions within each 8-frame set are undocumented. Visual
inspection required to identify unused vs. alternate-pose frames.
