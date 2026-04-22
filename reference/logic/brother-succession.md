# Brother Succession — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c
> Cross-refs: [RESEARCH §17](../RESEARCH.md#17-brother-succession), [STORYLINE.md](../STORYLINE.md), [game-loop.md#resolve_player_state](game-loop.md#resolve_player_state)

## Overview

The hero role is played by three brothers in sequence: Julian → Phillip → Kevin. The global `brother` counter tracks which brother is active (0 before first call, 1..3 during play, >3 after the third brother dies). Only one brother is alive at any time; the inactive brothers' inventories persist in the arrays `julstuff`, `philstuff`, `kevstuff` (each `ARROWBASE = 35` bytes, `fmain.c:430`) and the global `stuff` pointer is re-seated at every transition (`fmain.c:2845`).

Starting a new game and losing the active brother both run through the same function, `revive(new)`. When `new == True`, `revive` places the dead brother's bones + ghost in the world, loads the next brother's starting stats from `blist[]`, clears the new brother's inventory except for a dirk, teleports to Tambry at `(19036, 15755)`, shows two placards, and resets the time-of-day. When `new == False` (fairy rescue or pit-fall recovery), `revive` skips all of that and simply restores the current brother at `(safe_x, safe_y)` with fresh vitality. The transition is gated by [`resolve_player_state`](game-loop.md#resolve_player_state) (phase 7 of the game loop), which counts `goodfairy` down after death and picks the rescue or succession branch based on `luck` and `state`.

When a later brother finds a dead brother's bones (world object id `28`), the old brother's 31 pre-gold inventory slots are added to the current brother's inventory and both ghost set-figures are retired (`fmain.c:3173-3178`). Stats (brave/luck/kind/wealth) never transfer — they are always re-loaded from `blist[]`.

A companion cross-reference: the `goodfairy` counter and its DEAD/FALL gating are documented in [`game-loop.md#resolve_player_state`](game-loop.md#resolve_player_state); `checkdead`, which decrements `luck` on hero death and triggers the ladder, is documented in [`combat.md#checkdead`](combat.md#checkdead).

## Symbols

```pseudo
# Locals declared below refer to these globals (proposed additions to SYMBOLS.md,
# see final report):
#   brother: i16            # fmain.c:567 — active brother index (0 pre-game, 1..3 in play, >3 game-over)
#   princess: i16           # fmain.c:568 — rescued-princess counter (not per-brother)
#   wealth: i16             # fmain.c:562 — hero gold total
#   fatigue: i16            # fmain.c:562 — hero fatigue
#   safe_x, safe_y, safe_r  # fmain.c:558-559 — last safe-zone coord + region
#   new_region: u16         # fmain.c — pending region for load_all
#   actor_file: i16         # fmain.c:573
#   set_file: i16           # fmain.c:573
#   lightlevel: u16         # fmain.c:571
#   secret_timer, light_timer: i16  # fmain.c:577
#   mdex: i16               # fmain.c — missile-list write cursor
#   quitflag: bool          # fmain.c:589
#   fiery_death: bool       # fmain.c — hero-in-lava-tile flag
#   julstuff, philstuff, kevstuff: list[u8]  # fmain.c:432 — per-brother inventory arrays of ARROWBASE
#   blist: list[Bro]        # fmain.c:2806-2812 — TABLE:blist (brave/luck/kind/wealth/stuff-ptr)
#   ob_listg: list          # fmain2.c:1001 — global-object list (bones, ghosts, …)
#   ob_list8: list          # fmain.c — inside-region object list (slot 9 is the princess)
#   inv_list: list          # fmain.c — inventory-slot metadata
#   hero_place: u16         # fmain.c:569
#   handler_data: object    # fmain.c — input-handler shared state (.laydown, .pickup)

ARROWBASE            = 35        # fmain.c:430 — per-brother inventory length
GOLDBASE             = 31        # fmain.c:429 — first gold-counter slot
STARTING_DIRK        = 1         # fmain.c:2850 — WEAPON_DIRK and stuff[0] initial count
TAMBRY_SPAWN_X       = 19036     # fmain.c:2853 — new-brother spawn X
TAMBRY_SPAWN_Y       = 15755     # fmain.c:2853 — new-brother spawn Y
TAMBRY_REGION        = 3         # fmain.c:2853 — Tambry region number
RAFT_INIT_X          = 13668     # fmain.c:2819 — anim_list[1] raft reset X
RAFT_INIT_Y          = 14470     # fmain.c:2819 — anim_list[1] raft reset Y
SETFIG_INIT_Y        = 15000     # fmain.c:2824 — anim_list[2] set-figure reset Y
MAP_CAMERA_OFFSET_X  = 144       # fmain.c:2898 — map_x = hero_x - 144
MAP_CAMERA_OFFSET_Y  = 90        # fmain.c:2899 — map_y = hero_y - 90
VIT_BASE             = 15        # fmain.c:2901 — vitality floor in 15+brave/4
VIT_BRAVE_DIV        = 4         # fmain.c:2901 — vitality bravery divisor
DAYNIGHT_RESET       = 8000      # fmain.c:2904 — morning-of-resurrection tick
LIGHTLEVEL_RESET     = 300       # fmain.c:2904 — daylight brightness
VIEWSTATUS_CORRUPT   = 99        # fmain.c:2856 — full-screen placard takeover
VIEWSTATUS_PLACARD   = 2         # fmain.c:2910 — hold final placard (brother > 3)
VIEWSTATUS_PLAYFIELD = 3         # fmain.c:2910 — normal playfield
PRINCESS_OBJ_SLOT    = 9         # fmain.c:2843 — ob_list8 index for the princess set-figure
BONES_OBJ_ID         = 28        # fmain.c:3174 — "% of <brother>'s bones" object id
GHOST_OFFSET         = 2         # fmain.c:2840 — ob_listg[brother+2] addresses the ghost set-figure
OB_STAT_BONES_ON_GROUND = 1      # fmain.c:2839 — ob_stat value meaning "lying in world"
OB_STAT_SETFIG_ACTIVE   = 3      # fmain.c:2840 — ob_stat value meaning "live set-figure"
PLACARD_HOLD_TICKS   = 120       # fmain.c:2869 — initial placard on-screen time
PLACARD_GAP_TICKS    = 80        # fmain.c:2872 — delay before second placard
PLACARD_CLEAR_TICKS  = 10        # fmain.c:2873 — hold between clear + second text
GAME_OVER_DELAY      = 500       # fmain.c:2871 — end-of-tale pause
ACTOR_FILE_BROTHER   = 6         # fmain.c:2882 — cfile id used for all three brother sprite sets
SET_FILE_TAMBRY      = 13        # fmain.c:2882 — set-figure pack for Tambry
ANIX_DEFAULT         = 3         # fmain.c:2906 — baseline active-actor count after revive
GAME_OVER_THRESHOLD  = 3         # fmain.c:2871, 2910 — brother > 3 means all three dead
```

## revive

Source: `fmain.c:2814-2912`
Called by: `main` (game start, `fmain.c:1245`), `resolve_player_state` (death / fall / rescue, `fmain.c:1390-1393`)
Calls: `stopscore`, `map_adjust`, `map_message`, `SetFont`, `placard_text`, `placard`, `Delay`, `SetRast`, `SetAPen`, `RectFill`, `shape_read`, `message_off`, `event`, `print_cont`, `fade_down`, `load_all`, `set_options`, `prq`, `setmood`, `TABLE:blist`

```pseudo
def revive(new: bool) -> None:
    """Start or restart the active brother; place ghost/bones when succeeding."""
    # Reset the raft actor (anim_list[1]).                                   fmain.c:2818-2821
    anim_list[1].type = RAFT
    anim_list[1].abs_x = RAFT_INIT_X
    anim_list[1].abs_y = RAFT_INIT_Y
    anim_list[1].index = 0
    anim_list[1].weapon = 0
    anim_list[1].environ = 0

    # Reset the companion set-figure actor (anim_list[2]).                   fmain.c:2823-2826
    anim_list[2].type = SETFIG
    anim_list[2].abs_x = RAFT_INIT_X
    anim_list[2].abs_y = SETFIG_INIT_Y
    anim_list[2].index = 0
    anim_list[2].weapon = 0

    # Re-seat the hero actor (anim_list[0]).                                 fmain.c:2828-2830
    an = anim_list[0]
    an.type = PHIL
    an.goal = GOAL_USER

    # Clear pending pickup/drop and all battle flags.                        fmain.c:2832-2833
    handler_data.laydown = 0
    handler_data.pickup = 0
    battleflag = False
    goodfairy = 0
    mdex = 0

    if new:
        stopscore()                                                          # fmain.c:2835

        # -------- Place dead brother's bones + ghost (only for brothers 1 & 2)
        if brother > 0 and brother < GAME_OVER_THRESHOLD:                    # fmain.c:2837 — 3 = guard against oob
            ob_listg[brother].xc = hero_x                                    # fmain.c:2838
            ob_listg[brother].yc = hero_y                                    # fmain.c:2838
            ob_listg[brother].ob_stat = OB_STAT_BONES_ON_GROUND              # fmain.c:2839
            ob_listg[brother + GHOST_OFFSET].ob_stat = OB_STAT_SETFIG_ACTIVE # fmain.c:2840

        # Reset the princess set-figure so the new brother can rescue her.   fmain.c:2843
        ob_list8[PRINCESS_OBJ_SLOT].ob_stat = OB_STAT_SETFIG_ACTIVE

        # -------- Load next brother's stats + inventory pointer             fmain.c:2844-2847
        br = blist[brother]                                                  # TABLE:blist
        brave = br.brave
        luck = br.luck
        kind = br.kind
        wealth = br.wealth
        stuff = br.stuff                                                     # switches active inventory array
        brother = brother + 1                                                # advance 0→1→2→3→4

        # -------- Clear pre-gold inventory and give a dirk.                 fmain.c:2849-2850
        i = 0
        while i < GOLDBASE:
            stuff[i] = 0
            i = i + 1
        stuff[0] = STARTING_DIRK                                             # first slot = Dirk count
        an.weapon = WEAPON_DIRK

        # -------- Reset timers + safe-zone + camera                         fmain.c:2852-2855
        secret_timer = 0
        light_timer = 0
        freeze_timer = 0
        safe_x = TAMBRY_SPAWN_X
        safe_y = TAMBRY_SPAWN_Y
        region_num = TAMBRY_REGION
        safe_r = TAMBRY_REGION
        map_adjust(safe_x, safe_y)
        viewstatus = VIEWSTATUS_CORRUPT                                      # full-screen takeover for placards
        actors_on_screen = True
        actors_loading = False

        # -------- First placard: who is setting out, or "stay at home".     fmain.c:2860-2869
        map_message()
        SetFont(rp, afont)
        if brother == 1:
            placard_text(0)                                                  # fmain.c:2862, msg1 — Julian
            # Clear drawing page bitmap then restore viewing page.           fmain.c:2863-2864
            SetRast(rp, 0)                                                   # fmain.c:2863 — paraphrase of rp_map.BitMap swap+clear
        elif brother == 2:
            placard_text(1)                                                  # fmain.c:2866, msg2 — Julian lost
        elif brother == GAME_OVER_THRESHOLD:                                 # brother == 3 — Kevin
            placard_text(3)                                                  # fmain.c:2867, msg4 — Phillip lost
        else:
            placard_text(5)                                                  # fmain.c:2868, msg6 — end of tale
        placard()
        Delay(PLACARD_HOLD_TICKS)

        # -------- Game over (all three dead) OR second placard.             fmain.c:2871-2878
        if brother > GAME_OVER_THRESHOLD:
            quitflag = True
            Delay(GAME_OVER_DELAY)
        elif brother > 1:
            Delay(PLACARD_GAP_TICKS)
            SetAPen(rp, 0)
            RectFill(rp, 13, 13, 271, 107)                                   # fmain.c:2873 — clear placard box (13,13)..(271,107)
            Delay(PLACARD_CLEAR_TICKS)
            SetAPen(rp, 24)                                                  # fmain.c:2874, 24 = placard ink colour
            if brother == 2:
                placard_text(2)                                              # fmain.c:2875, msg3 — Phillip sets out
            else:
                placard_text(4)                                              # fmain.c:2875, msg5 — Kevin sets out
            Delay(PLACARD_HOLD_TICKS)

        # -------- Restore text font + load brother sprites.                 fmain.c:2880-2882
        SetFont(rp, tfont)
        rp = rp_text
        actor_file = ACTOR_FILE_BROTHER
        set_file = SET_FILE_TAMBRY
        shape_read()                                                         # loads read_shapes(brother-1) internally

        # -------- Scroll a narrative line if the tale continues.            fmain.c:2884-2892
        if brother < 4:                                                      # 4 = just past game-over threshold
            message_off()
            hero_place = 2                                                   # 2 = Tambry place-name index
            event(9)                                                         # msg 9 — "% started the journey …"
            if brother == 1:
                print_cont(".")
            elif brother == 2:
                event(10)                                                    # msg 10 — "as had his brother before him."
            elif brother == GAME_OVER_THRESHOLD:
                event(11)                                                    # msg 11 — "as had his brothers before him."
    else:
        fade_down()                                                          # fmain.c:2894 — fairy rescue / fall return

    # -------- Common finalisation (both paths).                             fmain.c:2896-2911
    hero_x = safe_x
    hero_y = safe_y
    an.abs_x = safe_x
    an.abs_y = safe_y
    map_x = hero_x - MAP_CAMERA_OFFSET_X
    map_y = hero_y - MAP_CAMERA_OFFSET_Y
    new_region = safe_r
    load_all()                                                               # pulls region_num and redraws world
    an.vitality = VIT_BASE + (brave // VIT_BRAVE_DIV)                        # fmain.c:2901 — 15 + brave/4
    an.environ = 0
    an.state = STATE_STILL
    an.race = -1
    daynight = DAYNIGHT_RESET
    lightlevel = LIGHTLEVEL_RESET
    hunger = 0
    fatigue = 0
    anix = ANIX_DEFAULT
    set_options()
    prq(7)                                                                   # fmain.c:2909, 7 = vitality print-queue
    prq(4)                                                                   # fmain.c:2909, 4 = stats print-queue
    if brother > GAME_OVER_THRESHOLD:
        viewstatus = VIEWSTATUS_PLACARD
    else:
        viewstatus = VIEWSTATUS_PLAYFIELD
        setmood(True)
    fiery_death = False
    xtype = 0
```

### Notes on control flow

- **Game start** runs the same code path: `main` calls `revive(True)` at `fmain.c:1245` with `brother == 0`. The `brother > 0 && brother < 3` guard at `fmain.c:2837` suppresses the bones/ghost placement on the very first call; every other predicate reads the freshly-loaded Julian record.
- **Kevin's death** increments `brother` from 3 to 4 at `fmain.c:2847` *before* the `brother > 3` test at `fmain.c:2871`. The stat load at `fmain.c:2844-2846` reads `blist[3]`, one past the end of the declared `blist` array. The values are never observed because the immediate `brother > 3` branch sets `quitflag = True` and the tail of `revive` only uses `brave` once (for vitality, on a character the player will never control). Logged in PROBLEMS.md as an out-of-bounds read.
- **Fairy / fall path** (`new == False`) skips every block gated by `if new`, including the `stopscore`, bones, stats reload, inventory wipe, placards, sprite reload, and narrative event. `safe_x/safe_y/safe_r` were set by an earlier gameplay event (entering a safe zone); vitality is restored to `15 + brave/4`, hunger and fatigue are cleared, and the current brother resumes.
- **Inventory carry-over** across a fairy rescue is total (`stuff` is not touched in the `else` branch), whereas a succession wipes slots `0..GOLDBASE-1` but leaves the gold slots `GOLDBASE..ARROWBASE-1` intact on the *new* brother — however those slots were `0` to begin with on a fresh brother's array, and the dead brother's gold persists in his own stored array. Recovery happens via `pickup_brother_bones`.

## pickup_brother_bones

Source: `fmain.c:3174-3178`
Called by: `do_option` (CMODE_ITEMS → PICK UP NEAREST, `fmain.c:3143-3179`)
Calls: `announce_treasure`

```pseudo
def pickup_brother_bones(x: i16) -> None:
    """Merge a dead brother's inventory into the current brother's when bones are picked up.

    `x` is the picked object's `anim_list[nearest].vitality & 0x7f`, which for bones (id 28)
    equals the dead brother's 1-based index (1 = Julian, 2 = Phillip). Kevin is always the
    last active brother, so his bones never appear.
    """
    announce_treasure("his brother's bones.")                                 # fmain.c:3173
    # Both ghost set-figures retire regardless of which bones were found.    fmain.c:3174
    ob_listg[3].ob_stat = 0                                                   # 3 = Julian's ghost slot
    ob_listg[4].ob_stat = 0                                                   # 4 = Phillip's ghost slot
    k = 0
    while k < GOLDBASE:
        if x == 1:
            stuff[k] = stuff[k] + julstuff[k]                                 # fmain.c:3176 — merge Julian's items
        else:
            stuff[k] = stuff[k] + philstuff[k]                                # fmain.c:3177 — merge Phillip's items
        k = k + 1
```

### Interaction with `revive`

- The bones on the ground at `(ob_listg[brother].xc, ob_listg[brother].yc)` were placed by the prior `revive(True)` call at the death coordinates (`fmain.c:2838-2839`).
- Picking up either set of bones retires *both* ghost set-figures (`ob_listg[3]` and `ob_listg[4]`, `fmain.c:3174`) — so a player who finds Phillip's bones first also dispels Julian's ghost.
- Gold (slots `GOLDBASE..ARROWBASE-1`) is not transferred. Only the 31 item/magic/key/stat slots are merged.

## See also

- [game-loop.md#resolve_player_state](game-loop.md#resolve_player_state) — the phase 7 ladder that decides between fairy rescue, fall return, and brother succession based on `goodfairy`, `luck`, and `state`.
- [combat.md#checkdead](combat.md#checkdead) — sets `STATE_DYING`, decrements `luck` on hero death, and thereby arms the succession branch.
- [save-load.md#mod1save](save-load.md#mod1save) — serialises `julstuff`, `philstuff`, `kevstuff`, then re-seats `stuff = blist[brother-1].stuff` on load.
