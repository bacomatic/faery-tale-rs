# Sprite Atlas Documentation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create `reference/data/` tier with a README and three sprite atlas files covering all game sprite sheets, populated by source-code inference with unknown frames flagged.

**Architecture:** New `reference/data/` subdirectory holds a README defining the data-tier convention, plus `sprites/` with three focused atlas files grouped by sheet role. No existing files are modified except `reference/README.md` (one new table row). All frame data is derived from existing source files and research docs — no new tooling required.

**Tech Stack:** Markdown, source files `fmain.c`/`fmain2.c`, existing research docs (`logic/sprite-rendering.md`, `_discovery/sprite-compositing.md`, `_discovery/inventory.md`, `_discovery/world-objects.md`, `_discovery/disk-io.md`, `_discovery/carriers.md`).

---

## Key facts (read before starting any task)

**`cfiles[]` table** (`fmain2.c:639-657`): sprite sheet dimensions and frame counts.

| Index | Identity | W×H px | Frames | Seq slot |
|-------|----------|--------|--------|----------|
| 0 | Julian | 16×32 | 67 | PHIL |
| 1 | Phillip | 16×32 | 67 | PHIL |
| 2 | Kevin | 16×32 | 67 | PHIL |
| 3 | Objects | 16×16 | 116 | OBJECTS |
| 4 | Raft | 32×32 | 2 | RAFT |
| 5 | Turtle | 32×32 | 16 | CARRIER |
| 6 | Ogre | 16×32 | 64 | ENEMY |
| 7 | Ghost | 16×32 | 64 | ENEMY |
| 8 | Dark Knight / Spiders | 16×32 | 64 | ENEMY |
| 9 | Necromancer / Farmer / Loraii | 16×32 | 64 | ENEMY |
| 10 | Dragon | 48×40 | 5 | DRAGON |
| 11 | Bird / Swan | 64×64 | 8 | CARRIER |
| 12 | Snake / Salamander | 16×32 | 64 | ENEMY |
| 13 | Wizard/Priest set | 16×32 | 8 | SETFIG |
| 14 | Royal set | 16×32 | 8 | SETFIG |
| 15 | Bartender set | 16×32 | 8 | SETFIG |
| 16 | Witch set | 16×32 | 8 | SETFIG |
| 17 | Ranger/Beggar set | 16×32 | 8 | SETFIG |

**`sequences` enum** (`ftale.h:88`): `PHIL=0, OBJECTS=1, ENEMY=2, RAFT=3, SETFIG=4, CARRIER=5, DRAGON=6`

**OBJECTS frame derivation rules** (`fmain.c:2472–2479`, `fmain.c:3133`):
- Most frames: 16×16 px
- Half-height (16×8 px) set: `inum == 0x1b`, `8 ≤ inum ≤ 12`, `inum == 25 or 26`, `0x11 ≤ inum ≤ 0x17`, any `inum` with bit 7 set
- Inventory icon render: `n = image_number * 80 + img_off` (byte offset into OBJECTS data)

**`inv_list[]` → OBJECTS frame** (`fmain.c:380-424`): `image_number` is the OBJECTS `inum` for each inventory slot.

| inv idx | image_number (inum) | name |
|---------|---------------------|------|
| 0 | 12 | Dirk |
| 1 | 9 | Mace |
| 2 | 8 | Sword |
| 3 | 10 | Bow |
| 4 | 17 | Magic Wand |
| 5 | 27 | Golden Lasso |
| 6 | 23 | Sea Shell |
| 7 | 27 | Sun Stone (same frame as Lasso, img_off=8) |
| 8 | 3 | Arrows |
| 9 | 18 | Blue Stone |
| 10 | 19 | Green Jewel |
| 11 | 22 | Glass Vial |
| 12 | 21 | Crystal Orb |
| 13 | 23 | Bird Totem (same sheet row as Sea Shell, img_off=0) |
| 14 | 17 | Gold Ring (same sheet row as Wand, img_off=0) |
| 15 | 24 | Jade Skull |
| 16 | 25 | Gold Key |
| 17 | 25 | Green Key (img_off=8, lower half) |
| 18 | 114 | Blue Key |
| 19 | 114 | Red Key (img_off=8, lower half) |
| 20 | 26 | Grey Key |
| 21 | 26 | White Key (img_off=8, lower half) |
| 22 | 11 | Talisman |
| 23 | 19 | Rose (same sheet row as Green Jewel, img_off=8) |
| 24 | 20 | Fruit |
| 25 | 21 | Gold Statue (same sheet row as Crystal Orb, img_off=8) |
| 26 | 22 | Book (same sheet row as Glass Vial, img_off=8) |
| 27 | 8 | Herb (same sheet row as Sword, img_off=8) |
| 28 | 9 | Writ (same sheet row as Mace, img_off=8) |
| 29 | 10 | Bone (same sheet row as Bow, img_off=8) |
| 30 | 12 | Shard (same sheet row as Dirk, img_off=8) |

**World object `ob_id` → OBJECTS `inum`** (`fmain2.c:1287`): `an->index = list->ob_id` — the ob_id IS the inum for OBJECTS-type world items. Named values from `enum obytes` (`fmain2.c:967-977`): QUIVER=11, MONEY=13, URN=14, CHEST=15, SACKS=16, G_RING=17, B_STONE=18, G_JEWEL=19, SCRAP=20, C_ORB=21, VIAL=22, B_TOTEM=23, J_SKULL=24, GOLD_KEY=25, GREY_KEY=26, FOOTSTOOL=31, TURTLE=102, BLUE_KEY=114, M_WAND=145, MEAL=146, ROSE=147, FRUIT=148, STATUE=149, BOOK=150, SHELL=151, GREEN_KEY=153, WHITE_KEY=154, RED_KEY=242.

**Weapon overlay OBJECTS frames** (`fmain.c:2440-2444`, `fmain.c:154-204`):
- `statelist[inum].wpn_no + k` where k: mace=32, sword=48, Dirk=64, bow=0
- `statelist[].wpn_no` ranges 0..15 across the 87 entries (base-frame within each weapon block)
- Dirk overlays: frames 64..79; Sword overlays: 48..63; Mace overlays: 32..47; Bow overlays: 0..15
- Wand overlays: frames 103..110 (`facing + 103`, facings 0..7)
- Bow direction frames: 30 (E/W), 0x51=81 (N), 0x53=83 (S)

**Special OBJECTS frames**:
- 0x11–0x17 (17–23): arrow flight frames (half-height)
- 0x1b (27): arrow shaft (half-height)
- 25, 26: bones/scrap (half-height) — NOTE: these are also `GREY_KEY=26` and `GOLD_KEY=25` as world object ob_ids. The half-height rule applies when these indices appear as OBJECTS inum from the sprite size logic, distinct from their use as ob_id values.
- 0x58 (88): fiery-death overlay (`fmain.c:2454`)
- 97, 98: drowning bubbles (`fmain.c:2497`)
- 100–101: bubble/spell effects, skip terrain mask (`fmain.c:2568`)

**`statelist[87]`** (`fmain.c:154-204`): maps animation `inum` → body frame `figure` (PHIL sheet) + weapon overlay anchor. Full table already in `logic/sprite-rendering.md`.

**`diroffs[16]`** (`fmain.c:1010`): walk base frames: S=0, SW=8, N=16, NE=24; fight base frames: SE=32, SW=44, NW=56, NE=68.

**`fallstates[24]`** (`fmain2.c:871-874`): per-brother fall frames. Julian: 0x20,0x22,0x3a,0x6f,0x70,0x71. Phillip: 0x24,0x27,0x3c,0x6f,0x70,0x71. Kevin: 0x37,0x38,0x3d,0x6f,0x70,0x71. Frames 0x6f,0x70,0x71 shared by all brothers.

**RAFT frames** (`fmain.c:2463-2464`, `fmain2.c:1318`): frame 0 = raft in water (only frame ever rendered for actual raft). Frame 1 = grounded swan (used via swan rendering override when `atype==CARRIER && riding==0 && actor_file==11` → `atype=RAFT, inum=1`).

**CARRIER frames**: Turtle (`cfiles[5]`): 16 frames, 32×32 px. Bird/Swan (`cfiles[11]`): 8 frames, 64×64 px. Frame references in carrier logic code for mount/dismount and movement states.

**DRAGON frames** (`cfiles[10]`): 5 frames, 48×40 px. Dragon is DRAGON type; referenced in combat/frustration.

**SETFIG frames**: Each SETFIG set (`cfiles[13-17]`) has 8 frames, 16×32 px. `setfig_table[]` (`fmain.c:24-39`) maps ob_id to `{cfile_entry, image_base, can_talk}`:

| ob_id | NPC | cfile | image_base | can_talk |
|-------|-----|-------|------------|----------|
| 0 | Wizard | 13 | 0 | yes |
| 1 | Priest | 13 | 4 | yes |
| 2 | Guard (front) | 14 | 0 | no |
| 3 | Guard (back) | 14 | 1 | no |
| 4 | Princess | 14 | 2 | no |
| 5 | King | 14 | 4 | yes |
| 6 | Noble | 14 | 6 | no |
| 7 | Sorceress | 14 | 7 | no |
| 8 | Bartender | 15 | 0 | no |
| 9 | Witch | 16 | 0 | no |
| 10 | Spectre | 16 | 6 | no |
| 11 | Ghost | 16 | 7 | no |
| 12 | Ranger | 17 | 0 | yes |
| 13 | Beggar | 17 | 4 | yes |

`an->index = setfig_table[ob_id].image_base` — so the SETFIG sheet `inum` is `image_base`.

---

## Task 1: Create `reference/data/README.md`

**Files:**
- Create: `reference/data/README.md`

- [ ] Create the file with this exact content:

```markdown
# Data Tables

This directory contains source-extracted tabular data for Faery Tale Adventure.

## What belongs here

Dense per-entry reference tables where the primary value is the data itself rather
than behavioral logic or trace narrative:

- Sprite sheet frame registries (atlases)
- Future candidates: terrain-type table, encounter probability tables, item/weapon
  stat tables

## Trust level

Same as `logic/` — normative, source-verified. Entries are derived from source code
(`fmain.c`, `fmain2.c`, `ftale.h`) with source line references. Unknown entries are
explicitly marked `*(unknown)*` rather than omitted.

## What is NOT here

- Behavioral pseudo-code → see `logic/`
- Raw trace notes → see `_discovery/`
- Narrative/quest data → see `STORYLINE.md` and sub-documents

## Contents

| File | Purpose |
|------|---------|
| [sprites/objects.md](sprites/objects.md) | OBJECTS sheet — 116 frames (items, overlays, effects) |
| [sprites/actors.md](sprites/actors.md) | PHIL + ENEMY sheets — actor animation body frames |
| [sprites/carriers.md](sprites/carriers.md) | RAFT, CARRIER, DRAGON, SETFIG sheets |
```

- [ ] Commit:

```bash
git add reference/data/README.md
git commit -m "docs(data): add data/ tier README"
```

---

## Task 2: Create `reference/data/sprites/objects.md`

**Files:**
- Create: `reference/data/sprites/objects.md`

This is the largest atlas. Work through the OBJECTS sheet frame by frame. Populate names from the key-facts tables above. Any frame index not covered by those tables gets `*(unknown)*` and is listed in the Unknown Frames section.

- [ ] Create the file. Use the complete content below — copy exactly, do not abbreviate:

```markdown
# OBJECTS Sheet Atlas

> Sheet: OBJECTS  
> `seq_list` slot: 1 (`OBJECTS`)  
> Source: `fmain2.c:649` (`cfiles[3]`), `fmain.c:380–424`  
> Disk block: 1312 (36 blocks)  
> Frame count: 116  
> Default size: 16×16 px per frame  
> Half-height rule: frames 0x11–0x17, 0x1b, 8–12, 25–26, and any `inum` with bit 7 set
> render at 16×8 px (lower 8 scanlines of the same 16-scanline row addressable via
> bit-7 flag). See `logic/sprite-rendering.md § compute_sprite_size`.

Each frame occupies one 16-scanline row in the sheet. Half-height frames pack two
8-scanline sub-frames into one row; the lower sub-frame is accessed by setting bit 7
of `inum` (which also shifts the source Y by +8).

`image_number` in `inv_list[]` is the OBJECTS `inum` for inventory icons (`fmain.c:3133`).  
World object `ob_id` IS the OBJECTS `inum` directly (`fmain2.c:1287`).

## Frame Table

| `inum` | Name / description | Size (px) | Source ref |
|--------|-------------------|-----------|------------|
| `0x00` (0) | *(unknown)* | 16×16 | — |
| `0x01` (1) | *(unknown)* | 16×16 | — |
| `0x02` (2) | *(unknown)* | 16×16 | — |
| `0x03` (3) | Arrows (inventory icon) | 16×8 | `fmain.c:390` `inv_list[8]` |
| `0x04` (4) | *(unknown)* | 16×16 | — |
| `0x05` (5) | *(unknown)* | 16×16 | — |
| `0x06` (6) | *(unknown)* | 16×16 | — |
| `0x07` (7) | *(unknown)* | 16×16 | — |
| `0x08` (8) | Sword (inventory icon, upper half) / small ground item | 16×8 | `fmain.c:383` `inv_list[2]`; `fmain.c:2478` |
| `0x09` (9) | Mace (inventory icon, upper half) / small ground item | 16×8 | `fmain.c:382` `inv_list[1]`; `fmain.c:2478` |
| `0x0a` (10) | Bow (inventory icon, upper half) / small ground item | 16×8 | `fmain.c:384` `inv_list[3]`; `fmain.c:2478` |
| `0x0b` (11) | Talisman (inventory icon, upper half) / small ground item | 16×8 | `fmain.c:407` `inv_list[22]`; `fmain.c:2478` |
| `0x0c` (12) | Dirk (inventory icon, upper half) / small ground item | 16×8 | `fmain.c:381` `inv_list[0]`; `fmain.c:2478` |
| `0x0d` (13) | Money / 50 gold pieces (world ob_id=13) | 16×16 | `fmain2.c:977` `MONEY` |
| `0x0e` (14) | Brass Urn (world ob_id=14) | 16×16 | `fmain2.c:977` `URN` |
| `0x0f` (15) | Chest (world ob_id=15) | 16×16 | `fmain2.c:977` `CHEST` |
| `0x10` (16) | Sacks (world ob_id=16) | 16×16 | `fmain2.c:977` `SACKS` |
| `0x11` (17) | Arrow flight frame 1 | 16×8 | `fmain.c:2479` |
| `0x12` (18) | Arrow flight frame 2 | 16×8 | `fmain.c:2479` |
| `0x13` (19) | Arrow flight frame 3 | 16×8 | `fmain.c:2479` |
| `0x14` (20) | Arrow flight frame 4 | 16×8 | `fmain.c:2479` |
| `0x15` (21) | Arrow flight frame 5 | 16×8 | `fmain.c:2479` |
| `0x16` (22) | Arrow flight frame 6 | 16×8 | `fmain.c:2479` |
| `0x17` (23) | Arrow flight frame 7 | 16×8 | `fmain.c:2479` |
| `0x18` (24) | *(unknown)* | 16×16 | — |
| `0x19` (25) | Bones / scrap (half-height, upper sub-frame) | 16×8 | `fmain.c:2478` |
| `0x1a` (26) | Bones / scrap (half-height, upper sub-frame) | 16×8 | `fmain.c:2478` |
| `0x1b` (27) | Arrow shaft | 16×8 | `fmain.c:2478` |
| `0x1c` (28) | *(unknown)* | 16×16 | — |
| `0x1d` (29) | *(unknown)* | 16×16 | — |
| `0x1e` (30) | Bow overlay — E/W direction | 16×16 | `fmain.c:2431` |
| `0x1f` (31) | Footstool (world ob_id=31) | 16×16 | `fmain2.c:977` `FOOTSTOOL` |
| `0x20` (32) | Mace overlay — base frame 0 (walk S) | 16×16 | `fmain.c:2440` `WPN_K_MACE=32` |
| `0x21` (33) | Mace overlay — base frame 1 | 16×16 | `fmain.c:2440` |
| `0x22` (34) | Mace overlay — base frame 2 | 16×16 | `fmain.c:2440` |
| `0x23` (35) | Mace overlay — base frame 3 | 16×16 | `fmain.c:2440` |
| `0x24` (36) | Mace overlay — base frame 4 | 16×16 | `fmain.c:2440` |
| `0x25` (37) | Mace overlay — base frame 5 | 16×16 | `fmain.c:2440` |
| `0x26` (38) | Mace overlay — base frame 6 | 16×16 | `fmain.c:2440` |
| `0x27` (39) | Mace overlay — base frame 7 | 16×16 | `fmain.c:2440` |
| `0x28` (40) | Mace overlay — base frame 8 | 16×16 | `fmain.c:2440` |
| `0x29` (41) | Mace overlay — base frame 9 | 16×16 | `fmain.c:2440` |
| `0x2a` (42) | Mace overlay — base frame 10 | 16×16 | `fmain.c:2440` |
| `0x2b` (43) | Mace overlay — base frame 11 | 16×16 | `fmain.c:2440` |
| `0x2c` (44) | Mace overlay — base frame 12 | 16×16 | `fmain.c:2440` |
| `0x2d` (45) | Mace overlay — base frame 13 | 16×16 | `fmain.c:2440` |
| `0x2e` (46) | Mace overlay — base frame 14 | 16×16 | `fmain.c:2440` |
| `0x2f` (47) | Mace overlay — base frame 15 | 16×16 | `fmain.c:2440` |
| `0x30` (48) | Sword overlay — base frame 0 (walk S) | 16×16 | `fmain.c:2441` `WPN_K_SWORD=48` |
| `0x31` (49) | Sword overlay — base frame 1 | 16×16 | `fmain.c:2441` |
| `0x32` (50) | Sword overlay — base frame 2 | 16×16 | `fmain.c:2441` |
| `0x33` (51) | Sword overlay — base frame 3 | 16×16 | `fmain.c:2441` |
| `0x34` (52) | Sword overlay — base frame 4 | 16×16 | `fmain.c:2441` |
| `0x35` (53) | Sword overlay — base frame 5 | 16×16 | `fmain.c:2441` |
| `0x36` (54) | Sword overlay — base frame 6 | 16×16 | `fmain.c:2441` |
| `0x37` (55) | Sword overlay — base frame 7 | 16×16 | `fmain.c:2441` |
| `0x38` (56) | Sword overlay — base frame 8 | 16×16 | `fmain.c:2441` |
| `0x39` (57) | Sword overlay — base frame 9 | 16×16 | `fmain.c:2441` |
| `0x3a` (58) | Sword overlay — base frame 10 | 16×16 | `fmain.c:2441` |
| `0x3b` (59) | Sword overlay — base frame 11 | 16×16 | `fmain.c:2441` |
| `0x3c` (60) | Sword overlay — base frame 12 | 16×16 | `fmain.c:2441` |
| `0x3d` (61) | Sword overlay — base frame 13 | 16×16 | `fmain.c:2441` |
| `0x3e` (62) | Sword overlay — base frame 14 | 16×16 | `fmain.c:2441` |
| `0x3f` (63) | Sword overlay — base frame 15 | 16×16 | `fmain.c:2441` |
| `0x40` (64) | Dirk overlay — base frame 0 (walk S) | 16×16 | `fmain.c:2442` `WPN_K_DIRK=64` |
| `0x41` (65) | Dirk overlay — base frame 1 | 16×16 | `fmain.c:2442` |
| `0x42` (66) | Dirk overlay — base frame 2 | 16×16 | `fmain.c:2442` |
| `0x43` (67) | Dirk overlay — base frame 3 | 16×16 | `fmain.c:2442` |
| `0x44` (68) | Dirk overlay — base frame 4 | 16×16 | `fmain.c:2442` |
| `0x45` (69) | Dirk overlay — base frame 5 | 16×16 | `fmain.c:2442` |
| `0x46` (70) | Dirk overlay — base frame 6 | 16×16 | `fmain.c:2442` |
| `0x47` (71) | Dirk overlay — base frame 7 | 16×16 | `fmain.c:2442` |
| `0x48` (72) | Dirk overlay — base frame 8 | 16×16 | `fmain.c:2442` |
| `0x49` (73) | Dirk overlay — base frame 9 | 16×16 | `fmain.c:2442` |
| `0x4a` (74) | Dirk overlay — base frame 10 | 16×16 | `fmain.c:2442` |
| `0x4b` (75) | Dirk overlay — base frame 11 | 16×16 | `fmain.c:2442` |
| `0x4c` (76) | Dirk overlay — base frame 12 | 16×16 | `fmain.c:2442` |
| `0x4d` (77) | Dirk overlay — base frame 13 | 16×16 | `fmain.c:2442` |
| `0x4e` (78) | Dirk overlay — base frame 14 | 16×16 | `fmain.c:2442` |
| `0x4f` (79) | Dirk overlay — base frame 15 | 16×16 | `fmain.c:2442` |
| `0x50` (80) | *(unknown)* | 16×16 | — |
| `0x51` (81) | Bow overlay — N direction | 16×16 | `fmain.c:2432` |
| `0x52` (82) | *(unknown)* | 16×16 | — |
| `0x53` (83) | Bow overlay — S direction | 16×16 | `fmain.c:2433` |
| `0x54` (84) | *(unknown)* | 16×16 | — |
| `0x55` (85) | *(unknown)* | 16×16 | — |
| `0x56` (86) | *(unknown)* | 16×16 | — |
| `0x57` (87) | *(unknown)* | 16×16 | — |
| `0x58` (88) | Fiery-death overlay (dying actor in lava zone) | 16×16 | `fmain.c:2454` `WPN_FIERY_DEATH_INUM` |
| `0x59` (89) | *(unknown)* | 16×16 | — |
| `0x5a` (90) | *(unknown)* | 16×16 | — |
| `0x5b` (91) | *(unknown)* | 16×16 | — |
| `0x5c` (92) | *(unknown)* | 16×16 | — |
| `0x5d` (93) | *(unknown)* | 16×16 | — |
| `0x5e` (94) | *(unknown)* | 16×16 | — |
| `0x5f` (95) | *(unknown)* | 16×16 | — |
| `0x60` (96) | *(unknown)* | 16×16 | — |
| `0x61` (97) | Drowning bubble frame A | 16×16 | `fmain.c:2497` |
| `0x62` (98) | Drowning bubble frame B | 16×16 | `fmain.c:2497` |
| `0x63` (99) | *(unknown)* | 16×16 | — |
| `0x64` (100) | Bubble / spell effect A (no terrain mask) | 16×16 | `fmain.c:2568` |
| `0x65` (101) | Bubble / spell effect B (no terrain mask) | 16×16 | `fmain.c:2568` |
| `0x66` (102) | Turtle eggs (world ob_id=102) | 16×16 | `fmain2.c:977` `TURTLE` |
| `0x67` (103) | Wand overlay — facing NW (facing 0) | 16×16 | `fmain.c:2436` `WPN_WAND_INUM_BASE=103` |
| `0x68` (104) | Wand overlay — facing N (facing 1) | 16×16 | `fmain.c:2436` |
| `0x69` (105) | Wand overlay — facing NE (facing 2) | 16×16 | `fmain.c:2436` |
| `0x6a` (106) | Wand overlay — facing E (facing 3) | 16×16 | `fmain.c:2436` |
| `0x6b` (107) | Wand overlay — facing SE (facing 4) | 16×16 | `fmain.c:2436` |
| `0x6c` (108) | Wand overlay — facing S (facing 5) | 16×16 | `fmain.c:2436` |
| `0x6d` (109) | Wand overlay — facing SW (facing 6) | 16×16 | `fmain.c:2436` |
| `0x6e` (110) | Wand overlay — facing W (facing 7) | 16×16 | `fmain.c:2436` |
| `0x6f` (111) | *(unknown)* | 16×16 | — |
| `0x70` (112) | *(unknown)* | 16×16 | — |
| `0x71` (113) | *(unknown)* | 16×16 | — |
| `0x72` (114) | Blue / Red key (inventory icon, both share this row) | 16×16 | `fmain.c:402-403` `inv_list[18,19]` `BLUE_KEY` |
| `0x73` (115) | *(unknown)* | 16×16 | — |

## Notes

- **Weapon overlay index derivation:** `inum = statelist[body_inum].wpn_no + k` where
  k = {32=mace, 48=sword, 64=Dirk, 0=bow}. `wpn_no` values in `statelist` range 0–15,
  selecting one of 16 positions within each weapon block. See `logic/sprite-rendering.md
  § select_atype_inum`.

- **Half-height packing:** frames in the half-height set are stored as the upper 8
  scanlines of a 16-scanline row. The lower 8 scanlines are a second sub-frame
  accessible via `inum | 0x80` (bit-7 flag), which also shifts the source Y offset by +8.
  There is no sheet metadata encoding this — the list at `fmain.c:2478–2479` is the
  authoritative per-frame size table.

- **Shared `image_number` rows:** Several inventory items share the same OBJECTS frame
  row (same `image_number`) and are distinguished by `img_off` (0 = upper 8 scanlines,
  8 = lower 8 scanlines): Sword/Herb (8), Mace/Writ (9), Bow/Bone (10), Dirk/Shard (12),
  Wand/Gold Ring (17), Sea Shell/Bird Totem (23), Crystal Orb/Gold Statue (21),
  Glass Vial/Book (22), Green Jewel/Rose (19), Gold Key/Green Key (25),
  Blue Key/Red Key (114), Grey Key/White Key (26).

- **World ob_id identity:** For items rendered as OBJECTS-type actors from `set_objects`,
  `an->index = list->ob_id` directly (`fmain2.c:1287`). The ob_id constants in
  `enum obytes` thus double as OBJECTS frame indices.

## Unknown Frames

Frames with no code reference found. Candidates for visual inspection:

0, 1, 2, 4, 5, 6, 7, 24, 28, 29, 80, 82, 84, 85, 86, 87, 89, 90, 91, 92, 93, 94, 95,
96, 99, 111, 112, 113, 115
```

- [ ] Commit:

```bash
git add reference/data/sprites/objects.md
git commit -m "docs(data): add OBJECTS sheet sprite atlas"
```

---

## Task 3: Create `reference/data/sprites/actors.md`

**Files:**
- Create: `reference/data/sprites/actors.md`

- [ ] Create the file with this exact content:

```markdown
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
(`cfiles[12]`) is special: its frames begin at `inum + 0x24` (i.e. the enemy sheet is
loaded into ENEMY slot but snake frames are accessed at a +36 offset —
`fmain.c:2459`). Carriers (turtle, bird) and dragon also use ENEMY memory space for
loading but are accessed via CARRIER or DRAGON atype, not ENEMY.

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
```

- [ ] Commit:

```bash
git add reference/data/sprites/actors.md
git commit -m "docs(data): add PHIL + ENEMY actor sheet sprite atlas"
```

---

## Task 4: Create `reference/data/sprites/carriers.md`

**Files:**
- Create: `reference/data/sprites/carriers.md`

- [ ] Create the file with this exact content:

```markdown
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
```

- [ ] Commit:

```bash
git add reference/data/sprites/carriers.md
git commit -m "docs(data): add RAFT, CARRIER, DRAGON, SETFIG sprite atlas"
```

---

## Task 5: Update `reference/README.md`

**Files:**
- Modify: `reference/README.md`

- [ ] Open `reference/README.md`. Find the Canonical Documentation table (starts around line 18). Add one row after the `porting/README.md` row:

```markdown
| [data/README.md](data/README.md) | Data tier hub — sprite atlases and other source-extracted tabular data. |
```

The table should look like this after the edit:

```markdown
| Document | Purpose |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | High-level subsystem architecture, data flow, game-loop structure, display and memory model. |
| [RESEARCH.md](RESEARCH.md) | Ground-truth mechanics reference — **hub/index** linking to the sub-documents below. |
| [STORYLINE.md](STORYLINE.md) | Narrative progression — **hub/index** linking to the sub-documents below. |
| [PROBLEMS.md](PROBLEMS.md) | Open and resolved research questions that cannot be settled by straightforward source tracing. |
| [porting/README.md](porting/README.md) | Porting guidance hub for implementation-facing instructions and subsystem checklists. |
| [data/README.md](data/README.md) | Data tier hub — sprite atlases and other source-extracted tabular data. |
| [world_db.json](world_db.json) | Spatial database of objects, doors, encounter extents, terrain summaries, and region grids. |
| [quest_db.json](quest_db.json) | Machine-readable quest and progression data extracted from source analysis. |
```

- [ ] Commit:

```bash
git add reference/README.md
git commit -m "docs: register data/ tier in reference README"
```

---

## Self-review notes

- All frame indices in objects.md cover 0–115 (116 frames) ✓
- PHIL table covers all figure values that appear in statelist (0–66) ✓
- ENEMY table uses range notation where individual frames are unnamed (correct per spec) ✓
- SETFIG tables derive directly from `setfig_table[]` in `fmain.c:24-39` ✓
- RAFT frame 1 (grounded swan) and the swan-override path are both documented ✓
- `fallstates` high-index frames (0x6f–0x71) note that they may be OBJECTS frames — flagged as pending investigation ✓
- No TBDs, no placeholders ✓
