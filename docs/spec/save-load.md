## 24. Save/Load System

### 24.1 File Format

Raw sequential binary dump with no headers, no version field, no checksums. 8 slots named `A.faery` through `H.faery`. Written in native big-endian (68000) byte order via AmigaDOS `Write()`/`Read()`.

### 24.2 Save Data Layout

| Order | Data | Size |
|-------|------|------|
| 1 | Misc variables (map_x through pad7) | 80 bytes |
| 2 | `region_num` | 2 bytes |
| 3 | `anix`, `anix2`, `mdex` | 6 bytes |
| 4 | Active actor list (`anim_list[0..anix-1]`) | anix × 22 bytes |
| 5 | Julian's inventory (`julstuff`) | 35 bytes |
| 6 | Phillip's inventory (`philstuff`) | 35 bytes |
| 7 | Kevin's inventory (`kevstuff`) | 35 bytes |
| 8 | Missile list | 6 × 10 bytes (60) |
| 9 | Extent entries 0–1 (bird/turtle positions) | 24 bytes |
| 10 | Global object list (`ob_listg`, 11 × 6) | 66 bytes |
| 11 | Per-region object counts (`mapobs`) | 20 bytes |
| 12 | Per-region distributed flags (`dstobs`) | 20 bytes |
| 13 | All regional object tables | Σ mapobs[i]×6 |

Typical file size: ~1,200–1,500 bytes.

### 24.3 The 80-Byte Misc Variables Block

Contiguous from `map_x`:
- `map_x, map_y, hero_x, hero_y` (8 bytes)
- `safe_x, safe_y, safe_r` (6 bytes)
- `img_x, img_y, cheat1` (6 bytes)
- `riding, flying, wcarry, turtleprox, raftprox` (10 bytes)
- `brave, luck, kind, wealth, hunger, fatigue` (12 bytes)
- `brother, princess, hero_sector, hero_place` (8 bytes)
- `daynight, lightlevel, actor_file, set_file` (8 bytes)
- `active_carrier, xtype, leader` (6 bytes)
- `secret_timer, light_timer, freeze_timer` (6 bytes)
- `cmode, encounter_type` (4 bytes)
- `pad1–pad7` (14 bytes)

`cheat1` persists at byte offset 18 — only way to enable is hex-editing a save file.

**Note — P21 BSS mismatch (repo `fmain` binary only):** The original shipped game writes this block correctly in declaration order. The `fmain` binary included in the research repository was compiled with a later Aztec C toolchain that scattered these BSS globals across memory, so `saveload(&map_x, 80)` in that binary captures `map_x` at offset 0 followed by 78 bytes of unrelated variables — `hero_x`, `hero_y`, `daynight`, `riding`, and the stats globals are never saved or restored. This is a bug in the repo executable only; it does **not** reflect the original game's behavior. The port **must** implement the correct, declared-order layout documented above. Save files produced by the repo `fmain` binary are incompatible with original-game saves. See PROBLEMS.md P21.

### 24.4 Disk Detection

`savegame()` probes writable media in priority order: hard drive (`Lock("image")`) → df1: → df0: (if not game disk, verified by absence of `winpic`). Falls back to prompting for disk insertion with 30-second timeout.

### 24.5 Post-Load Cleanup

Reset on load: `encounter_number`, `wt`, `actors_loading`, `encounter_type` all cleared to 0. `viewstatus` set to 99 (force full redraw). `shape_read()` reloads all sprite data. `set_options()` rebuilds menu states from inventory.

### 24.6 Persistence Rules

**Persisted**: Hero position, stats (brave/luck/kind/wealth/hunger/fatigue), all 3 brothers' inventories (35 items each), daynight cycle, active actors, missiles, world objects (global + all 10 regions), bird/turtle extent positions, carrier state, cheat flag.

**Reset on load**: encounter_number, wt, actors_loading, encounter_type, viewstatus.

**Not saved**: Display state (copper lists, rendering buffers), input handler state, music playback position, extent entries 2–21 (static initializers), battleflag, goodfairy, etc.

---


