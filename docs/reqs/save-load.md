## 21. Save/Load

### Requirements

| ID | Requirement |
|----|-------------|
| R-SAVE-001 | 8 save slots named `A.faery` through `H.faery`. |
| R-SAVE-002 | Save format: raw sequential binary dump — no headers, no version field, no checksums. Written in big-endian (68000) byte order. |
| R-SAVE-003 | Save data blocks in order: 80-byte misc vars (map_x through pad7), 2-byte region_num, 6-byte anim indices (anix, anix2, mdex), `anix × 22` byte anim_list, 35-byte Julian inventory, 35-byte Phillip inventory, 35-byte Kevin inventory, 60-byte missiles (6 × 10), 24-byte extents (bird/turtle positions), 66-byte global objects (11 × 6), 20-byte map object counts, 20-byte distributed flags, variable per-region object tables. |
| R-SAVE-004 | Typical save file size: ~1,200–1,500 bytes. |
| R-SAVE-005 | Post-load cleanup: clear encounter_number/wt/actors_loading/encounter_type to 0, set viewstatus=99 (force full redraw), reload sprites via `shape_read()`, rebuild menu states via `set_options()`. |
| R-SAVE-006 | **Persisted**: hero position, stats (brave/luck/kind/wealth/hunger/fatigue), all 3 brothers' inventories, daynight cycle, active actors, missiles, world objects (global + all 10 regions), bird/turtle extent positions, carrier state, cheat flag. |
| R-SAVE-007 | **Not saved**: display state (copper lists, rendering buffers), input handler state, music playback position, extent entries 2–21 (static initializers), viewstatus, battleflag, goodfairy. |
| R-SAVE-008 | Save/load accessible from GAME → SAVEX → FILE menu chain. `svflag` determines save vs. load mode. |

### User Stories

- As a player, I can save my game to one of 8 slots and resume later.
- As a player, loading a save restores the complete game state including position, inventory, and quest progress.
- As a player, I see the game properly reinitialize display and encounters after loading.

---


