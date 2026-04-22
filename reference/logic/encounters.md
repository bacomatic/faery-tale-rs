# Encounters — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c, fmain2.c, fsubs.asm
> Cross-refs: [RESEARCH §8](../RESEARCH.md#8-encounters--monster-spawning), [_discovery/encounters.md](../_discovery/encounters.md), [logic/game-loop.md](game-loop.md), [logic/combat.md](combat.md)

## Overview

Encounter spawning in Faery Tale has two drivers, both fired from the Phase
14 no-motion tick (`no_motion_tick` in [logic/game-loop.md](game-loop.md#no_motion_tick)).
Sub-phase 14i (`place_extent_encounters`, every 16 daynight ticks) consumes
the pending `encounter_number` counter by picking a wilderness cluster point
around the hero and filling empty or recyclable actor slots with
`set_encounter`. Sub-phase 14j (`roll_wilderness_encounter`, every 32 ticks)
is the upstream source of that counter: when the hero is alone, out-of-carrier,
and in a normal extent (`xtype < 50`), a per-tick `rand64()` vs.
`danger_level` roll picks an `encounter_type`, applies three biome overrides,
and calls `load_actors` to either reuse or disk-load the appropriate shape
file.

`set_loc` is the random-ring spawn origin picker used in 14i.
`set_encounter` is the per-slot placer: it finds a collision-free coordinate
inside a 63-pixel box around the cluster origin, pulls race and weapon
indices out of `TABLE:encounter_chart` and `TABLE:weapon_probs`, and assigns
the resulting ATTACK1/2 or ARCHER1/2 goal. `load_actors` is the async
disk-I/O entry that schedules a shape-file read when `actor_file` doesn't
match the new race's `file_id`; completion is polled at 14h and finalized by
`prep`, which waits on the disk request and builds the sprite mask table.

Forced-encounter extents (etype 50–61, 83) are resolved inside `find_place`
rather than here; this doc covers only the `find_place`-independent periodic
spawning and the shared placement helpers. See
[_discovery/encounters.md](../_discovery/encounters.md) for the extent-table
and etype-category reference.

Two rolled lookups — treasure drops (`roll_treasure`) and enemy weapon
selection (`roll_weapon`) — are documented here as self-contained helpers.
`roll_weapon` is called inline from `set_encounter` on every spawned enemy;
`roll_treasure` is called from the body-search pickup path at
`fmain.c:3270-3273`, which is outside the Phase 14 flow and outside
combat.md's `checkdead`/`aftermath` scope.

## Symbols

No new locals beyond the function-local bindings in each pseudo block. New
globals (`encounter_x`, `encounter_y`, `encounter_type`, `mixflag`, `wt`,
`danger_level`, `actor_file`, `seq_list`) and constants introduced by this
doc are listed in the wave report for orchestrator review; until they land
in [SYMBOLS.md](SYMBOLS.md), all numeric literals outside `{-1, 0, 1, 2}`
carry inline citations as required by [STYLE §7](STYLE.md#7-numeric-literals).

## place_extent_encounters

Source: `fmain.c:2058-2078`
Called by: `no_motion_tick` (Phase 14i, gated on `(daynight & 15) == 0 and encounter_number != 0 and not actors_loading`)
Calls: `rand`, `set_loc`, `px_to_im`, `set_encounter`, `anim_list`, `anix`, `encounter_number`, `encounter_x`, `encounter_y`, `xtype`, `mixflag`, `wt`, `STATE_DEAD`

```pseudo
def place_extent_encounters() -> None:
    """Phase 14i body. Drain `encounter_number` into anim_list[3..6] at a random
    ring of ~150-213 px around the hero; up to 9 candidate cluster points tried."""
    # fmain.c:2059 — seed mixflag from a 31-bit random; two zero-out conditions below.
    mixflag = rand(0, 0x7fffffff)                         # fmain.c:2059, 0x7fffffff = rand() 31-bit range
    # fmain.c:2059 — special extents (xtype>=50 forced encounters) never mix.
    if xtype > 49:                                        # fmain.c:2059, 49 = special-extent floor minus 1
        mixflag = 0
    # fmain.c:2060 — biome-uniform zones (xtype divisible by 4) also never mix.
    if (xtype & 3) == 0:                                  # fmain.c:2060, 3 = 4-value remainder mask
        mixflag = 0
    # fmain.c:2059 — wt is the weapon_probs[] column index this tick, random 0..3.
    wt = rand(0, 3)                                       # fmain.c:2059, 3 = rand4 range

    # fmain.c:2061-2077 — try up to 9 random cluster points; stop at the first walkable one.
    k = 1
    while k < 10:                                         # fmain.c:2061, 10 = placement retry cap
        set_loc()
        if px_to_im(encounter_x, encounter_y) == 0:       # fmain.c:2063, 0 = open terrain code
            # fmain.c:2064-2067 — fill empty slots 3..6 (anix grows).
            while encounter_number != 0 and anix < 7:     # fmain.c:2064, 7 = anim_list last enemy slot + 1
                if set_encounter(anix, 63):               # fmain.c:2065, 63 = default spread box
                    anix = anix + 1
                encounter_number = encounter_number - 1
            # fmain.c:2068-2074 — slots full: overwrite dead occupants in slots 3..6.
            i = 3                                         # fmain.c:2068, 3 = first enemy slot
            while i < 7 and encounter_number != 0:        # fmain.c:2068, 7 = one past last enemy slot
                an = anim_list[i]
                # fmain.c:2071 — race 2 (wraith) corpses can be recycled even if still visible.
                recyclable = (an.state == STATE_DEAD
                              and (an.visible == 0 or an.race == 2))  # fmain.c:2071, 2 = wraith race
                if recyclable:
                    set_encounter(i, 63)                  # fmain.c:2072, 63 = default spread box
                    encounter_number = encounter_number - 1
                i = i + 1
            return                                        # fmain.c:2076 — break out of k-loop on first hit
        k = k + 1
```

## roll_wilderness_encounter

Source: `fmain.c:2080-2092`
Called by: `no_motion_tick` (Phase 14j, gated on `(daynight & 31) == 0 and not actors_on_screen and not actors_loading and active_carrier == 0 and xtype < 50`)
Calls: `rand`, `load_actors`, `encounter_type`, `mixflag`, `danger_level`, `region_num`, `xtype`

```pseudo
def roll_wilderness_encounter() -> None:
    """Phase 14j body. Roll danger_level against rand64; on hit pick an encounter_type,
    apply biome overrides, and queue a shape-file load via load_actors()."""
    # fmain.c:2082-2083 — indoor regions get a +3 danger bias on top of xtype.
    if region_num > 7:                                    # fmain.c:2082, 7 = last outdoor region id
        danger_level = 5 + xtype                          # fmain.c:2082, 5 = indoor danger bias
    else:
        danger_level = 2 + xtype                          # fmain.c:2083, 2 = outdoor danger bias

    # fmain.c:2085 — probability (danger_level+1)/64 per 32-tick window.
    if rand(0, 63) > danger_level:                        # fmain.c:2085, 63 = rand64 range
        return

    # fmain.c:2086 — base race roll: 0=Ogre, 1=Orcs, 2=Wraith, 3=Skeleton.
    encounter_type = rand(0, 3)                           # fmain.c:2086, 3 = rand4 range

    # fmain.c:2087-2088 — swamp biome (xtype 7): replace Wraith with Snake.
    if xtype == 7 and encounter_type == 2:                # fmain.c:2087, 7 = swamp extent etype; 2 = Wraith race
        encounter_type = 4                                # fmain.c:2088, 4 = Snake race

    # fmain.c:2089 — spider region (xtype 8): force Spider and disable mixing.
    if xtype == 8:                                        # fmain.c:2089, 8 = spider-region etype
        encounter_type = 6                                # fmain.c:2089, 6 = Spider race
        mixflag = 0

    # fmain.c:2090 — xtype 49 forces Wraith and disables mixing (no active extent uses this).
    if xtype == 49:                                       # fmain.c:2090, 49 = reserved extent etype (unused)
        encounter_type = 2                                # fmain.c:2090, 2 = Wraith race
        mixflag = 0

    load_actors()                                         # fmain.c:2091
```

## set_loc

Source: `fmain2.c:1714-1720`
Called by: `place_extent_encounters`
Calls: `rand`, `newx`, `newy`, `hero_x`, `hero_y`, `encounter_x`, `encounter_y`

```pseudo
def set_loc() -> None:
    """Pick a cluster origin (encounter_x, encounter_y) on a random ring of
    150..213 pixels around the hero in one of the 8 compass directions."""
    j = rand(0, 7)                                        # fmain2.c:1716, 7 = 8-direction range
    d = 150 + rand(0, 63)                                 # fmain2.c:1717, 150 = min distance; 63 = rand64 range
    encounter_x = newx(hero_x, j, d)                      # fmain2.c:1718
    encounter_y = newy(hero_y, j, d)                      # fmain2.c:1719
```

## set_encounter

Source: `fmain.c:2736-2768`
Called by: `place_extent_encounters`, `find_place` (forced-encounter path at `fmain.c:2706-2712`)
Calls: `bitrand`, `rand`, `proxcheck`, `px_to_im`, `roll_weapon`, `anim_list`, `encounter_chart`, `extn`, `encounter_x`, `encounter_y`, `encounter_type`, `mixflag`, `wt`, `map_x`, `map_y`, `xtype`, `STATE_STILL`, `ENEMY`, `GOAL_ATTACK1`, `GOAL_ARCHER1`

```pseudo
def set_encounter(i: int, spread: int) -> bool:
    """Place one enemy into anim_list[i] near (encounter_x, encounter_y).
    Returns True if a collision-free spot was found within 15 attempts."""
    an = anim_list[i]
    placed = False

    # fmain.c:2741 — Dark Knight (race filter 7) pins to a fixed world coord.
    if extn.v3 == 7:                                      # fmain.c:2741, 7 = DKnight race filter
        xtest = 21635                                     # fmain.c:2741 — hidden-valley DKnight x
        ytest = 25762                                     # fmain.c:2741 — hidden-valley DKnight y
        placed = True
    else:
        # fmain.c:2742-2747 — up to 15 jittered tries inside a 63-px box.
        j = 0
        while j < 15:                                     # fmain.c:2742, 15 = MAX_TRY
            xtest = encounter_x + bitrand(spread) - (spread // 2)   # fmain.c:2744
            ytest = encounter_y + bitrand(spread) - (spread // 2)   # fmain.c:2745
            if proxcheck(xtest, ytest, i) == 0:
                placed = True
                break
            # fmain.c:2746 — astral plane (xtype 52) also accepts the "void" terrain code 7.
            if xtype == 52 and px_to_im(xtest, ytest) == 7:  # fmain.c:2746, 52 = astral-plane etype; 7 = void terrain
                placed = True
                break
            j = j + 1
        if not placed:
            return False

    an.abs_x = xtest
    an.abs_y = ytest
    an.type = ENEMY

    # fmain.c:2753-2755 — mixflag bit 1 pairs races (0<->1, 2<->3) except snakes.
    if (mixflag & 2) != 0 and encounter_type != 4:        # fmain.c:2754, 2 = mix-pair bit; 4 = Snake (never mixed)
        race = (encounter_type & 0xfffe) + rand(0, 1)     # fmain.c:2755, 0xfffe = pair-index mask
    else:
        race = encounter_type
    an.race = race

    # fmain.c:2756 — mixflag bit 2 re-rolls the weapon column each spawn.
    if (mixflag & 4) != 0:                                # fmain.c:2756, 4 = mix-weapon bit
        wt = rand(0, 3)                                   # fmain.c:2756, 3 = rand4 range
    an.weapon = roll_weapon(encounter_chart[race].arms, wt)

    an.state = STATE_STILL
    an.environ = 0
    an.facing = 0

    # fmain.c:2762-2763 — bow bit (weapon 4 or 5) picks ARCHER1/2; else ATTACK1/2.
    clever = encounter_chart[race].cleverness
    if (an.weapon & 4) != 0:                              # fmain.c:2762, 4 = ranged-weapon bit
        an.goal = GOAL_ARCHER1 + clever
    else:
        an.goal = GOAL_ATTACK1 + clever

    an.vitality = encounter_chart[race].hitpoints
    an.rel_x = an.abs_x - map_x - 8                       # fmain.c:2765, 8 = sprite x anchor offset
    an.rel_y = an.abs_y - map_y - 26                      # fmain.c:2766, 26 = sprite y anchor offset
    return True
```

Notes:
- The DKnight branch leaves `j` unread; the historical source has an
  uninitialized-variable read on the `j == MAX_TRY` gate that survives only
  because that path is already committed via `placed = True`. Porters should
  use the explicit `placed` sentinel above.

## prep

Source: `fmain2.c:743-751`
Called by: `no_motion_tick` (Phase 14h, `fmain.c:2054`), `find_place` forced-encounter path (`fmain.c:2701`, `fmain.c:2710`), `load_carrier` (`fmain.c:2797`)
Calls: `WaitDiskIO`, `InvalidDiskIO`, `make_mask`, `seq_list`

```pseudo
def prep(slot: int) -> None:
    """Block until the shape-file disk request for `slot` completes, then build
    the per-sprite mask table for that sequence."""
    WaitDiskIO(8)                                         # fmain2.c:745, 8 = actor-shape I/O channel
    InvalidDiskIO(8)                                      # fmain2.c:746, 8 = actor-shape I/O channel
    make_mask(seq_list[slot].location,
              seq_list[slot].maskloc,
              seq_list[slot].width,
              seq_list[slot].height,
              seq_list[slot].count)                       # fmain2.c:748-749
```

## load_actors

Source: `fmain.c:2722-2733`
Called by: `roll_wilderness_encounter`, `find_place` (forced-encounter path at `fmain.c:2699`, `fmain.c:2707`)
Calls: `rand`, `read_shapes`, `encounter_chart`, `seq_list`, `extn`, `encounter_number`, `encounter_type`, `actor_file`, `actors_loading`, `anix`, `active_carrier`, `ENEMY`

```pseudo
def load_actors() -> None:
    """Set the pending encounter_number and, if the new race lives in a different
    shape file than is currently loaded, kick off an async disk read."""
    # fmain.c:2725 — herd size: v1 guaranteed + 0..v2-1 random bonus.
    encounter_number = extn.v1 + rand(0, extn.v2 - 1)     # fmain.c:2725 — rnd(n) = rand() % n

    # fmain.c:2726-2733 — shape-file swap only when the new race's file_id differs.
    new_file = encounter_chart[encounter_type].file_id
    if actor_file != new_file:                            # fmain.c:2726
        actor_file = new_file
        anix = 3                                          # fmain.c:2728, 3 = first enemy slot (clears slots 3..6)
        nextshape = seq_list[ENEMY].location              # fmain.c:2729
        read_shapes(actor_file)                           # fmain.c:2730
        actors_loading = True
        active_carrier = 0                                # fmain.c:2732 — drop any active bird/turtle/dragon
```

## roll_treasure

Source: `fmain.c:3270-3273`
Called by: body-search pickup path at `fmain.c:3270-3273` (defer: inventory/pickup doc)
Calls: `rand`, `encounter_chart`, `treasure_probs`

```pseudo
def roll_treasure(race: int) -> int:
    """Pick one inv_list[] slot index for a corpse drop, or 0 for nothing.
    Setfig corpses (race bit 7 set) never drop treasure."""
    # fmain.c:3271 — setfig bit marks NPCs; they carry no rolled loot.
    if (race & 0x80) != 0:                                # fmain.c:3271, 0x80 = setfig race-high bit
        return 0                                          # fmain.c:3271 — no drop
    # fmain.c:3272-3273 — pick one of 8 probs-table entries for this race's treasure tier.
    tier = encounter_chart[race].treasure
    col = rand(0, 7)                                      # fmain.c:3272, 7 = rand8 range
    return treasure_probs[tier * 8 + col]                 # fmain.c:3273, 8 = columns per treasure tier
```

## roll_weapon

Source: `fmain.c:2757-2758`, `fmain2.c:860-868`
Called by: `set_encounter`
Calls: `weapon_probs`

```pseudo
def roll_weapon(arms: int, col: int) -> int:
    """Look up one weapon code (0=none,1=dirk,2=mace,3=sword,4=bow,5=wand,8=touch)
    from the `arms` row and `col` column (0..3) of weapon_probs[]."""
    return weapon_probs[arms * 4 + col]                   # fmain.c:2757-2758, 4 = columns per arms row
```

## Notes

- **14i vs 14j coupling.** `roll_wilderness_encounter` (14j) only sets
  `encounter_number`; the actual slot fill happens in `place_extent_encounters`
  (14i) on a later tick. This means a 32-tick danger roll can translate into
  several 16-tick placement passes if more than 4 monsters are queued.
- **Forced-encounter reuse.** The same `set_encounter` / `load_actors` / `prep`
  trio is used by the `find_place` forced-extent path (etypes 50–61, 83).
  `find_place` sets `encounter_x = hero_x` and `encounter_y = hero_y` directly,
  bypassing `set_loc`, then calls the same primitives. See
  [_discovery/encounters.md](../_discovery/encounters.md#find_place) for the
  extent-category switch.
- **Mixflag zeroing.** `mixflag` is freshly rolled each 14i pass, so its bit
  layout is only meaningful for the duration of one placement batch. Bit 1
  pairs races (ogre↔orc, wraith↔skeleton) and bit 2 rerolls the weapon
  column; the 31-bit random value exposes every other bit to higher-layer
  code that reads `mixflag`, but no other bit is currently consumed.
- **Async I/O gate.** `load_actors` returning does not mean the shape file is
  ready. `no_motion_tick` Phase 14h polls `CheckDiskIO(8)` and calls `prep`
  when the read completes; `actors_loading` gates 14i/14j from spawning into
  a half-loaded shape table.
- **Astral plane terrain acceptance.** The `px_to_im == 7` branch in
  `set_encounter` only fires inside the astral-plane extent (xtype 52). Code 7
  is the void "no-terrain" marker that would otherwise fail `proxcheck`.
