## 11. AI & Behavior

### 11.1 Goal Modes

11 high-level goal modes stored in `shape.goal`:

| Value | Name | Purpose |
|-------|------|---------|
| 0 | USER | Player-controlled |
| 1 | ATTACK1 | Attack stupidly (cleverness 0) |
| 2 | ATTACK2 | Attack cleverly (cleverness 1) |
| 3 | ARCHER1 | Archery attack style 1 |
| 4 | ARCHER2 | Archery attack style 2 |
| 5 | FLEE | Run directly away from hero |
| 6 | STAND | Stand still, face hero |
| 7 | DEATH | Dead character |
| 8 | WAIT | Wait to speak to hero |
| 9 | FOLLOWER | Follow another character |
| 10 | CONFUSED | Run around randomly |

ATTACK1 vs ATTACK2 determined by `cleverness` in `encounter_chart[]`.

Unless noted otherwise, every "tick" in this section refers to the 15 Hz animation/AI gameplay tick. Rendering still presents at 30 fps, so one AI tick spans two displayed frames.

### 11.2 Tactical Modes

13 tactical modes stored in `shape.tactic`:

| Value | Name | Purpose |
|-------|------|---------|
| 0 | FRUST | Frustrated — try a different tactic |
| 1 | PURSUE | Move toward hero |
| 2 | FOLLOW | Move toward another character |
| 3 | BUMBLE_SEEK | Bumble around seeking target |
| 4 | RANDOM | Move in random direction |
| 5 | BACKUP | Reverse current direction |
| 6 | EVADE | Move 90° from hero |
| 7 | HIDE | Seek hiding place (planned, never implemented) |
| 8 | SHOOT | Shoot an arrow |
| 9 | SHOOTFRUST | Arrows not connecting — re-evaluate |
| 10 | EGG_SEEK | Snakes seeking turtle eggs |
| 11 | DOOR_SEEK | Vestigial — replaced by hardcoded DKnight logic |
| 12 | DOOR_LET | Vestigial — replaced by hardcoded DKnight logic |

Source comment: "choices 2–5 can be selected randomly for getting around obstacles."

### 11.3 `set_course` Algorithm

`set_course(object, target_x, target_y, mode)` — 7 pathfinding modes:

**Direction computation**:
1. Mode 6: uses target_x/target_y directly as xdif/ydif (raw vector)
2. All other modes: `xdif = self.abs_x − target_x`, `ydif = self.abs_y − target_y`
3. Compute `xdir = sign(xdif)`, `ydir = sign(ydif)`

**Directional snapping** (mode ≠ 4): if one axis dominates, minor axis zeroed:
- `(|xdif| >> 1) > |ydif|` → `ydir = 0` (mostly horizontal)
- `(|ydif| >> 1) > |xdif|` → `xdir = 0` (mostly vertical)

Mode 4 skips snapping, always allowing diagonal.

**com2 lookup**: `j = com2[4 − 3*ydir − xdir]`. If j == 9 (at target): `state = STILL`, return.

**Random deviation**: if deviation > 0: `rand() & 2` (bit 1, not bit 0) determines `j += deviation` or `j −= deviation`, then `j &= 7`.

| Mode | Behavior | Deviation |
|------|----------|-----------|
| 0 | Toward target with snapping | 0 |
| 1 | Toward target + deviation when distance < 40 | 1 |
| 2 | Toward target + deviation when distance < 30 | 1 |
| 3 | Away from target (reverses direction) | 0 |
| 4 | Toward target without snapping (always diagonal) | 0 |
| 5 | Toward target with snapping; does NOT set state to WALKING | 0 |
| 6 | Uses target_x/target_y as raw direction vector | 0 |

**Important**: these mode numbers are NOT the tactical mode constants. `do_tactic()` maps tactics to `set_course` modes:

| Tactic | set_course mode | Target |
|--------|----------------|--------|
| PURSUE (1) | 0 | Hero |
| FOLLOW (2) | 0 | Leader (+20 Y offset) |
| BUMBLE_SEEK (3) | 4 | Hero (no snap) |
| BACKUP (5) | 3 | Hero (reversed) |
| EVADE (6) | 2 | Neighboring actor (+20 Y) |
| SHOOT (8) | 0 or 5 | Hero (face only when aligned) |
| EGG_SEEK (10) | 0 | Fixed coords (23087, 5667) |
| RANDOM (4) | *(direct)* | `facing = rand() & 7` |

### 11.4 `do_tactic()` Dispatch

All tactical movement rate-limited by random gate:
```
base: !(rand() & 7) = 12.5% chance
ATTACK2 goal: !(rand() & 3) = 25% chance
```

When gate fails, actor continues previous trajectory unchanged.

| Tactic | Rate-limited? | Notes |
|--------|---------------|-------|
| PURSUE | Yes | `set_course` mode 0 to hero |
| SHOOT | **No** — fires every tick | Checks axis alignment, transitions between mode 0 and 5 |
| RANDOM | Facing only | `state = WALKING` unconditional; facing changes when gate passes |
| BUMBLE_SEEK | Yes | `set_course` mode 4 (no snap) |
| BACKUP | Yes | `set_course` mode 3 (reverse) |
| FOLLOW | Yes | `set_course` mode 0 to leader + 20 Y |
| EVADE | Yes | `set_course` mode 2; target = `anim_list[i*2]` |
| EGG_SEEK | Yes | `set_course` mode 0 to fixed (23087, 5667) |

### 11.5 AI Main Loop

Processes actors 2 through `anix−1` (skipping player and raft). Processing order:

1. **Goodfairy suspend**: if `goodfairy > 0 && < 120`, all AI halts
2. **CARRIER type**: every 16 ticks, face player via `set_course(i, hero_x, hero_y, 5)`. No other AI
3. **SETFIG type**: skipped entirely
4. **Distance & battle detection**: within 300×300 pixels → `actors_on_screen = TRUE`, `battleflag = TRUE`
5. **Random reconsider**: base `!bitrand(15)` = 1/16 (6.25%)
6. **Goal overrides**: hero dead → FLEE/FOLLOWER; vitality < 2 → FLEE; extent mismatch → FLEE; unarmed → CONFUSED
7. **Frustration handling**: FRUST/SHOOTFRUST → random tactic: ranged = `rand4()+2` → {FOLLOW, BUMBLE_SEEK, RANDOM, BACKUP}; melee = `rand2()+3` → {BUMBLE_SEEK, RANDOM}
8. **Hostile AI** (modes ≤ ARCHER2): reconsider frequency adjustment, tactic selection, melee engagement
9. **FLEE**: `do_tactic(i, BACKUP)`
10. **FOLLOWER**: `do_tactic(i, FOLLOW)`
11. **STAND**: face hero, force STILL state
12. **WAIT**: force STILL, no facing change
13. **CONFUSED** and others: no processing — actor continues last trajectory

`leader` set to first living active enemy at loop end.

### 11.6 Hostile AI Detail

For modes ≤ ARCHER2, reconsider frequency: `if ((mode & 2) == 0) r = !rand4()` → 25% for ATTACK1 and ARCHER2. ATTACK2 and ARCHER1 keep base 6.25%.

Tactic assignment on reconsider:

| Condition | Tactic |
|-----------|--------|
| race==4 && turtle_eggs | EGG_SEEK |
| weapon < 1 | RANDOM (mode→CONFUSED) |
| vitality < 6 && rand2() | EVADE |
| Archer, xd<40 && yd<30 | BACKUP |
| Archer, xd<70 && yd<70 | SHOOT |
| Archer, far | PURSUE |
| Melee, default | PURSUE |

Melee engagement: `thresh = 14 − mode`. DKnight overrides to 16. Within thresh → FIGHTING state. Outside → `do_tactic(i, tactic)`.

### 11.7 Cleverness Effects

| Property | Cleverness 0 | Cleverness 1 |
|----------|-------------|-------------|
| Goal mode | ATTACK1 / ARCHER1 | ATTACK2 / ARCHER2 |
| `do_tactic` rate | 12.5% per tick | 25% per tick (ATTACK2 only) |
| Tactic reconsider | 25% (ATTACK1) or 6.25% (ARCHER1) | 6.25% (ATTACK2) or 25% (ARCHER2) |
| Melee threshold | 13 (ATTACK1) or 11 (ARCHER1) | 12 (ATTACK2) or 10 (ARCHER2) |

Clever enemies: Orcs, Wraith, Snake, Spider, DKnight, Loraii.
Stupid enemies: Ogre, Skeleton, Salamander, Necromancer, Woodcutter.

**P23 — ARCHER2 re-plan paradox (original behavior, preserve):** The bit test that determines reconsider frequency is `(mode & 2) == 0` → 25%. ATTACK1 = 1 and ARCHER2 = 4; both satisfy `& 2 == 0` and get 25%. ATTACK2 = 2 and ARCHER1 = 3; both have `& 2 != 0` and keep base 6.25%. So "clever" archers (ARCHER2) re-plan four times as often as "stupid" archers (ARCHER1), while "clever" melee (ATTACK2) re-plan four times less often than "stupid" melee (ATTACK1). The "clever" label inverts its meaning for bow-wielders. The port MUST reproduce this cadence exactly — it is the original behavior, not a bug to fix. See PROBLEMS.md P23.

### 11.8 DKnight Special Behavior

- Fixed position at (21635, 25762); does not pursue, does not call `do_tactic()`
- Out of range: `state = STILL`, `facing = 5` (south) — physically blocks passage
- In range (xd < 16, yd < 16): enters FIGHTING state
- Never flees (exempt when race matches extent v3 for xtype > 59)
- Respawns every time player re-enters hidden valley zone
- Proximity speech: `speak(41)`; death speech: `speak(42)`

### 11.9 CONFUSED Mode

Assigned when hostile actor loses weapon (`weapon < 1`). First tick: `do_tactic(i, RANDOM)`. Subsequent ticks: CONFUSED (10) fails all goal-mode checks — no AI processing occurs. Actor walks in last random direction until blocked.

### 11.10 NPC Frustration Cycle

When an NPC's movement is blocked in all three probe directions (`dir`, `dir+1`, `dir−2`), the blocked-handler at `fmain.c:1654-1661` sets `an->tactic = FRUST`. On the next AI tick, the frustration handler (`fmain.c:2141-2143`) picks a random escape tactic keyed on weapon type:

```c
if (tactic == FRUST || tactic == SHOOTFRUST) {
    if (an->weapon & 4) do_tactic(i, rand4() + 2);  // ranged
    else                 do_tactic(i, rand2() + 3);  // melee
}
```

| Weapon class | Expression | Range | Possible tactics |
|---|---|---|---|
| Ranged (`weapon & 4` set) | `rand4() + 2` | 2–5 | FOLLOW, BUMBLE_SEEK, RANDOM, BACKUP |
| Melee (`weapon & 4` clear) | `rand2() + 3` | 3–4 | BUMBLE_SEEK, RANDOM |

**Cross-goal-mode behavior** (original, preserve): the frustration handler fires before the goal-mode dispatch, so FRUST can override FLEE, FOLLOWER, and CONFUSED goals — e.g., a fleeing actor blocked at a wall may BUMBLE_SEEK toward the hero instead of continuing to BACKUP. Documented as a quirk of the original; port SHOULD reproduce.

**SHOOTFRUST is dead code** (original): tactic value 9 is defined and checked in the handler above, but is **never assigned** anywhere in the original codebase. The missile collision path never touches `an->tactic`. The SHOOTFRUST branch is therefore unreachable at runtime; the port MAY omit any assignment logic for it, but SHOULD retain the tactic constant and the `|| tactic == SHOOTFRUST` branch in the frustration handler for fidelity and to avoid regression if a future edit adds assignment.

**`set_encounter()` initialization gap** (original, minor cosmetic bug): `set_encounter()` does not initialize `an->tactic` when reusing an `anim_list[]` slot. If the stale value is 0 (FRUST), the frustration handler fires on the actor's first AI tick, giving it a one-off random direction. The port MAY initialize `tactic` to PURSUE (or any non-FRUST sentinel) on spawn to suppress this cosmetic bug.

---



## 12. Encounter Generation

### 12.1 Extent Zones

`extent_list[23]` — 22 zones plus sentinel at index 22. Each entry:

```rust
struct Extent {
    x1: u16, y1: u16, x2: u16, y2: u16,  // bounding box (exclusive)
    etype: u8,                              // zone type
    v1: u8, v2: u8, v3: u8,               // parameters
}
```

`find_place()` performs linear scan of entries 0–21; first match wins. Entry 22 (etype=3, whole world) is sentinel fallback.

| Idx | Location | etype | v1 | v2 | v3 | Category |
|-----|----------|-------|----|----|----|----------|
| 0 | Bird (swan) | 70 | 0 | 1 | 11 | Carrier |
| 1 | Turtle (movable) | 70 | 0 | 1 | 5 | Carrier |
| 2 | Dragon | 70 | 0 | 1 | 10 | Carrier |
| 3 | Spider pit | 53 | 4 | 1 | 6 | Forced encounter |
| 4 | Necromancer | 60 | 1 | 1 | 9 | Special figure |
| 5 | Turtle eggs | 61 | 3 | 2 | 4 | Special figure |
| 6 | Princess rescue | 83 | 1 | 1 | 0 | Peace (special) |
| 7 | Graveyard | 48 | 8 | 8 | 2 | Regular (very high) |
| 8 | Around city | 80 | 4 | 20 | 0 | Peace zone |
| 9 | Astral plane | 52 | 3 | 1 | 8 | Forced encounter |
| 10 | King's domain | 81 | 0 | 1 | 0 | Peace + weapon block |
| 11 | Sorceress domain | 82 | 0 | 1 | 0 | Peace + weapon block |
| 12–14 | Buildings/cabins | 80 | 0 | 1 | 0 | Peace zone |
| 15 | Hidden valley | 60 | 1 | 1 | 7 | Special figure (DKnight) |
| 16 | Swamp region | 7 | 1 | 8 | 0 | Regular (swamp) |
| 17–18 | Spider regions | 8 | 1 | 8 | 0 | Regular (spiders) |
| 19 | Village | 80 | 0 | 1 | 0 | Peace zone |
| 20–21 | Around village/city | 3 | 1 | 3 | 0 | Regular (low) |
| *22* | *Whole world* | *3* | *1* | *8* | *0* | *Sentinel fallback* |

Only extents 0 (bird) and 1 (turtle) are persisted in savegames. Turtle starts at (0,0,0,0) and is repositioned via `move_extent()`.

### 12.2 Extent Type Categories

| etype Range | Category | Behavior |
|-------------|----------|----------|
| 0–49 | Regular encounter zone | Sets `xtype`; random encounters per danger timer |
| 50–59 | Forced group encounter | Monsters spawn immediately; v1=count, v3=type |
| 52 | Astral plane (special) | Forces encounter_type=8 (Loraii); synchronous load |
| 60–61 | Special figure | Unique NPC at extent center if not already present |
| 70 | Carrier | Loads bird/turtle/dragon via `load_carrier(v3)` |
| 80 | Peace zone | Blocks random encounters (`xtype ≥ 50` fails guard) |
| 81 | King peace | Peace + weapon draw blocked: `event(15)` |
| 82 | Sorceress peace | Peace + weapon draw blocked: `event(16)` |
| 83 | Princess rescue | Triggers `rescue()` if `ob_list8[9].ob_stat` set |

### 12.3 Danger Level & Spawn Logic

**Placement check — every 16 frames**: places already-loaded monsters into anim_list[3–6]. Up to 10 random locations tried via `set_loc()` (150–213 pixels from hero). Each must have terrain type 0 (walkable). Dead slots recycled when all 4 full.

**Danger check — every 32 frames**: conditions = no actors on screen, no pending load, no carrier, `xtype < 50`.

Danger level formula:
```
Indoor (region_num > 7): danger_level = 5 + xtype
Outdoor:                 danger_level = 2 + xtype
```

Spawn probability: `rand64() <= danger_level` → `(danger_level + 1) / 64`:

| Zone | xtype | Outdoor Danger | Probability |
|------|-------|----------------|-------------|
| World / village area | 3 | 5 | 6/64 = 9.4% |
| Swamp region | 7 | 9 | 10/64 = 15.6% |
| Spider region | 8 | 10 | 11/64 = 17.2% |
| Graveyard | 48 | 50 | 51/64 = 79.7% |

### 12.4 Monster Type Selection

Base: `rand4()` (0–3 → ogre, orc, wraith, skeleton). Overrides:

| Condition | Override |
|-----------|----------|
| Swamp (xtype=7) | Wraith roll (2) → Snake (4) |
| Spider region (xtype=8) | All rolls → Spider (6) |
| xtype=49 | All rolls → Wraith (2) |

### 12.5 Monster Count

```
encounter_number = v1 + rnd(v2)
```

| Zone | v1 | v2 | Count Range |
|------|----|----|-------------|
| Whole world | 1 | 8 | 1–8 |
| Around village/city | 1 | 3 | 1–3 |
| Spider pit | 4 | 1 | 4 (forced) |
| Graveyard | 8 | 8 | 8–15 |

Only 4 enemy slots (indices 3–6); excess encounter_number resolves over time.

### 12.6 Actor Placement — `set_encounter(i, spread)`

Up to 15 attempts (`MAX_TRY`):

- **DKnight** (v3 == 7): hardcoded position (21635, 25762), placement loop skipped
- **Normal**: random offset from encounter origin: `encounter_x + bitrand(spread) − spread/2`. Accept if `proxcheck == 0`
- **Astral**: also accept if `px_to_im == 7` (ice terrain)

**Race mixing**: `mixflag & 2` → `race = (encounter_type & 0xFFFE) + rand2()`. Disabled for xtype > 49 or xtype % 4 == 0.

**Weapon selection**: `weapon_probs[arms * 4 + wt]`. `wt` re-randomized per enemy if `mixflag & 4`, otherwise shared in batch.

### 12.7 Special Extents

**Carriers** (etype 70): `load_carrier(v3)` places carrier in anim_list[3]:

| v3 | Carrier | Notes |
|----|---------|-------|
| 11 | Swan | Requires Golden Lasso (`stuff[5]`) to mount |
| 5 | Turtle | Extent starts at (0,0,0,0); repositioned via `move_extent()` |
| 10 | Dragon | Has own fireball attack logic |

Carrier extent: 500×400 box centered on point via `move_extent()`.

**Spider pit** (etype 53, idx 3): spawns 4 spiders immediately. `mixflag=0, wt=0`.

**Necromancer / DKnight** (etype 60): unique NPC at extent center. Only spawns if not already present (`anim_list[3].race != v3` or `anix < 4`).

**Princess rescue** (etype 83, idx 6): when `ob_list8[9].ob_stat` set → `rescue()`: displays placard, increments `princess` counter, teleports hero to (5511, 33780), repositions bird extent via `move_extent(0, 22205, 21231)`.

### 12.8 Peace Zones

Extents with etype 80–83 set `xtype ≥ 50`, failing the `xtype < 50` guard. The `aggressive` field in `encounter_chart[]` is **never read** at runtime.

- etype 81 (King): weapon draw → `event(15)` admonishing message
- etype 82 (Sorceress): weapon draw → `event(16)` calming message

---

### C. Timing Constants

| Parameter | Value | Notes |
|-----------|-------|-------|
| Presentation rate | 30 fps | Video refresh / presented frames |
| Animation + AI tick | 15 Hz | One gameplay tick every two presented frames |
| Audio VBL | 60 Hz | Audio interrupt rate |
| Day cycle | 24000 ticks | Separate world-clock counter; see §17 |
| Hunger tick | 128 ticks | Shared world-clock cadence; see survival/day/night specs |
| Health regen | 1024 ticks | Shared world-clock cadence; see survival/day/night specs |
| Safe zone check | 128 ticks | Same as hunger tick |
| Sleep advance | 64 ticks/frame | 63 extra + 1 normal |
| Default tempo | 150 | Timeclock counts per VBL |
| AI reconsider (base) | 1/16 per tick | 6.25% per AI tick via `!bitrand(15)` |
| AI reconsider (ATTACK1/ARCHER2) | 1/4 per tick | 25% per AI tick via `!rand4()` |
| Tactic execution (base) | 1/8 per tick | 12.5% per AI tick via `!(rand()&7)` |
| Tactic execution (ATTACK2) | 1/4 per tick | 25% per AI tick via `!(rand()&3)` |
| Encounter placement | 16 ticks | ≈ 1.07 seconds at 15 Hz |
| Encounter generation | 32 ticks | ≈ 2.13 seconds at 15 Hz |
| Carrier facing update | 16 ticks | Every 16 AI ticks for CARRIER type |
| Death animation | 7 frames | tactic counts down 7→0 |
| Goodfairy total | 255 frames | ≈ 8.5 seconds |
| Goodfairy luck gate | < 200 | ~56 frames after death |
| Goodfairy fairy visible | < 120 | ~80 frames after luck gate |
| Timer heartbeat | 16 events | ticker 0→16 synthesizes key |
| FALL friction | 25%/tick | `(vel * 3) / 4` per AI tick |
| Ice velocity cap | 42 (normal), 40 (swan) | Max magnitude per axis |
| Camera dead zone X | ±20 pixels | No scroll within zone |
| Camera dead zone Y | ±10 pixels | No scroll within zone |
| Camera snap X | > 70 pixels | Instant camera reposition |
| Camera snap Y (down) | > 44 pixels | Asymmetric for sprite offset |
| Camera snap Y (up) | > 24 pixels | Asymmetric for sprite offset |
| Player frust threshold 1 | > 20 | Head-shake (dex=84/85, §9.8) |
| Player frust threshold 2 | > 40 | Fixed south-facing pose (dex=40, §9.8) |
| Hunger stumble threshold | > 120 | 1/4 chance of direction deflection |

### B. Brother Stats

| Property | Julian | Phillip | Kevin |
|----------|--------|---------|-------|
| brave | 35 | 20 | 15 |
| luck | 20 | 35 | 20 |
| kind | 15 | 15 | 35 |
| wealth | 20 | 15 | 10 |
| Starting vitality | 23 | 20 | 18 |
| Vitality formula | `15 + brave/4` | `15 + brave/4` | `15 + brave/4` |
| Starting melee reach | 6 | 6 | 5 |
| Monster hit rate at start | 86% | 92% | 94% |
| Fairy rescues (from start) | 3 | 6 | 3 |
| Starting weapon | Dirk (1) | Dirk (1) | Dirk (1) |

