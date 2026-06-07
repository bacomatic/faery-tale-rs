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
