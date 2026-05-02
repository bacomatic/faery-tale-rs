# Save/Load System Checklist

Source scope:
- `fmain2.c:1474-1549` (`savegame()` — main entry point)
- `fmain2.c:1553-1558` (`saveload()` — low-level block I/O)
- `fmain.c:3621-3631` (`mod1save()` — brother inventories + missiles)
- `fmain.c:557-604` (the 80-byte misc-variable block and variable layout)
- `fmain2.c:1519, 1560-1566` (extent persistence and post-load restoration)

Purpose:
- Ensure ports serialize and restore all game state fields in the exact original byte order and endianness.

## A. Save File Format — Block Order

- [ ] Write/read blocks sequentially in exactly this order (`fmain2.c:1505-1527`):
  1. 80 bytes: misc variables starting at `&map_x` (`fmain2.c:1508`)
  2. 2 bytes: `region_num` (`fmain2.c:1511`)
  3. 6 bytes: `anix`, `anix2`, `mdex` (`fmain2.c:1514`)
  4. `anix × 22` bytes: `anim_list[0..anix-1]` (`struct shape`) (`fmain2.c:1515`)
  5. 105 bytes: `julstuff[35]`, `philstuff[35]`, `kevstuff[35]` via `mod1save()` (`fmain.c:3623-3625`)
  6. 60 bytes: `missile_list[6]` (10 bytes each) (`fmain.c:3630`)
  7. 24 bytes: `extent_list[0..1]` (12 bytes each, first 2 extents only) (`fmain2.c:1519`)
  8. 66 bytes: `ob_listg[11]` (6 bytes each, global objects) (`fmain2.c:1522`)
  9. 20 bytes: `mapobs[10]` (2 bytes each, per-region object counts) (`fmain2.c:1523`)
  10. 20 bytes: `dstobs[10]` (2 bytes each, per-region distributed flags) (`fmain2.c:1524`)
  11. variable: `ob_table[i]` for `i=0..9`, each `mapobs[i] × 6` bytes (`fmain2.c:1525-1526`)
- [ ] Total size is variable — `anix` and `mapobs[]` values determine the length.

## B. The 80-Byte Misc Block (`fmain.c:557-581`)

- [ ] Serialized as raw 68000 memory dump (big-endian, no marshalling) — native byte order.
- [ ] Block starts at `map_x` and spans exactly 80 bytes (40 consecutive 2-byte variables).
- [ ] Variable layout within the 80-byte block (byte offset from `map_x`):

| Offset | Variable | Notes |
|--------|----------|-------|
| 0 | `map_x` | World scroll X |
| 2 | `map_y` | World scroll Y |
| 4 | `hero_x` | Hero pixel X |
| 6 | `hero_y` | Hero pixel Y |
| 8 | `safe_x` | Last safe X |
| 10 | `safe_y` | Last safe Y |
| 12 | `safe_r` | Last safe region |
| 14 | `img_x` | Image X |
| 16 | `img_y` | Image Y |
| 18 | `cheat1` | Cheat flag — persists across saves |
| 20 | `riding` | Carrier riding flag |
| 22 | `flying` | Flying state |
| 24 | `wcarry` | Walk-carry state |
| 26 | `turtleprox` | Turtle proximity |
| 28 | `raftprox` | Raft proximity |
| 30 | `brave` | Bravery stat |
| 32 | `luck` | Luck stat |
| 34 | `kind` | Kindness stat |
| 36 | `wealth` | Wealth (gold) |
| 38 | `hunger` | Hunger counter |
| 40 | `fatigue` | Fatigue counter |
| 42 | `brother` | Active brother (1=Julian, 2=Phillip, 3=Kevin) |
| 44 | `princess` | Princess rescued flag |
| 46 | `hero_sector` | Current sector index |
| 48 | `hero_place` | Current place ID |
| 50 | `daynight` | Day/night counter |
| 52 | `lightlevel` | Derived light level (resaved, not recalculated on load) |
| 54 | `actor_file` | Which actor sprite file is loaded |
| 56 | `set_file` | Which sprite set file is loaded |
| 58 | `active_carrier` | Current carrier actor index |
| 60 | `xtype` | Current extent type |
| 62 | `leader` | Combat leader actor index |
| 64 | `secret_timer` | Secret passage timer |
| 66 | `light_timer` | Torch duration timer |
| 68 | `freeze_timer` | Time-stop duration timer |
| 70 | `cmode` | Current menu mode (FILE returns to GAME) |
| 72 | `encounter_type` | Current encounter type |
| 74–78 | `pad1,pad2,pad3` | Padding — saved but unused on load |

- [ ] Variables declared AFTER `pad3` (offset 80+) are NOT saved: `viewstatus`, `flasher`, `battleflag`, etc.

## C. Extent Persistence (`fmain2.c:1519`)

- [ ] Only `extent_list[0]` and `extent_list[1]` (first 2 entries, 24 bytes total) are saved.
- [ ] These correspond to the mutable bird and turtle extents — the only extents that change at runtime.
- [ ] All other extent entries are fixed and not saved; they must be initialized from the hardcoded table on load.
- [ ] On load: restore `extent_list[0..1]` before calling any `find_place()`.

## D. Post-Load Restoration (`fmain2.c:1541-1548`)

- [ ] Runs only when loading (not saving): `svflag == 0`.
- [ ] Clear encounter tracking: `wt = 0`, `encounter_number = 0`, `encounter_type = 0`, `actors_loading = 0`.
- [ ] Call `shape_read()` — reloads hero sprite graphics for the restored `brother` value.
- [ ] Call `set_options()` — refreshes menu enabled states from restored `stuff[]` inventory.
- [ ] Set `viewstatus = 99` — forces full display redraw.
- [ ] Call `prq(4)` and `prq(7)` — queue vitality and wealth status bar updates.
- [ ] Three `print("")` calls — clear the text scroll area.

## E. Brother Inventory in `mod1save()` (`fmain.c:3621-3631`)

- [ ] Write/read `julstuff[35]`, `philstuff[35]`, `kevstuff[35]` as raw 35-byte blocks — always all three, regardless of active brother.
- [ ] After loading: reassign `stuff = blist[brother-1].stuff` to point at the active brother's array — `fmain.c:3628`.
- [ ] Missile list (`missile_list[6]`, 60 bytes) written immediately after brother arrays — same `mod1save()` call.

## F. Disk Detection Logic (`fmain2.c:1483-1501`)

- [ ] Priority order: hard drive (`image` file present) → `df1:` writable → `df0:` writable and not game disk → prompt for disk.
- [ ] Hard drive path: strip `"df1:"` prefix from filename, use `A.faery` in current directory.
- [ ] Slot letter embedded at `savename[4]` = `'A' + hit` (hit 0–7 → A–H) — `fmain2.c:1502`.
- [ ] Post-save (floppy only): wait for game disk in `df0:` (test for `winpic` file) before resuming — `fmain2.c:1534-1540`.

## G. What is NOT Saved

- [ ] Runtime transients (`viewstatus`, `battleflag`, `frustflag`, `quitflag`) — recalculated or reset on load.
- [ ] NPC screen presence (`witchflag`, `goodfairy`, `witchindex`) — transient display state.
- [ ] Proximity tracking (`nearest`, `nearest_person`, `perdist`) — recalculated each frame.
- [ ] Opened door tile states — live only in `sector_mem`; reset when sector is reloaded.
- [ ] `dayperiod` — derived from `daynight` after load.
- [ ] `pad4..pad7` — declared after the 80-byte window, never saved.

## H. Known Quirks To Preserve (or Deliberately Normalize)

- [ ] `cheat1` persists across saves — cheat mode survives load.
- [ ] `cmode` is saved but irrelevant — game re-enters GAME menu after save/load completes.
- [ ] `lightlevel` is re-saved (not recalculated on load) — but since it's derived from `daynight`, either approach gives the same result.
- [ ] `anix` is saved as part of the misc block AND used to determine block 4 length — consistency between the two reads is assumed.

## I. Minimum Parity Test Matrix

- [ ] Save → load → check hero position equals saved position.
- [ ] Save → load → check all three brother inventories restored correctly.
- [ ] Save → load → active carrier, riding/flying state, and `xtype` restored.
- [ ] Save → load with `freeze_timer > 0` → timer value restored (time-stop survives save).
- [ ] Save → load → `cheat1` value preserved.
- [ ] Save → load → `extent_list[0..1]` reflect last bird/turtle positions.
- [ ] Post-load: `viewstatus == 99`, `actors_loading == 0`, `encounter_type == 0`.
