## 10. Combat System

### 10.1 Melee Hit Detection

Per-frame check for every actor in fighting state (states 0–11):

**Strike Point**:
```
xs = newx(attacker.abs_x, facing, weapon*2) + rand8() - 3
ys = newy(attacker.abs_y, facing, weapon*2) + rand8() - 3
```

Strike point extends `weapon_code * 2` pixels in facing direction, with ±3 to ±4 pixel random jitter.

**Reach (`bv`)**:

| Attacker | Reach | Notes |
|----------|-------|-------|
| Player | `(brave / 20) + 5`, max 15 | Julian starts at 6, maxes at 15 at brave=200 |
| Monster | `2 + rand4()` = 2–5 | Re-rolled each frame |

**Target Matching**: Chebyshev distance (max of `|dx|`, `|dy|`) from strike point to target. All conditions must be true:
1. Distance < `bv`
2. `freeze_timer == 0`
3. Player attacks: automatic hit
4. Monster attacks: `rand256() > brave` must pass (bravery = dodge probability)

Monster hit probability: `(256 − brave) / 256`. At brave=35 → 86% hit rate. At brave=100 → 61%.

**Near-miss**: when distance < `bv + 2` and weapon ≠ wand → `effect(1, 150 + rand256())`.

### 10.2 Damage — `dohit(i, j, fc, wt)`

Parameters: `i` = attacker (−1=arrow, −2=fireball, 0=player, 3+=monster), `j` = defender, `fc` = facing, `wt` = damage.

**Immunity checks**:

| Target | Condition | Effect |
|--------|-----------|--------|
| Necromancer (race 9) | weapon < 4 | Immune; `speak(58)` |
| Witch (race 0x89) | weapon < 4 AND no Sun Stone (`stuff[7]==0`) | Immune; `speak(58)` |
| Spectre (race 0x8a) | Always | Immune, silent return |
| Ghost (race 0x8b) | Always | Immune, silent return |

**Damage**: `vitality -= wt` (weapon code IS the damage). Vitality floors at 0.

**Melee damage formula**: `wt + bitrand(2)` where `wt` = weapon code. Touch attack (code 8) clamps `wt` to 5 before calculation.

**Missile damage**: `rand8() + 4` = 4–11 for both arrows and fireballs.

**Knockback**: defender pushed 2 pixels in attacker's facing direction via `move_figure(j, fc, 2)`. If knockback succeeds and attacker is melee (`i >= 0`), attacker slides 2 pixels forward. DRAGON and SETFIG types immune to knockback.

Every `dohit()` call ends with `checkdead(j, 5)`.

### 10.3 Weapon Types & Damage

| Code | Name | Type | Damage Range | Strike Range |
|------|------|------|-------------|-------------|
| 0 | None | Melee | 0–2 | 0–4 px |
| 1 | Dirk | Melee | 1–3 | 2–6 px |
| 2 | Mace | Melee | 2–4 | 4–8 px |
| 3 | Sword | Melee | 3–5 | 6–10 px |
| 4 | Bow | Ranged | 4–11 | mt=6 px |
| 5 | Wand | Ranged | 4–11 | mt=9 px |
| 8 | Touch | Melee | 5–7 | 10–14 px |

Touch attack (code 8) is monster-only, used by Wraiths, Snakes, Spiders, and Loraii (arms group 6).

### 10.4 Missile System

6 missile slots. Assigned round-robin via `mdex`.

| Property | Arrow | Fireball |
|----------|-------|----------|
| Hit radius (`mt`) | 6 pixels | 9 pixels |
| Damage | `rand8() + 4` = 4–11 | `rand8() + 4` = 4–11 |
| `dohit` attacker code | −1 | −2 |

**Dodge check**: player target → `bv = brave`; monster target → `bv = 20`. Only slot 0 applies dodge (`bitrand(512) > bv`); slots 1–5 always hit if in range. ~17% of projectiles are dodge-eligible.

**Special ranged attacks**:

| Attacker | Damage | Rate |
|----------|--------|------|
| Dragon | 4–11 (fireball) | 25% per frame (`rand4() == 0`) |
| Witch | `rand2() + 1` = 1–2 | When `witchflag` set and distance < 100 |

### 10.5 Enemy Encounter Chart

| Index | Monster | HP | Aggressive | Arms | Cleverness | Treasure | File ID |
|-------|---------|-----|------------|------|------------|----------|---------|
| 0 | Ogre | 18 | TRUE | 2 | 0 | 2 | 6 |
| 1 | Orcs | 12 | TRUE | 4 | 1 | 1 | 6 |
| 2 | Wraith | 16 | TRUE | 6 | 1 | 4 | 7 |
| 3 | Skeleton | 8 | TRUE | 3 | 0 | 3 | 7 |
| 4 | Snake | 16 | TRUE | 6 | 1 | 0 | 8 |
| 5 | Salamander | 9 | TRUE | 3 | 0 | 0 | 7 |
| 6 | Spider | 10 | TRUE | 6 | 1 | 0 | 8 |
| 7 | DKnight | 40 | TRUE | 7 | 1 | 0 | 8 |
| 8 | Loraii | 12 | TRUE | 6 | 1 | 0 | 9 |
| 9 | Necromancer | 50 | TRUE | 5 | 0 | 0 | 9 |
| 10 | Woodcutter | 4 | 0 | 0 | 0 | 0 | 9 |

Field semantics:
- **hitpoints**: base vitality at spawn
- **aggressive**: TRUE = hostile (field is never read at runtime; peace zones use extent system)
- **arms**: indexes `weapon_probs[arms*4 + rnd(4)]` for weapon selection
- **cleverness**: 0 = ATTACK1 (stupid), 1 = ATTACK2 (clever)
- **treasure**: indexes `treasure_probs[treasure*8 + rnd(8)]` for loot
- **file_id**: image file index for sprite loading

### 10.6 Weapon Probability Table — `weapon_probs[32]`

8 groups of 4 entries, indexed by `arms * 4 + rnd(4)`:

| Group | Values | Weapons |
|-------|--------|---------|
| 0 | 0,0,0,0 | None |
| 1 | 1,1,1,1 | All dirks |
| 2 | 1,2,1,2 | Dirks and maces |
| 3 | 1,2,3,2 | Mostly maces, some swords |
| 4 | 4,4,3,2 | Bows and swords |
| 5 | 5,5,5,5 | All magic wands |
| 6 | 8,8,8,8 | Touch attack |
| 7 | 3,3,3,3 | All swords |

### 10.7 Treasure Probability Table — `treasure_probs[40]`

5 groups of 8 entries, indexed by `treasure * 8 + rnd(8)`:

**Group 0** (treasure=0): `{0,0,0,0,0,0,0,0}` — nothing. Used by Snake, Salamander, Spider, DKnight, Loraii, Necromancer, Woodcutter.

**Group 1** (treasure=1, Orcs): `{9,11,13,31,31,17,17,32}`:

| Roll | Index | Item |
|------|-------|------|
| 0 | 9 | Blue Stone |
| 1 | 11 | Glass Vial |
| 2 | 13 | Bird Totem |
| 3–4 | 31 | 2 Gold Pieces |
| 5–6 | 17 | Green Key |
| 7 | 32 | 5 Gold Pieces |

**Group 2** (treasure=2, Ogres): `{12,14,20,20,20,31,33,31}`:

| Roll | Index | Item |
|------|-------|------|
| 0 | 12 | Crystal Orb |
| 1 | 14 | Gold Ring |
| 2–4 | 20 | Grey Key |
| 5, 7 | 31 | 2 Gold Pieces |
| 6 | 33 | 10 Gold Pieces |

**Group 3** (treasure=3, Skeletons): `{10,10,16,16,11,17,18,19}`:

| Roll | Index | Item |
|------|-------|------|
| 0–1 | 10 | Green Jewel |
| 2–3 | 16 | Gold Key |
| 4 | 11 | Glass Vial |
| 5 | 17 | Green Key |
| 6 | 18 | Blue Key |
| 7 | 19 | Red Key |

**Group 4** (treasure=4, Wraiths): `{15,21,0,0,0,0,0,0}`:

| Roll | Index | Item |
|------|-------|------|
| 0 | 15 | Jade Skull |
| 1 | 21 | White Key |
| 2–7 | 0 | Nothing |

### 10.8 Death System — `checkdead(i, dtype)`

Triggers when `vitality < 1` and state ≠ DYING and state ≠ DEAD:

| Effect | Condition |
|--------|-----------|
| Set `goal=DEATH`, `state=DYING`, `tactic=7` | Always |
| DKnight death speech: `speak(42)` | race == 7 |
| `kind −= 3` | SETFIG type, not witch (race ≠ 0x89) |
| `brave++` | Enemy (i > 0) |
| `event(dtype)`, `luck −= 5`, `setmood(TRUE)` | Player (i == 0) |

Death event messages: dtype 5 = killed, 6 = drowned, 7 = burned, 8 = turned to stone.

Death animation: `tactic` counts down 7→0 (7 frames), sprites 80/81 alternating. At 0 → `state = DEAD`, sprite index 82.

**Special death drops**:

| Monster | On Death |
|---------|----------|
| Necromancer (race 0x09) | Transforms to Woodcutter (race 10, vitality 10); drops Talisman (object 139) |
| Witch (race 0x89) | Drops Golden Lasso (object 27) |

### 10.9 Goodfairy & Brother Succession

When player is DEAD or FALL, `goodfairy` countdown (u8, starts at 0):

| Range | Duration | Effect |
|-------|----------|--------|
| 255→200 | ~56 frames | Death sequence — corpse visible, death song |
| 199→120 | ~80 frames | **Luck gate**: luck < 1 → `revive(TRUE)` (brother succession); FALL → `revive(FALSE)` (non-lethal) |
| 119→20 | ~100 frames | Fairy sprite flies toward hero (only if luck ≥ 1) |
| 19→2 | ~18 frames | Resurrection glow effect |
| 1 | 1 frame | `revive(FALSE)` — fairy rescue, same character |

Luck cannot change during DEAD state. Fairy rescues from starting stats: Julian 3, Phillip 6, Kevin 3 (each death costs 5 luck; falls cost 2).

**`revive(TRUE)` — New brother**: brother increments (1→Julian, 2→Phillip, 3→Kevin, 4+→game over). Stats from `blist[]`. Inventory wiped for indices 0 to GOLDBASE−1. Starting weapon = Dirk. Vitality = `15 + brave/4`. Dead brother's body and ghost placed in world.

**`revive(FALSE)` — Fairy rescue**: no stat changes. Returns to `safe_x`/`safe_y`. Vitality = `15 + brave/4`.

### 10.10 Bravery & Luck

Bravery is both passive experience and active combat stat:

| Effect | Formula |
|--------|---------|
| Melee reach | `(brave / 20) + 5`, max 15 |
| Monster dodge | `rand256() > brave` must pass |
| Missile dodge (slot 0) | `bitrand(512) > brave` |
| Starting vitality | `15 + brave / 4` |
| Growth | +1 per enemy kill |

Compounding feedback loop: more kills → higher brave → longer reach + better dodge + more HP → more kills.

Luck: −5 per death, −2 per ledge fall. When depleted, next death is permanent.

---


