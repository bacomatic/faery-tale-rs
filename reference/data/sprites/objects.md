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
8-scanline sub-frames into one row. The upper sub-frame is addressed by `inum` directly;
the lower sub-frame is addressed by `inum | 0x80` (bit-7 flag), which shifts the source
Y by +8 (`fmain.c:2524`). Both sub-frames are listed separately in the table below.

`image_number` in `inv_list[]` is the OBJECTS `inum` for inventory icons (`fmain.c:3133`).  
World object `ob_id` IS the OBJECTS `inum` directly (`fmain2.c:1287`).

## Frame Table

| `inum` | Name / description | Size (px) | Source ref |
|--------|-------------------|-----------|------------|
| `0x00` (0) | Arrow in flight — facing 0 (NW) | 16×16 | `fmain.c:2319` `an->index = ms->direction`; `missile_type==1` (arrow) |
| `0x01` (1) | Arrow in flight — facing 1 (N) | 16×16 | `fmain.c:2319` |
| `0x02` (2) | Arrow in flight — facing 2 (NE) | 16×16 | `fmain.c:2319` |
| `0x03` (3) | Arrow in flight — facing 3 (E) / Arrows inventory icon | 16×16 | `fmain.c:2319`; `fmain.c:390` `inv_list[8]` |
| `0x04` (4) | Arrow in flight — facing 4 (SE) | 16×16 | `fmain.c:2319` |
| `0x05` (5) | Arrow in flight — facing 5 (S) | 16×16 | `fmain.c:2319` |
| `0x06` (6) | Arrow in flight — facing 6 (SW) | 16×16 | `fmain.c:2319` |
| `0x07` (7) | Arrow in flight — facing 7 (W) | 16×16 | `fmain.c:2319` |
| `0x08` (8) | Sword — inventory icon | 16×8 upper | `fmain.c:383` `inv_list[2]` `img_off=0` |
| `0x88` (8\|0x80) | Herb — inventory icon | 16×8 lower | `fmain.c:413` `inv_list[27]` `img_off=8` |
| `0x09` (9) | Mace — inventory icon | 16×8 upper | `fmain.c:382` `inv_list[1]` `img_off=0` |
| `0x89` (9\|0x80) | Writ — inventory icon | 16×8 lower | `fmain.c:414` `inv_list[28]` `img_off=8` |
| `0x0a` (10) | Bow — inventory icon | 16×8 upper | `fmain.c:384` `inv_list[3]` `img_off=0` |
| `0x8a` (10\|0x80) | Bone — inventory icon | 16×8 lower | `fmain.c:415` `inv_list[29]` `img_off=8` |
| `0x0b` (11) | Quiver of arrows (world ob_id=11) | 16×8 upper | `fmain.c:344` `QUIVER` |
| `0x8b` (11\|0x80) | Talisman — inventory icon | 16×8 lower | `fmain.c:407` `inv_list[22]` `img_off=8` |
| `0x0c` (12) | Dirk — inventory icon | 16×8 upper | `fmain.c:381` `inv_list[0]` `img_off=0` |
| `0x8c` (12\|0x80) | Shard — inventory icon | 16×8 lower | `fmain.c:417` `inv_list[30]` `img_off=8` |
| `0x0d` (13) | Money / 50 gold pieces (world ob_id=13) | 16×16 | `fmain2.c:977` `MONEY` |
| `0x0e` (14) | Brass Urn (world ob_id=14) | 16×16 | `fmain2.c:977` `URN` |
| `0x0f` (15) | Chest (world ob_id=15) | 16×16 | `fmain2.c:977` `CHEST` |
| `0x10` (16) | Sacks (world ob_id=16) | 16×16 | `fmain2.c:977` `SACKS` |
| `0x11` (17) | Magic Wand — inventory icon | 16×8 upper | `fmain.c:385` `inv_list[4]` `img_off=0` |
| `0x91` (17\|0x80) | Gold Ring — inventory icon | 16×8 lower | `fmain.c:397` `inv_list[14]` `img_off=8` |
| `0x12` (18) | Blue Stone — inventory icon | 16×8 upper | `fmain.c:391` `inv_list[9]` `img_off=0` |
| `0x92` (18\|0x80) | Meal / food (fish and vegetables) *(visual — no inv_list entry)* | 16×8 lower | — |
| `0x13` (19) | Green Jewel — inventory icon | 16×8 upper | `fmain.c:393` `inv_list[10]` `img_off=0` |
| `0x93` (19\|0x80) | Rose — inventory icon | 16×8 lower | `fmain.c:408` `inv_list[23]` `img_off=8` |
| `0x14` (20) | Writ *(visual — no inv_list entry at this offset)* | 16×8 upper | — |
| `0x94` (20\|0x80) | Apple — inventory icon | 16×8 lower | `fmain.c:409` `inv_list[24]` `img_off=8` |
| `0x15` (21) | Crystal Orb — inventory icon | 16×8 upper | `fmain.c:395` `inv_list[12]` `img_off=0` |
| `0x95` (21\|0x80) | Gold Statue — inventory icon | 16×8 lower | `fmain.c:411` `inv_list[25]` `img_off=8` |
| `0x16` (22) | Glass Vial — inventory icon | 16×8 upper | `fmain.c:394` `inv_list[11]` `img_off=0` |
| `0x96` (22\|0x80) | Book — inventory icon | 16×8 lower | `fmain.c:412` `inv_list[26]` `img_off=8` |
| `0x17` (23) | Bird Totem — inventory icon | 16×8 upper | `fmain.c:396` `inv_list[13]` `img_off=0` |
| `0x97` (23\|0x80) | Sea Shell — inventory icon | 16×8 lower | `fmain.c:388` `inv_list[6]` `img_off=8` |
| `0x18` (24) | Jade Skull (quest item, `J_SKULL`, world ob_id=24) | 16×16 | `fmain.c` `G_RING`..`J_SKULL` range 17–24 |
| `0x19` (25) | Gold Key — inventory icon | 16×8 upper | `fmain.c:400` `inv_list[16]` `img_off=0` |
| `0x99` (25\|0x80) | Green Key — inventory icon | 16×8 lower | `fmain.c:401` `inv_list[17]` `img_off=8` |
| `0x1a` (26) | Grey Key — inventory icon | 16×8 upper | `fmain.c:405` `inv_list[20]` `img_off=0` |
| `0x9a` (26\|0x80) | White Key — inventory icon | 16×8 lower | `fmain.c:406` `inv_list[21]` `img_off=8` |
| `0x1b` (27) | Arrow shaft (dropped/ground item) | 16×8 upper | `fmain.c:2478` |
| `0x9b` (27\|0x80) | Sun Stone — inventory icon | 16×8 lower | `fmain.c:389` `inv_list[7]` `img_off=8` |
| `0x1c` (28) | Dead brother's bones (world ob_id=28) | 16×16 | `fmain.c:3172-3176` |
| `0x1d` (29) | Opened / empty chest (world ob_id=29) | 16×16 | `fmain2.c:1208`; set when CHEST(15) is looted; `fmain.c:3184` |
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
| `0x50` (80) | Bow draw overlay — shoot state 0 (half-height) | 16×8 | `fmain.c:2444` `statelist[shoot_state].wpn_no + WPN_K_BOW`; `wpn_no≥80` = bit-7 half-height |
| `0x51` (81) | Bow overlay — facing N (held-at-draw) | 16×16 | `fmain.c:2433` |
| `0x52` (82) | Bow draw overlay — shoot state 2 (half-height) | 16×8 | `fmain.c:2444` `wpn_no≥80` |
| `0x53` (83) | Bow overlay — facing S (held-at-draw) | 16×16 | `fmain.c:2432` |
| `0x54` (84) | Bow draw overlay — shoot state 4 (half-height) | 16×8 | `fmain.c:2444` `wpn_no≥80` |
| `0x55` (85) | Bow draw overlay — shoot state 5 (half-height) | 16×8 | `fmain.c:2444` `wpn_no≥80` |
| `0x56` (86) | Bow draw overlay — shoot state 6 (half-height) | 16×8 | `fmain.c:2444` `wpn_no≥80` |
| `0x57` (87) | Bow draw overlay — shoot state 7 (half-height) | 16×8 | `fmain.c:2444` `wpn_no≥80` |
| `0x58` (88) | Fireball impact / spent-fireball puff | 16×16 | `fmain.c:2321` `missile_type==3` → `an->index=0x58`; set at hit (`fmain.c:2330`) |
| `0x59` (89) | Fireball projectile — facing 0 (NW) | 16×16 | `fmain.c:2322` `direction+0x59`; wand (`weapon-3=2`) and dragon (`missile_type=2`) |
| `0x5a` (90) | Fireball projectile — facing 1 (N) | 16×16 | `fmain.c:2322` |
| `0x5b` (91) | Fireball projectile — facing 2 (NE) | 16×16 | `fmain.c:2322` |
| `0x5c` (92) | Fireball projectile — facing 3 (E) | 16×16 | `fmain.c:2322` |
| `0x5d` (93) | Fireball projectile — facing 4 (SE) | 16×16 | `fmain.c:2322` |
| `0x5e` (94) | Fireball projectile — facing 5 (S) | 16×16 | `fmain.c:2322` |
| `0x5f` (95) | Fireball projectile — facing 6 (SW) | 16×16 | `fmain.c:2322` |
| `0x60` (96) | Fireball projectile — facing 7 (W) | 16×16 | `fmain.c:2322` |
| `0x61` (97) | Drowning bubble frame A | 16×16 | `fmain.c:2497` |
| `0x62` (98) | Drowning bubble frame B | 16×16 | `fmain.c:2497` |
| `0x63` (99) | *(unused)* | 16×16 | — |
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
| `0x6f` (111) | Hero silhouette falling — large | 16×16 | `fmain2.c:871` `fallstates[3]` |
| `0x70` (112) | Hero silhouette falling — medium | 16×16 | `fmain2.c:871` `fallstates[4]` |
| `0x71` (113) | Hero silhouette falling — small | 16×16 | `fmain2.c:871` `fallstates[5]` |
| `0x72` (114) | Blue Key — inventory icon | 16×8 upper | `fmain.c:402` `inv_list[18]` `img_off=0` |
| `0xf2` (114\|0x80) | Red Key — inventory icon | 16×8 lower | `fmain.c:403` `inv_list[19]` `img_off=8` |
| `0x73` (115) | *(unused)* | 16×16 | — |

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

Frames with no code reference found. Visual descriptions marked *(visual)* in the table
above were identified from extracted PNG sprites (`sprite_output/objects_unknown_labeled.png`);
these are tentative and may be revised after gameplay verification.

**Unused frames (no code reference):** 99 (0x63), 115 (0x73)

**Visually identified, source-unconfirmed:** 0x92 (meal/food), 0x14 (writ)
