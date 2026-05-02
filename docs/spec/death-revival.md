## 20. Death & Revival

### 20.1 Death Detection (`checkdead`)

Triggers when `an->vitality < 1` and actor state is not already DYING or DEAD:

```
an->vitality = 0
an->tactic = 7
an->goal = DEATH
an->state = DYING
```

**Hero death** (`i == 0`): `event(dtype)` (death message by type), `luck -= 5`, `setmood(TRUE)` (death music).

**NPC kill** (`i != 0`):
- If SETFIG and not witch (0x89): `kind -= 3`
- If DreamKnight (race 7): `speak(42)`
- Always: `brave++`

Death types: 5 = combat, 6 = drowning, 27 = lava. DYING → DEAD transition occurs when `tactic` counts down to 0 during the death animation.

### 20.2 Fairy Rescue Mechanism

Activates when hero's state is DEAD or FALL. Uses `goodfairy` counter (`unsigned char`, starts at 0, wraps to 255 on first decrement).

**Timeline after hero enters DEAD/FALL state:**

| `goodfairy` Range | Frames | Behavior |
|-------------------|--------|----------|
| 255–200 | 2–57 | **Death sequence plays.** Death animation and death song always complete fully before any rescue decision. |
| 199–120 | 58–137 | **Luck gate**: `luck < 1` → `revive(TRUE)` (brother succession). FALL state → `revive(FALSE)` (non-lethal recovery). `luck >= 1` and DEAD → countdown continues toward fairy rescue. |
| 119–20 | 138–237 | Fairy sprite visible, flying toward hero. `battleflag = FALSE`. AI suspended. |
| 19–2 | 238–255 | Resurrection glow effect. |
| 1 | 256 | `revive(FALSE)` — fairy rescues hero, same brother continues. |

**Key design property**: The luck gate is **fully deterministic** with no random element. `checkdead()` sets `luck -= 5` on hero death. Luck cannot change during DEAD state because:

- `checkdead` is guarded by `state != DYING && state != DEAD`
- Pit fall luck loss requires movement
- Sorceress luck gain requires TALK interaction

If `luck >= 1` when the gate first fires, fairy rescue is guaranteed. Since luck cannot change during DEAD state, the gate is effectively a one-time decision at the moment `goodfairy` first drops below 200. FALL state always gets `revive(FALSE)` regardless of luck (pit falls are non-lethal).

### 20.3 Brother Base Stats (`blist[]`)

| Brother | `brother` | brave | luck | kind | wealth | Starting HP (`15 + brave/4`) | Max Fairy Rescues |
|---------|-----------|-------|------|------|--------|------------------------------|-------------------|
| Julian | 1 | 35 | 20 | 15 | 20 | 23 | 3 |
| Phillip | 2 | 20 | 35 | 15 | 15 | 20 | 6 |
| Kevin | 3 | 15 | 20 | 35 | 10 | 18 | 3 |

Each brother has an independent 36-byte inventory array in memory (35 active slots 0–34 plus `ARROWBASE = 35` accumulator): `julstuff`, `philstuff`, `kevstuff`. The save-file payload serializes 35 bytes per brother (the accumulator is transient and not persisted — see §24).

Design: Julian is the strongest fighter (highest bravery/HP), Phillip has the most fairy rescues available (highest luck), Kevin is the diplomat (highest kindness, weakest combatant).

### 20.4 `revive()` — Resurrection and Succession

`revive(new)`: `new = TRUE` for brother succession, `new = FALSE` for fairy rescue/fall recovery.

#### Common Setup (both paths)

- `anim_list[1]` placed as RAFT, `anim_list[2]` as SETFIG (reset carriers)
- `handler_data.laydown = handler_data.pickup = 0`
- `battleflag = goodfairy = mdex = 0`

#### New Brother Path (`new == TRUE`)

1. **Place dead brother ghost** (brothers 1–2 only; Kevin has no successor):
   - `ob_listg[brother].xc/yc = hero_x/hero_y; ob_stat = 1` — bones at death location
   - `ob_listg[brother + 2].ob_stat = 3` — ghost setfig activated

2. **Load new brother stats**:
   - `ob_list8[9].ob_stat = 3` — re-enable princess as captive
   - Load stats from `blist[brother]`: `brave, luck, kind, wealth`
   - `stuff` pointer switches to new brother's inventory array
   - `brother++`

3. **Clear inventory**: Zero first 31 slots (`GOLDBASE`). Give one Dirk: `stuff[0] = an->weapon = 1`.

4. **Reset timers**: `secret_timer = light_timer = freeze_timer = 0`. Spawn at `(19036, 15755)` in region 3 (Tambry area).

5. **Display placard** (brother-specific):
   - Brother 1 (Julian): `placard_text(0)` — "Rescue the Talisman!"
   - Brother 2 (Phillip): `placard_text(1)` + `placard_text(2)` — Julian failed / Phillip sets out
   - Brother 3 (Kevin): `placard_text(3)` + `placard_text(4)` — Phillip failed / Kevin takes quest

6. **Load sprites**: `shape_read()` → `read_shapes(brother − 1)` for correct character sprite.

7. **Journey message**: event(9) "started the journey in Tambry", with brother-specific suffix: event(10) "as had his brother before him" for Phillip, event(11) "as had his brothers before him" for Kevin.

#### Fairy Rescue Path (`new == FALSE`)

Skips ghost placement, stat/inventory reset, and placard text. Hero respawns at current `safe_x, safe_y` with current stats. Only vitality, hunger, and fatigue are reset.

#### Common Finalization (both paths)

- Position: `hero_x = safe_x, hero_y = safe_y`
- Vitality: `15 + brave / 4` (full HP)
- Time: `daynight = 8000, lightlevel = 300` (morning)
- `hunger = fatigue = 0`
- `an->state = STILL; an->race = -1`

### 20.5 Dead Brother Ghost and Bones

**Ghost placement** (during succession): Bones object placed at hero's death coordinates. Ghost setfig activated to allow interaction. Ghost dialogue: `speak(49)` — "I am the ghost of your dead brother. Find my bones…"

**Bones pickup** (ob_id 28): When a living brother picks up bones:

1. Both ghost setfigs cleared: `ob_listg[3].ob_stat = ob_listg[4].ob_stat = 0`
2. Dead brother's inventory merged: `for (k = 0; k < GOLDBASE; k++) stuff[k] += dead_brother_stuff[k]`

Dead brother stuff pointer: index 1 = Julian's stuff, index 2 = Phillip's stuff.

### 20.6 Inventory Serialization (`mod1save`)

Serializes all three brothers' inventory arrays (35 bytes each) and the missile list. After loading, `stuff = blist[brother − 1].stuff` reassigns the active inventory pointer.

### 20.7 Game Over

When `brother > 3` (all three brothers dead):

- `placard_text(5)`: "And so ends our sad tale. The Lesson of the Story: Stay at Home!"
- `quitflag = TRUE`
- `Delay(500)` — 10-second pause (500 ticks at 50 Hz Amiga `Delay()` timing)

### 20.8 What Persists Across Brothers

| Persists | Resets |
|----------|--------|
| Princess counter (`princess`) | Stats (loaded fresh from `blist[]`) |
| Quest flags (`ob_listg`, `ob_list8` stats) | Inventory (zeroed; only a Dirk given) |
| Object world state (all `ob_list` data) | Position (back to Tambry 19036, 15755) |
| `dstobs[]` distribution flags | Hunger / fatigue (→ 0) |
| | Timers (secret, light, freeze → 0) |
| | `daynight` (→ 8000), `lightlevel` (→ 300) |

Princess counter persists across succession. However, `ob_list8[9].ob_stat` IS reset to 3 during `revive()`, enabling each new brother to trigger a rescue. After `princess >= 3`, no further rescues fire because `ob_list8[9].ob_stat` stays 0 after the third `rescue()` call.
```

**Key corrections from existing docs:**
1. **R-SURV-003**: Fixed `AND` → `OR` for hunger/fatigue HP damage condition (research clearly shows `||`)


