# Magic Items — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c, fmain2.c
> Cross-refs: [RESEARCH §10](../RESEARCH.md#10-inventory--items), [inventory.md](inventory.md), [combat.md](combat.md), [movement.md](movement.md), [quests.md](quests.md), [day-night.md](day-night.md), [visual-effects.md](visual-effects.md), [game-loop.md](game-loop.md)

## Overview

This file is the single source of truth for every magic item in *The Faery
Tale Adventure*. "Magic" here means any inventory item whose effect is
non-mundane: the seven consumable spells in the `MAGIC` submenu
(`stuff[9..15]`), the Magic Wand (`stuff[4]`) ranged weapon, the three
passive-possession items (Sun Stone `stuff[7]`, Talisman `stuff[22]`,
Crystal Shard `stuff[30]`), and the Bone (`stuff[29]`) — included because it
is the only way to obtain the Crystal Shard. The three global spell-effect
timers (`light_timer`, `secret_timer`, `freeze_timer`) and their per-frame
decrement are also documented here.

The seven `MAGIC` submenu spells share a single dispatcher (`magic_dispatch`)
that gates on ownership and an extent-level "no magic" flag, branches per
slot to apply the effect, then decrements the slot count. The three timers
are advanced by `decrement_timers` in Phase 5 of every tick (see
[game-loop.md#game_tick](game-loop.md#game_tick)) and consulted by AI,
combat, day-night, and palette code as documented inline below. Passive
items do not have their own dispatcher — they are read in-place by the
subsystem that uses them; this file catalogs each read site so a porter can
trace every passive magic effect from one location.

### Catalog

| `stuff[]` | Item | Kind | Effect site (single source of truth) |
|---|---|---|---|
| 4 | Magic Wand | Weapon (`weapon == 5`) | Ranged fireball (mt=9), can damage Necromancer / masked Witch — [combat.md#missile_step](combat.md#missile_step), [combat.md#dohit](combat.md#dohit) |
| 7 | Sun Stone | Passive | Holding lifts masked-Witch (race `0x89`) damage immunity in [combat.md#dohit](combat.md#dohit) (`fmain2.c:233`); USE while `witchflag` is set plays `speak(60)` — [inventory.md#use_dispatch](inventory.md#use_dispatch) |
| 9 | Blue Stone | Spell | Stone-circle ring teleport + heal fall-through — [magic_dispatch](#magic_dispatch) `case 5` (`fmain.c:3326-3354`) |
| 10 | Green Jewel | Spell | `light_timer += 760` — [magic_dispatch](#magic_dispatch) `case 6` (`fmain.c:3306`); brightens dark areas via [visual-effects.md#fade_page](visual-effects.md#fade_page) and [day-night.md#day_fade](day-night.md#day_fade) |
| 11 | Glass Vial | Spell | Heal `vitality += rand8() + 4` clamped at `15 + brave/4` — [magic_dispatch](#magic_dispatch) `case 7` (`fmain.c:3348-3354`) |
| 12 | Crystal Orb | Spell | `secret_timer += 360` — [magic_dispatch](#magic_dispatch) `case 8` (`fmain.c:3307`); recolors Region 9 sky in [visual-effects.md#fade_page](visual-effects.md#fade_page) |
| 13 | Bird Totem | Spell | Overhead-map "+" marker for the hero — [magic_dispatch](#magic_dispatch) `case 9` (`fmain.c:3309-3325`); gated to `region_num <= 7` unless `cheat1` |
| 14 | Gold Ring | Spell | `freeze_timer += 100` — [magic_dispatch](#magic_dispatch) `case 10` (`fmain.c:3308`); inhibited while mounted (`riding > 1`) |
| 15 | Jade Skull | Spell | Mass-kill all live enemies with `race < 7` — [magic_dispatch](#magic_dispatch) `case 11` (`fmain.c:3355-3363`) |
| 22 | Talisman | Win flag | Picking it up triggers `quitflag = TRUE` and the win sequence — [inventory.md#take_command](inventory.md#take_command) (`fmain.c:3244-3247`); also abused by the `.` cheat key (`fmain.c:1299` clears `stuff[22]`) |
| 29 | Bone | Quest item | GIVE to Spectre (race `0x8a`) → consumes Bone, drops Crystal Shard (object 140) — [quests.md#give_item_to_npc](quests.md#give_item_to_npc) (`fmain.c:3501-3503`) |
| 30 | Crystal Shard | Passive | Holding overrides terrain type 12 (crystal wall) blocking in [movement.md#walk_step](movement.md#walk_step) (`fmain.c:1609`); never consumed |

### Global spell timers

| Timer | Set by | Decremented by | Read sites |
|---|---|---|---|
| `light_timer` | Green Jewel (`magic_dispatch` case 6) | `decrement_timers` (`fmain.c:1380`) | `day_fade` raises night-floor by 200 (`fmain2.c:1655`); `fade_page` lifts red-channel to match green when set (`fmain2.c:407`) |
| `secret_timer` | Crystal Orb (`magic_dispatch` case 8) | `decrement_timers` (`fmain.c:1381`) | `fade_page` recolors Region 9 sky to bright green when set (`fmain2.c:383-384`) |
| `freeze_timer` | Gold Ring (`magic_dispatch` case 10) | `decrement_timers` (`fmain.c:1382`) | Skips actor `i > 0` ticks ([game-loop.md#actor_tick](game-loop.md#actor_tick) `fmain.c:1473`); halts `daynight++` (`fmain.c:2023`); jumps past `find_place` and encounter logic (`fmain.c:2048`); blocks NPC melee swings ([game-loop.md#melee_hit_detection](game-loop.md#melee_hit_detection) `fmain.c:2240,2260`); blocks all missile flight (`fmain.c:2267`); flagged on body-search to allow looting frozen actors (`fmain.c:3250`) |

All three timers are zeroed on revive / brother succession (`fmain.c:2852`)
— see [brother-succession.md#revive](brother-succession.md#revive).

## Symbols

No new locals beyond each function's declared parameters. Global
identifiers resolve in [SYMBOLS.md](SYMBOLS.md) or in each function's
`Calls:` header. Numeric literals that are `stuff[]` slot indices, magic
slot offsets (`MAGICBASE = 9`), narr.asm speech indices, or per-spell
constants (`760`, `360`, `100`, `40`, `144`) carry inline citations because
the corresponding named constants are not yet registered. `MAGICBASE` and
`stone_list` are proposed for future SYMBOLS.md entries.

## decrement_timers

Source: `fmain.c:1380-1382`
Called by: `game_tick`
Calls: none

```pseudo
def decrement_timers() -> None:
    """Phase 5: count down the three magic-spell duration timers; saturate at 0."""
    if light_timer != 0:                                     # fmain.c:1380
        light_timer = light_timer - 1
    if secret_timer != 0:                                    # fmain.c:1381
        secret_timer = secret_timer - 1
    if freeze_timer != 0:                                    # fmain.c:1382
        freeze_timer = freeze_timer - 1
```

This three-line block runs unconditionally on every non-paused, non-view
tick — i.e. inside the inner game loop after the pause gate but before
`update_fiery_death_zone`. The three counters are independent and tick at
the same rate; the per-frame decrement is what makes each spell's
"+760 / +360 / +100" charge correspond directly to a number of frames of
effect.

The function is the only writer that decrements the timers. The other two
write paths are: (a) `magic_dispatch` adds charges (see below); and
(b) [brother-succession.md#revive](brother-succession.md#revive) zeros all
three at `fmain.c:2852` so a newly-revived brother never inherits a
predecessor's spell duration.

## magic_dispatch

Source: `fmain.c:3300-3365`
Called by: `option_handler` (via `do_option` `CMODE_MAGIC` branch)
Calls: `event`, `speak`, `rand8`, `bigdraw`, `SetDrMd`, `SetAPen`, `Move`, `Text`, `stillscreen`, `prq`, `colorplay`, `xfer`, `checkdead`, `set_options`, `print`, `stone_list`, `planes`, `secx`, `secy`, `rp_map`, `JAM1`

```pseudo
def magic_dispatch(hit: int) -> None:
    """Consume one magic item and apply its per-slot effect. Decrement stuff[4+hit]; refresh menu when exhausted. Branches that fail their precondition return early without consuming the charge."""
    # Menu label order at fmain.c:501 is: Stone Jewel Vial Orb Totem Ring Skull (hit 5..11).
    # Slot mapping: stuff[4+hit] — 5→stuff[9] Blue Stone, 6→Green Jewel, 7→Glass Vial, 8→Crystal Orb,
    # 9→Bird Totem, 10→Gold Ring, 11→Jade Skull.
    if hit < 5 or stuff[4 + hit] == 0:                       # fmain.c:3303 — no magic owned
        event(21)                                            # fmain.c:3303 — 21 = "if only I had some magic!"
        return
    if extn.v3 == 9:                                         # fmain.c:3304 — extent dampener on magic (astral zone)
        speak(59)                                            # fmain.c:3304 — 59 = "magic doesn't work here"
        return
    if hit == 5:                                             # fmain.c:3326 — Blue Stone: stone-circle teleport
        if hero_sector != 144:                               # fmain.c:3327 — 144 = stone-circle sector id
            return                                           # fmain.c:3347 — not on a circle ⇒ don't consume
        # 85 and 64 are the stone-tile sub-cell offsets inside the 256-wide sector.
        if (hero_x & 255) // 85 != 1 or (hero_y & 255) // 64 != 1:  # fmain.c:3328 — off-tile
            return                                           # fmain.c:3345
        x = hero_x >> 8                                      # fmain.c:3330 — sector-relative X of the stone
        y = hero_y >> 8                                      # fmain.c:3330 — sector-relative Y of the stone
        i = 0
        found = 0
        while i < 11:                                        # fmain.c:3331 — 11 = stone_list pair count
            if stone_list[i + i] == x and stone_list[i + i + 1] == y:  # fmain.c:3332 — match current stone
                i = i + anim_list[0].facing + 1              # fmain.c:3333 — step `facing+1` stones forward
                if i > 10:                                   # fmain.c:3333 — wrap 11-entry ring
                    i = i - 11                               # fmain.c:3333 — 11 stones in ring
                nx = (stone_list[i + i] << 8) + (hero_x & 255)  # fmain.c:3334 — sibling world X (preserve sub-cell)
                ny = (stone_list[i + i + 1] << 8) + (hero_y & 255)  # fmain.c:3335 — sibling world Y
                colorplay()                                  # fmain.c:3336 — 32-frame palette strobe
                xfer(nx, ny, True)                           # fmain.c:3337 — teleport + reload region
                if riding != 0:                              # fmain.c:3338 — drag mount along with hero
                    anim_list[wcarry].abs_x = anim_list[0].abs_x
                    anim_list[wcarry].abs_y = anim_list[0].abs_y
                found = 1
                break
            i = i + 1
        if found == 0:                                       # fmain.c:3347 — no sibling found
            return
        # Case 5 has no `break` in the C source (fmain.c:3347) — fall through into the Glass Vial heal.
        anim_list[0].vitality = anim_list[0].vitality + rand8() + 4  # fmain.c:3349 — 4 = min heal bonus
        cap = 15 + brave // 4                                # fmain.c:3350 — 15 = VIT_BASE, 4 = VIT_BRAVE_DIV
        if anim_list[0].vitality > cap:                      # fmain.c:3350 — clamp overheal
            anim_list[0].vitality = cap
        else:
            print("That feels a lot better!")                # fmain.c:3352 — only print if cap not hit
        prq(4)                                               # fmain.c:3353 — HUD vitality refresh
    elif hit == 6:                                           # fmain.c:3306 — Green Jewel: illumination
        light_timer = light_timer + 760                      # fmain.c:3306 — 760 = illumination ticks
    elif hit == 7:                                           # fmain.c:3348 — Glass Vial: heal
        anim_list[0].vitality = anim_list[0].vitality + rand8() + 4  # fmain.c:3349 — 4 = min heal bonus
        cap = 15 + brave // 4                                # fmain.c:3350 — 15 = VIT_BASE, 4 = VIT_BRAVE_DIV
        if anim_list[0].vitality > cap:                      # fmain.c:3350 — clamp overheal
            anim_list[0].vitality = cap
        else:
            print("That feels a lot better!")                # fmain.c:3352
        prq(4)                                               # fmain.c:3353 — HUD vitality refresh
    elif hit == 8:                                           # fmain.c:3307 — Crystal Orb: secret reveal
        secret_timer = secret_timer + 360                    # fmain.c:3307 — 360 = reveal-secrets ticks
    elif hit == 9:                                           # fmain.c:3309 — Bird Totem: overhead map
        if cheat1 == 0 and region_num > 7:                   # fmain.c:3310 — regions 8,9 locked without cheat
            return                                           # fmain.c:3310 — no consume outside permitted region
        bm_draw = fp_drawing.ri_page.BitMap                  # fmain.c:3311
        planes = bm_draw.Planes                              # fmain.c:3312
        bigdraw(map_x, map_y)                                # fmain.c:3313 — blit the map into the drawing page
        # Convert hero world coord → screen pixel on the map bitmap. 16 = tile size; 4 and 3 = x/y pixel offsets.
        i = (hero_x >> 4) - ((secx + xreg) << 4) - 4         # fmain.c:3315
        j = (hero_y >> 4) - ((secy + yreg) << 4) + 3         # fmain.c:3316
        rp_map.BitMap = bm_draw                              # fmain.c:3317
        SetDrMd(rp_map, JAM1)                                # fmain.c:3318 — JAM1 = opaque-pen mode
        SetAPen(rp_map, 31)                                  # fmain.c:3319 — 31 = palette index for the marker
        if i > 0 and i < 320 and j > 0 and j < 143:          # fmain.c:3320 — 320×143 = map visible area
            Move(rp_map, i, j)                               # fmain.c:3321
            Text(rp_map, "+", 1)                             # fmain.c:3321
        viewstatus = 1                                       # fmain.c:3322 — 1 = VIEWSTATUS_MAP
        stillscreen()                                        # fmain.c:3323 — freeze playfield
        prq(5)                                               # fmain.c:3324 — queue options redraw
    elif hit == 10:                                          # fmain.c:3308 — Gold Ring: time freeze
        if riding > 1:                                       # fmain.c:3308 — 1 = minimum riding code that still allows freeze
            return                                           # fmain.c:3308 — while mounted on swan/dragon: no-op, no consume
        freeze_timer = freeze_timer + 100                    # fmain.c:3308 — 100 = freeze ticks
    elif hit == 11:                                          # fmain.c:3355 — Jade Skull: mass kill
        i = 1
        while i < anix:                                      # fmain.c:3357 — iterate live monster slots
            an = anim_list[i]
            if an.vitality != 0 and an.type == ENEMY and an.race < 7:  # fmain.c:3358 — 7 = cut-off excluding Dark Knight / Loraii / Necromancer
                an.vitality = 0                              # fmain.c:3359
                checkdead(i, 0)                              # fmain.c:3359 — ENEMY death bookkeeping
                brave = brave - 1                            # fmain.c:3359 — 1 bravery lost per killed foe
            i = i + 1
        if battleflag:                                       # fmain.c:3362 — 34 = "all fall before you" recap
            event(34)                                        # fmain.c:3362 — event index 34
    # Consumption epilogue (single-line `if (!--stuff[4+hit])` in the C source at fmain.c:3365).
    if stuff[4 + hit] - 1 == 0:                              # fmain.c:3365 — MAGICBASE=4; decrement slot; 0 ⇒ disable menu row
        stuff[4 + hit] = 0                                   # fmain.c:3365 — MAGICBASE=4
        set_options()                                        # fmain.c:3365 — refresh menus[MAGIC].enabled[]
    else:
        stuff[4 + hit] = stuff[4 + hit] - 1                  # fmain.c:3365 — decrement count by 1
```

**Consumption gate.** The decrement epilogue at `fmain.c:3365` runs only for
branches that fall through to the bottom of the MAGIC switch. Blue Stone
(wrong sector or off-tile or no sibling stone), Bird Totem (`region_num > 7`
without `cheat1`), and Gold Ring (`riding > 1`) all short-circuit with
`return`, preserving the charge so the player is not penalised for a
precondition miss.

**Blue Stone fall-through.** The C `case 5:` at `fmain.c:3326` has no
`break` before `case 7:` at `fmain.c:3348`, so every successful stone
teleport also runs the Glass Vial heal — a single Blue Stone use both
teleports *and* heals. This is documented in
[RESEARCH §10](../RESEARCH.md#10-inventory--items) and verified by the lack of
`break;` between the `xfer()` call and the `vitality += rand8()+4` block.

**Kill-spell race gate.** The `race < 7` test at `fmain.c:3358` means the
Jade Skull spares race codes 7 (Dark Knight), 8 (Loraii), 9 (Necromancer),
and every SETFIG (race bit 7 set). `checkdead(i, 0)` runs the standard
ENEMY death path documented in [combat.md#checkdead](combat.md#checkdead),
including `treasure_probs` drops and `brave`/`kind` bookkeeping.

**Stone-circle ring.** The `stone_list[]` table at `fmain.c:374-376` holds
11 `(x, y)` sector-relative pairs that name the stones in sector 144. Stone
selection advances by `facing + 1` positions in the ring (modulo 11), so
the destination depends on which way the hero is looking when the spell
fires. Direction 0 (NW) jumps one stone forward; direction 7 (W) jumps
eight stones forward, i.e. three back.

## Notes

- **No standalone "use Sun Stone" function.** The Sun Stone effect is
  passive — `dohit` reads `stuff[7]` directly to decide whether to grant
  the masked Witch (race `0x89`) immunity. The USE-submenu hit on the Sun
  Stone (`fmain.c:3462`, slot 8 in the USE menu, `stuff[7]`) only triggers
  the flavour line `speak(60)` when the witch is on screen
  (`witchflag != 0`); it does not toggle any state. See
  [inventory.md#use_dispatch](inventory.md#use_dispatch).
- **Talisman as win condition.** The Talisman has no in-game effect once
  picked up beyond the post-pickup `quitflag = TRUE` latch in
  [inventory.md#take_command](inventory.md#take_command). The cheat key
  `.` (gated on `cheat1`) at `fmain.c:1299` zeroes `stuff[22]` so a
  developer can hold the Talisman without immediately ending the game; this
  is the only writer that clears `stuff[22]` outside of save-load and
  brother succession.
- **Crystal Shard as terrain key.** The Shard is never consumed. It acts
  purely as a hero-only terrain bypass: `proxcheck` in
  [movement.md#walk_step](movement.md#walk_step) returns 12 for the
  crystal-wall tile (terra set 8 image 93), and the line
  `if (stuff[30] && j==12) goto newloc` at `fmain.c:1609` lets the hero
  pass. NPCs are not gated by `stuff[30]` and remain blocked.
- **Bone is one-way.** Once given to the Spectre at `fmain.c:3503`,
  `stuff[29]` is zeroed and replaced by Crystal Shard via `leave_item`.
  There is no other GIVE branch that consumes the Bone, and no other
  source of Crystal Shards in the world database. See
  [quests.md#give_item_to_npc](quests.md#give_item_to_npc).
- **Magic Wand vs the Necromancer/Witch.** The dispatch in `dohit`
  (`fmain2.c:231-234`) requires `weapon >= 4` (Bow or Magic Wand) to bypass
  the race-9 (Necromancer) immunity, and additionally requires
  `stuff[7] != 0` to bypass the race-`0x89` (masked Witch) immunity.
  The Wand's slot count `stuff[4]` is treated as a binary equip flag and is
  not decremented per shot — only its arrow analogue `stuff[8]` would be,
  but the Wand fires fireballs at no ammo cost (`fmain.c:1693` skips the
  `stuff[8]--` line when `mt == 9`).
- **Magic suppressed in the astral plane.** The `extn.v3 == 9` gate at
  `fmain.c:3304` blocks every spell from the MAGIC submenu while the hero
  stands inside any extent whose v3 byte is 9. This is the dampener used
  by the astral-plane region — see
  [astral-plane.md#find_place](astral-plane.md#find_place). The charge is
  not consumed.
- **Menu refresh.** The MAGIC submenu's `enabled[]` row is recomputed by
  `set_options` at `fmain.c:3530` (`menus[MAGIC].enabled[i+5] = stuff_flag(i+9)`).
  `magic_dispatch` calls `set_options()` directly at `fmain.c:3365` only on
  the transition to zero, because the displayed row changes from selectable
  to greyed out at that moment; non-zero decrements leave the menu state
  alone.
