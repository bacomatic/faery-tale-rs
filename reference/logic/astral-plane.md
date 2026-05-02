# Astral Plane — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c
> Cross-refs: [RESEARCH §9](../RESEARCH.md#9-encounter--spawning), [movement.md#walk_step](movement.md#walk_step), [encounters.md#set_encounter](encounters.md#set_encounter), [day-night.md#setmood](day-night.md#setmood), [carrier-transport.md#carrier_extent_update](carrier-transport.md#carrier_extent_update), [doors.md#xfer](doors.md#xfer), [brother-succession.md#revive](brother-succession.md#revive), [_discovery/astral-plane.md](../_discovery/astral-plane.md)

## Overview

The "astral plane" (in-game: **Spirit Plane**, narr.asm:213 `inside_msg #11`) is not a distinct game mode — it is a **geographical area** inside region 9 (dungeons) bounded by the extent rectangle `(0x2400, 0x8200)..(0x3100, 0x8a00)` with `etype == 52` (`fmain.c:353`). The mechanic is fully described by a single entry-point function, `find_place`, which detects extent transitions and — on first crossing into the astral box — preloads and announces the Loraii (`encounter_chart[8]`, 12 HP, 3–4 bodies per batch, file 9 — `fmain.c:61`).

There is **no "astral tick"**, **no dedicated astral state bit**, and **no "death-while-holding-Amulet" entry path**. The game has no Amulet item (see `inv_list[]` at `fmain.c:391-424`); the closest named item is the Talisman (`stuff[11]`, the victory object). Every observable astral behavior is produced by pre-existing subsystems gated on either `xtype == 52`, `hero_sector == 181`, or the astral coord box:

- **Entry** — `find_place` (below). Fires once per extent crossing, preloading Loraii shapes via `load_actors` + `prep` and setting `encounter_number` so the next [`place_extent_encounters`](encounters.md#place_extent_encounters) pass drains 3–4 Loraii into anim slots 3–6.
- **Spawn placement** — [`set_encounter`](encounters.md#set_encounter) accepts the "void" terrain code 7 inside the astral extent (`fmain.c:2746`), so Loraii may appear on tiles that would otherwise fail `proxcheck`.
- **Pit-fall trap** — the inline `j == 9 && i == 0 && xtype == 52` branch inside [`walk_step`](movement.md#walk_step) (`fmain.c:1766-1775`) puts the hero into `STATE_FALL`, docks `luck -= 2`, and re-kicks `setmood`. Recovery is handled by [`resolve_player_state`](game-loop.md#resolve_player_state) via the `goodfairy` countdown → [`revive(FALSE)`](brother-succession.md#revive), which returns the hero to `(safe_x, safe_y)` with fresh vitality. Ordinary pits outside the astral box are inert.
- **Quicksand drain** — the `hero_sector == 181` clause inside the quicksand branch of [`walk_step`](movement.md#walk_step) (`fmain.c:1782-1791`) teleports the drowning hero to `(0x1080, 34950)` in region 9 (dungeon cells) via [`xfer`](doors.md#xfer), rather than killing them; NPCs are killed outright.
- **Music** — [`setmood`](day-night.md#setmood) at `fmain.c:2939-2941` selects `track[16..19]` (astral song slot 4) whenever the hero's coordinates fall inside the same box that bounds the astral extent. This override beats battle / indoor / day-night and is surpassed only by the death theme.
- **Exit** — walking out of the extent box fires `find_place` again with a new `xtype`, and `carrier_extent_update` zeroes `active_carrier` (`fmain.c:2716`). The stargate door pair (`fmain.c:227-228`, `STAIR`) provides a scripted round-trip between the astral box and the doom-tower area via the generic [`xfer`](doors.md#xfer) primitive.

All spawn/placement/terrain primitives referenced above are normatively specified in their own logic reference docs. This file documents only the entry-point dispatcher and delegates everything else by cross-reference.

## Symbols

Globals already in [SYMBOLS.md](SYMBOLS.md) used here: `xtype`, `extn`, `encounter_type`, `encounter_number`, `encounter_x`, `encounter_y`, `anim_list`, `anix`, `mixflag`, `wt`, `active_carrier`, `hero_x`, `hero_y`, `hero_sector`, `hero_place`, `region_num`, `xreg`, `yreg`, `actors_loading`, `XTYPE_ASTRAL`, `SPECIAL_EXTENT_FLOOR`, `ENCOUNTER_SPREAD`, `ENEMY`, `MAP_FLUX`.

Proposed SYMBOLS.md additions (not applied here — orchestrator review required):

- `TABLE:extent_list` — `fmain.c:338-360` — 23-entry `struct extent[]` (x1, y1, x2, y2, etype, v1, v2, v3) plus a full-map sentinel at the end. The astral row is at `fmain.c:353`.
- `TABLE:place_tbl` — `fmain.c` outdoor place-name map; triple `(sec_lo, sec_hi, msg_idx)` per row, terminated by catch-all.
- `TABLE:inside_tbl` — indoor counterpart; sector ids above 255 map into this table.
- `TABLE:place_msg`, `TABLE:inside_msg` — message-id arrays indexed by the 3rd column of `place_tbl` / `inside_tbl`. Passed to `msg(ms, idx)`.
- `TABLE:encounter_chart` — `fmain.c:53-63` — per-encounter (hitpoints, aggressive, arms, cleverness, treasure, file_id). Entry 8 is Loraii, entry 9 is Necromancer.
- `EXT_COUNT` — `fmain.c` — compile-time entry count of `extent_list` (23 data rows + 1 sentinel).
- `XTYPE_RESCUE = 83` — `fmain.c:2685` — princess-rescue extent etype.
- `XTYPE_SWAN = 60` — `fmain.c:2688` — swan-carrier extent etype.
- `XTYPE_TURTLE_EGGS = 61` — `fmain.c:2688` — turtle-eggs extent etype.
- `ENCOUNTER_LORAII = 8` — `fmain.c:61, 2696` — `encounter_chart` index for Loraii.
- `ASTRAL_SECTOR_DRAIN = 181` — already partially registered as `DRAIN_SINK_SECTOR`; note that sector 181 is the astral plane's drain sector (`fmain.c:1785`, `_discovery/npc-terrain-speed.md:231`).
- `PLACE_TBL_STRIDE = 3` — `fmain.c:2661` — bytes per `place_tbl` / `inside_tbl` row (lo, hi, msg_idx).
- `PLACE_NAME_MOUNTAINS = 4` — `fmain.c:2665` — generic "mountains" `place_msg` index that is remapped per region.
- `PLACE_NAME_ALT_MOUNTAINS = 5` — `fmain.c:2668` — southern-hemisphere mountain label.
- `INDOOR_SECTOR_OFFSET = 256` — `fmain.c:2658` — added to `hero_sector` for interior regions so its high bit distinguishes `inside_tbl` entries from outdoor.
- `REGION_STRADDLE_X_BIT = 64` — `fmain.c:2670` — bit set in `(hero_x>>8) - xreg` when the camera is crossing a region edge (cols 64+ of a 64-col grid).
- `REGION_STRADDLE_Y_BIT = 32` — `fmain.c:2670` — same for `y` across a 32-row grid.

Globals referenced but not registered: `hero_sector` (register as `u16`), `hero_place` (`i16`), `xreg` (`i16`), `yreg` (`i16`), `MAP_FLUX` (macro → `bool`), `EXT_COUNT` (constant).

## find_place

Source: `fmain.c:2647-2720`
Called by: `walk_step` (astral drain teleport tail, `fmain.c:1789`), `check_door` (`fmain.c:1928, 1951`), `no_motion_tick` / `tick_daynight` (`fmain.c:2050`)
Calls: `msg`, `rescue`, `load_actors`, `prep`, `motor_off`, `set_encounter`, `carrier_extent_update`

```pseudo
def find_place(flag: i8) -> None:
    """Resolve hero's current place-name + extent; on extent change, fire rescue / astral Loraii preload / forced spawn / carrier update."""
    while True:
        sec = hero_sector & 255                                                   # fmain.c:2651, 255 = low-byte sector id
        hero_sector = sec
        # fmain.c:2655-2656 — two no-op expression statements recompute the
        # camera-straddle bits without storing them; preserved for fidelity.
        if region_num > 7:                                                        # fmain.c:2657, 7 = last outdoor region
            tbl = inside_tbl                                                      # TABLE:inside_tbl
            ms_table = inside_msg                                                 # TABLE:inside_msg
            hero_sector = hero_sector + 256                                       # fmain.c:2658, 256 = INDOOR_SECTOR_OFFSET
        else:
            tbl = place_tbl                                                       # TABLE:place_tbl
            ms_table = place_msg                                                  # TABLE:place_msg
        idx = 0
        while idx < 256:                                                          # fmain.c:2661, 256 = place-table scan cap
            if sec >= tbl[idx * 3] and sec <= tbl[idx * 3 + 1]:                   # fmain.c:2662, 3 = PLACE_TBL_STRIDE
                break
            idx = idx + 1
        place_idx = tbl[idx * 3 + 2]                                              # fmain.c:2664, 2 = msg-slot offset in row
        if place_idx == 4:                                                        # fmain.c:2665, 4 = PLACE_NAME_MOUNTAINS
            if region_num > 7:                                                    # fmain.c:2666 — keep "mountains" indoors
                place_idx = place_idx
            elif (region_num & 1) != 0:                                           # fmain.c:2667 — odd outdoor regions silence name
                place_idx = 0
            elif region_num > 3:                                                  # fmain.c:2668, 3 = last northern outdoor region
                place_idx = 5                                                     # fmain.c:2668, 5 = PLACE_NAME_ALT_MOUNTAINS
        straddle_x = ((hero_x >> 8) - xreg) & 64                                  # fmain.c:2670, 8 = pixel→supertile shift, 64 = REGION_STRADDLE_X_BIT
        straddle_y = ((hero_y >> 8) - yreg) & 32                                  # fmain.c:2670, 8 = pixel→supertile shift, 32 = REGION_STRADDLE_Y_BIT
        if MAP_FLUX or straddle_x or straddle_y:                                  # fmain.c:2670 — suppress name while scrolling / near edge
            place_idx = 0
        if place_idx and place_idx != hero_place:
            hero_place = place_idx
            if flag:
                msg(ms_table, place_idx)                                          # fmain.c:2673 — speak place-name

        # Match hero pixel against extent_list; fall through to the full-map sentinel row if none hits.
        ext_i = 0
        while ext_i < EXT_COUNT:                                                  # fmain.c:2676 — linear scan
            extn = extent_list[ext_i]                                             # TABLE:extent_list
            if hero_x > extn.x1 and hero_x < extn.x2:
                if hero_y > extn.y1 and hero_y < extn.y2:
                    break
            ext_i = ext_i + 1

        restart = False
        if xtype != extn.etype:
            xtype = extn.etype
            forced = False
            if xtype == 83 and ob_list8[9].ob_stat:                               # fmain.c:2685, 83 = XTYPE_RESCUE; 9 = princess obj slot
                rescue()
                flag = 0
                restart = True
            elif xtype >= 60:                                                     # fmain.c:2687, 60 = XTYPE_SWAN (carrier floor)
                if xtype == 60 or xtype == 61:                                    # fmain.c:2688, 60 = swan, 61 = turtle-eggs
                    if anim_list[3].race != extn.v3 or anix < 4:                  # fmain.c:2689, 3 = carrier slot, 4 = first enemy slot
                        encounter_x = (extn.x1 + extn.x2) // 2
                        encounter_y = (extn.y1 + extn.y2) // 2
                        forced = True
            elif xtype == 52:                                                     # fmain.c:2695, 52 = XTYPE_ASTRAL
                encounter_type = 8                                                # fmain.c:2696, 8 = ENCOUNTER_LORAII
                load_actors()
                prep(ENEMY)
                motor_off()
                actors_loading = False
                # NOTE: no set_encounter loop here — placement is deferred to the next
                # no_motion_tick 14i pass via place_extent_encounters draining encounter_number.
            elif xtype >= 50 and flag == 1:                                       # fmain.c:2699, 50 = SPECIAL_EXTENT_FLOOR
                encounter_x = hero_x
                encounter_y = hero_y
                forced = True
            if forced:
                encounter_type = extn.v3
                mixflag = 0
                wt = 0
                load_actors()
                prep(ENEMY)
                motor_off()
                actors_loading = False
                encounter_number = extn.v1
                anix = 3                                                          # fmain.c:2710, 3 = first enemy anim slot
                while encounter_number and anix < 7:                              # fmain.c:2711, 7 = enemy-slot cap (slots 3..6)
                    if set_encounter(anix, 63):                                   # fmain.c:2712, 63 = ENCOUNTER_SPREAD
                        anix = anix + 1
                    encounter_number = encounter_number - 1

        carrier_extent_update()                                                   # fmain.c:2716-2719 — delegates xtype<70 vs ==70 handling
        if restart:
            continue                                                              # fmain.c:2686 — goto findagain after princess rescue
        break
```

## Notes

- **No "astral state" flag.** The hero's `anim_list[0].state` never takes on an astral-specific value. Being "on the astral plane" is equivalent to having `hero_x, hero_y` inside the extent box (or equivalently `xtype == 52`). Every check that alters behavior re-evaluates the box each frame.
- **`flag` is a behavior switch, not just UI.** `flag != 0` controls place-name display (`msg` at `fmain.c:2672-2673`), but `flag == 1` specifically gates the generic forced-extent branch (`xtype >= 50` at `fmain.c:2700`). In current callsites: `find_place(2)` and `find_place(0)` do not run that branch; only the whirlpool/sink transfer path calls `find_place(1)` (`fmain.c:1789`), which enables it.
- **Entry cost is asynchronous.** `find_place` only *kicks* the disk read for Loraii shapes (`load_actors` → `read_shapes(9)`). `actors_loading` remains False for the astral branch because `prep(ENEMY)` was already called synchronously within `find_place` itself, unlike the generic wilderness path where 14h polls `CheckDiskIO(8)` and calls `prep` only when the read completes. Porters must ensure `prep` waits long enough on the actor-shape channel before the next frame.
- **Cleric warning cross-reference.** The in-game warning "Space may twist, and time itself may run backwards!" (`narr.asm:446-448`, `speak(36)`) describes three astral hazards observable from code: FALL-pits (terrain 9, `fmain.c:1766-1775`), velocity-ice (terrain 7, `reference/_discovery/astral-plane.md`), and backwards-walk lava (terrain 8, environ `k = -3`). All three mechanics live in [`walk_step`](movement.md#walk_step) and [`update_environ`](movement.md#update_environ) and are not re-specified here.
- **Final-boss arena is a separate extent.** The Necromancer extent at `(9563, 33883)..(10144, 34462)` (`fmain.c:344`) sits inside the astral box but has `etype == 53`, so `find_place` fires a second transition when the hero enters it. The Necromancer death drops the Talisman via [`necromancer_death_drop`](quests.md#necromancer_death_drop).
- **Stargate round-trip.** The two `STAIR` rows at `fmain.c:227-228` are indistinguishable from any other door to `check_door`; they teleport via [`xfer`](doors.md#xfer) with region re-derivation and music swap. No astral-specific code runs on door entry — the next `find_place` call picks up the extent change.
- **No death-triggered astral transition exists.** Hero death runs through [`checkdead`](combat.md#checkdead) → `STATE_DYING` → `resolve_player_state` → [`revive`](brother-succession.md#revive), which always respawns the brother at `(safe_x, safe_y)` or at Tambry `(19036, 15755)`. Neither path ever reads an inventory item to decide destination.
