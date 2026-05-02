## 18. Survival Mechanics

### 18.1 Player Stats Overview

Six core stats declared as global `short` variables: `brave`, `luck`, `kind`, `wealth`, `hunger`, `fatigue`.

Vitality is per-actor (`struct shape.vitality`), NOT a global. The hero's vitality is `anim_list[0].vitality`.

All stats are part of a contiguous saved block serialized via `saveload()`.

### 18.2 Hunger

**Increment**: +1 every 128 game ticks (`(daynight & 127) == 0`), when hero is alive and not sleeping.

**Thresholds:**

| Hunger | Effect |
|--------|--------|
| == 35 | event(0) — "getting rather hungry" (one-time) |
| == 60 | event(1) — "getting very hungry" (one-time) |
| > 90, `(hunger & 7) == 0` | event(2) — "starving!" (periodic, every 8th hunger increment) |
| > 100, `(hunger & 7) == 0` | `vitality -= 2` (only when `vitality > 5`) |
| > 120 | Movement wobble: direction ±1 with 75% probability (`rand4() != 0`; `rand2()` selects ±1) |
| > 140 | event(24) "passed out!", `hunger = 130`, `state = SLEEP` |

The `(hunger & 7) == 0` condition gates both starvation warnings and HP damage, firing every 8th hunger increment (≈ every 1024 daynight ticks).

HP damage fires when **either** `hunger > 100` **OR** `fatigue > 160` (logical OR):

```
if (anim_list[0].vitality > 5)
    if (hunger > 100 || fatigue > 160)
        anim_list[0].vitality -= 2; prq(4);
```

**Auto-Eating**: When `(daynight & 127) == 0` in a safe zone, if `hunger > 30` and `stuff[24] > 0` (has Fruit): `stuff[24]--; hunger -= 30; event(37)`. Uses direct subtraction, not via `eat()`.

### 18.3 Fatigue

**Increment**: +1 alongside hunger, same 128-tick timer and conditions.

**Thresholds:**

| Fatigue | Effect |
|---------|--------|
| == 70 | event(3) — "getting tired" (one-time) |
| == 90 | event(4) — "getting sleepy" (one-time) |
| > 160, `(hunger & 7) == 0` | `vitality -= 2` (shared condition with hunger > 100, see §18.2) |
| > 170 | event(12) — forced sleep (only when `vitality ≤ 5`) |

The forced sleep at `fatigue > 170` is in the `else` branch of the `vitality > 5` check — it only fires when HP is critically low.

### 18.4 Sleep Mechanics

**Voluntary sleep**: Inside buildings (region 8), standing on bed terrain tiles (IDs 161, 52, 162, 53) increments `sleepwait`. After 30 ticks:

- `fatigue < 50` → event(25) "not sleepy" — no sleep
- `fatigue >= 50` → event(26) "decided to lie down and sleep", `state = SLEEP`

**Sleep processing** (each frame while sleeping):

- `daynight += 63` — time advances rapidly (64× normal with the +1 increment)
- `fatigue--` (if > 0)
- Wake conditions (any): `fatigue == 0`, OR (`fatigue < 30` AND `daynight ∈ [9000, 10000)`), OR (`battleflag` AND `rand64() == 0`)
- On waking: `state = STILL`, Y-position snapped to grid alignment

### 18.5 `eat()` Function

```
eat(amt):
    hunger -= amt
    if hunger < 0: hunger = 0; event(13)   // "full"
    else: print("Yum!")
```

| Food Source | Amount | Notes |
|-------------|--------|-------|
| Pickup fruit (hungry, hunger ≥ 15) | `eat(30)` | Via `eat()` function |
| Buy food from shop | `eat(50)` | Via `eat()` function |
| Auto-eat fruit in safe zone | `hunger -= 30` | Direct subtraction, not via `eat()` |

When `hunger < 15`, picked-up fruit is stored instead of eaten: `stuff[24]++; event(36)`.

### 18.6 Vitality / HP

**Max HP formula**: `15 + brave / 4`

Used at: natural healing cap, revive, heal vial cap, priest healing.

**Natural healing**: Every 1024 ticks (`(daynight & 0x3FF) == 0`), hero gains +1 vitality up to max. During sleep, `daynight` advances by 63 per frame, so healing occurs ≈63× faster.

**Heal vial**: `vitality += rand8() + 4` (4–11 HP), capped at max.

**Priest healing**: Full heal to `15 + brave / 4`. Requires `kind >= 10`; below 10, priest gives dismissive dialogue.

**Damage sources:**

| Source | Amount | Condition |
|--------|--------|-----------|
| Combat hits | `vitality -= wt` (weapon table value) | Per hit |
| Hunger/fatigue | −2 | When `(hunger & 7) == 0` and (hunger > 100 OR fatigue > 160) and vitality > 5 |
| Drowning (`environ == 30`) | −1 | Every 8 ticks. Wraiths (race 2) and skeletons (race 3) are immune. |
| Lava (`environ > 2`) | −1 per tick | `environ > 15` = instant death |

Rose (`stuff[23]`) prevents lava damage by forcing `environ = 0` each tick (hero only).

Wraiths (race 2) and snakes (race 4) have terrain forced to 0 at `fmain.c:1639`, so NPCs of those races never enter drowning or lava environ in the first place.

### 18.7 Stat Changes

#### Bravery

| Change | Condition |
|--------|-----------|
| +1 | Kill any non-hero actor |
| −1 | Per target killed by Jade Skull |

Initial values: Julian = 35, Phillip = 20, Kevin = 15.

Combat effects: hero melee hit range = `brave / 20 + 5`, hero missile bravery = full `brave` value, enemy hit dodge = `rand256() > brave`.

#### Luck

| Change | Condition |
|--------|-----------|
| −5 | Hero death |
| −2 | Fall into pit |
| +5 (probabilistic) | Sorceress talk: `if (luck < rand64()) luck += 5` |

Clamped ≥ 0 on HUD redraw. Initial values: Julian = 20, Phillip = 35, Kevin = 20. Luck < 1 after death triggers brother succession instead of fairy rescue.

#### Kindness

| Change | Condition |
|--------|-----------|
| −3 | Kill non-witch SETFIG |
| +1 (probabilistic) | Give gold: `if (rand64() > kind) kind++` |

Clamped ≥ 0 in `checkdead`. Initial values: Julian = 15, Phillip = 15, Kevin = 35. Below 10: wizards and priests give dismissive dialogue.

#### Wealth

| Change | Condition |
|--------|-----------|
| +50 | Loot gold bag (MONEY pickup) |
| +100 | Container gold |
| +100 | Princess rescue reward |
| +variable | Corpse loot (`inv_list[j].maxshown` for gold items) |
| −price | Buy item from shop |
| −2 | Give gold to NPC |

Initial values: Julian = 20, Phillip = 15, Kevin = 10.

### 18.8 Safe Zones

Updated every 128 ticks when ALL conditions met:

- No enemies visible or loading
- No witch encounter active
- Hero on solid ground (`environ == 0`)
- No danger flag
- Hero alive

Safe zone coordinates (`safe_x, safe_y`) stored for fairy rescue respawn point.

### 18.9 Fiery Death Zone

Rectangle: `8802 < map_x < 13562`, `24744 < map_y < 29544`.

- Hero with rose (`stuff[23]`): immune (`environ` reset to 0 each tick)
- `environ > 15`: instant death
- `environ > 2`: −1 vitality per tick

### 18.10 HUD Display

Stats rendered via print queue:

- **prq(7)**: `Brv`, `Lck`, `Knd`, `Wlth` — four stat values
- **prq(4)**: `Vit` — vitality value

Hunger and fatigue are **not** displayed on the HUD — communicated only through event messages.

### 18.11 Random Number Generation

All random values in the game are produced by a Linear Congruential Generator (LCG):

```
seed1 = low16(seed1) × 45821 + 1       // 16×16→32 unsigned multiply, then +1
output = ror32(seed1, 6) & 0x7FFFFFFF   // rotate right 6 bits, clear sign bit
```

Initial seed: `19837325` (hex `0x012ED98D`). The 68000 `mulu.w` operates on the low 16 bits only, so the effective state space is 2^16 with a maximum period of 65536.

No runtime reseeding — the seed is not derived from system time, VBlank counter, or user input. Sequence variation between sessions comes only from the copy-protection input loop, where each keystroke calls `rand()` with the result discarded.

**Function family:**

| Function | Returns | Formula |
|----------|---------|---------|
| `rand()` | 0 to 0x7FFFFFFF (31-bit) | Base LCG output |
| `bitrand(x)` | `rand() & x` | Masked random |
| `rand2()` | 0 or 1 | `rand() & 1` |
| `rand4()` | 0–3 | `rand() & 3` |
| `rand8()` | 0–7 | `rand() & 7` |
| `rand64()` | 0–63 | `rand() & 63` |
| `rand256()` | 0–255 | `rand() & 255` |
| `rnd(n)` | 0 to n−1 | `(rand() & 0xFFFF) % n` |

The `bitrand`/`randN` variants use bitwise AND, producing uniform results only when the mask is a power-of-two minus one. `rnd(n)` uses true modulo via 16-bit division.

---


