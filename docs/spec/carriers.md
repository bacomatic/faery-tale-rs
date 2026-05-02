## 21. Carriers & World Navigation

### 21.1 Carrier Types

| Carrier | `riding` value | Type constant | Sprite file | File ID | Rideable? |
|---------|---------------|---------------|-------------|---------|-----------|
| Raft | 1 | `RAFT` | cfiles[4] | 1348 | Yes (auto) |
| Turtle | 5 | `CARRIER` | cfiles[5] | 1351 | Yes (proximity) |
| Swan/Bird | 11 | `CARRIER` | cfiles[11] | 1120 | Yes (requires Golden Lasso) |
| Dragon | — | `DRAGON` | cfiles[10] | 1160 | **No** (hostile NPC) |

### 21.2 Raft

- Actor slot 1, type RAFT
- Activation: within 9px proximity, no active carrier, `wcarry==1`, terrain codes 3–5 (water/shore)
- Movement: snaps to hero position each frame (no autonomous movement)
- Prevents drowning while active (`riding == 1`)
- Dismount: automatic when proximity or terrain conditions fail

### 21.3 Turtle

- Actor slot 3, type CARRIER, `actor_file = 5`
- Summoned via USE menu (turtle item `stuff[6]`)
- Boarding: within 16px proximity, `wcarry==3`, sets `riding = 5`
- Cannot be summoned in central region bounds (11194–21373 X, 10205–16208 Y)

**Autonomous movement (unridden)** — `fmain.c:1520-1542`:

- Runs **every tick**. Speed is always **3** pixels (`fmain.c:1521-1522`).
- Uses `px_to_im()` directly (not `proxcheck()`): only commits a position update when the probed terrain is **exactly type 5** (very deep water). Types 2–4 (shallower water) and all land types are impassable to the autonomous turtle.
- Each tick, probes 4 directions in priority order from current facing `d`:
  1. `d`
  2. `(d + 1) & 7`
  3. `(d − 1) & 7`
  4. `(d − 2) & 7`
  The first direction whose probe lands on terrain 5 is committed; if none succeed the turtle does not move.
- The handler **does not persist** the chosen direction back to `an->facing` — it exits via `goto raise` (`fmain.c:1545`), bypassing the `an->facing = d` write at `newloc:` (`fmain.c:1633`). Facing is instead updated by the general CARRIER AI path (§11.4 `do_tactic`), which every 16 ticks calls `set_course(i, hero_x, hero_y, 5)` to aim the turtle's facing toward the hero (mode 5 — toward-target, no WALKING state change). Net effect: the turtle re-aims at the hero every 16 ticks and probes for water each tick in that general direction, producing a slow hero-seeking drift along coastlines and water bodies.
- **Bug — extent drift** (do not preserve): on a frame where no direction lands on terrain 5, `xtest`/`ytest` retain the last failed probe's coordinates; `move_extent(1, xtest, ytest)` at `fmain.c:1545` still executes, drifting the 500×400 extent box away from the turtle's actual `abs_x`/`abs_y`. Over time the extent can desync enough to make the turtle unreachable. See RESEARCH.md §9.6, [PROBLEMS.md §P22]. The port SHOULD fix this (skip `move_extent` on failed probes).

**Mounted movement (`riding == 5`)** — `fmain.c:1599`:

- Hero's WALKING step is forced to speed 3 (`fmain.c:1599`); checked before terrain effects, so riding also neutralizes lava push-back (terrain 8, `e = −2`).
- Movement uses the hero's standard `proxcheck()` — **the rider can walk onto any non-blocked terrain**, including land.
- The turtle's actual position (`abs_x`/`abs_y`) only updates when the hero stands on terrain 5 (`fmain.c:1541`); when the hero walks onto land, the turtle sprite stays at the water's edge.
- The `raftprox` flag forces `environ = 0` while riding (`fmain.c:1768`), preventing drowning. When the hero moves >16px away, `raftprox` drops to 0 and the rider dismounts automatically.

**Mounted-turtle exploit** (original behavior — preserve): melee recoil from `dohit()` pushes the rider via `move_figure(i, fc, 2)` (`fmain2.c:242-245, 322-329`), which only applies `proxcheck()` and bypasses the autonomous turtle's `px_to_im(...)==5` water-only rule, enabling transit over terrain the turtle itself cannot cross.

### 21.4 Swan (Bird)

**Terminology.** The flying carrier is called **"Swan"** in all player-facing narrative (event messages, quest texts, STORYLINE.md). Internally the original source code calls it **"bird"** — the actor file is `cfiles[11]`, the extent handler is `load_carrier(BIRD)`, and `actor_file == 11` is checked as the bird identifier. This specification uses "Swan" in narrative/UX context and "bird" when referring to code-level identifiers (`actor_file == 11`, `riding == 11`, `load_carrier(BIRD)`, `cfiles[11]`). The two names refer to the same entity.

- Actor slot 3, type CARRIER, `actor_file = 11`
- Extent zone 0 at (2118, 27237)
- Requires Golden Lasso (`stuff[5]`) to board
- Riding state: `riding = 11`, hero `environ = -2` (airborne)
- Movement: inertial flight physics
  - Velocity accumulates via directional acceleration
  - Max horizontal velocity ~32, max vertical ~40
  - Position updates by `vel/4` per frame
  - No terrain collision — `proxcheck` skipped
  - Auto-faces into wind via `set_course(0,-nvx,-nvy,6)`
- Dismount conditions: hero action button + velocity < 15 + clear ground below + not fiery terrain
  - Blocked in lava zone: event 32 ("Ground is too hot")
  - Blocked at high velocity: event 33 ("Flying too fast")

#### Sprite Selection (RESEARCH §2.2, `fmain.c:1497-1510, 2463-2464`)

The swan sprite is **not** driven by the motion-state enum. Two distinct rendering paths apply based on whether the swan is being ridden:

| Situation | Condition | Sheet | Frame |
|-----------|-----------|-------|-------|
| Mounted / flying | `riding == 11` | `cfiles[11]` (carrier) | facing 0–7 (`dex = d`, `fmain.c:1507`) |
| Grounded, not ridden | `riding != 11` && `actor_file == 11` | `cfiles[4]` (RAFT) | fixed frame `1` (`atype = RAFT; inum = 1`, `fmain.c:2463-2464`) |

The RAFT sheet (cfile 4, file\_id 1348) holds 2 images: frame 0 is the raft itself, frame 1 is the grounded-swan image. Frame 1 is reachable **only** via the grounded-swan render override; the raft handler itself never writes `an->index`, so the raft always uses frame 0.

When grounded, the swan holds position (`xtest = abs_x; ytest = abs_y`, `fmain.c:1504-1506`) — it does not walk or animate.

### 21.5 Dragon

- Actor slot 3, type DRAGON, `actor_file = 10`
- Extent zone 2 (dragon cave area)
- **Hostile** — not rideable
- HP: 50, shoots fireballs (type 2 missiles) with 25% chance per frame at speed 5
- Always faces south
- Can be killed

### 21.6 Carrier Loading

`load_carrier(n)` loads carrier sprites into the ENEMY shape memory slot — carriers and enemies share memory and cannot coexist. Carriers always occupy `anim_list[3]`. Loading sets `anix = 4` and positions the carrier at the center of its extent zone.

### 21.7 Carrier Interactions

| Interaction | Behavior |
|-------------|----------|
| Doors | All riding values block door entry |
| Random encounters | Suppressed while `active_carrier != 0` |
| Combat | Carriers skip melee/missile hit detection |
| Freeze spell | Blocked when `riding > 1` (turtle or swan) |
| Stone circle teleport | Carrier teleports with hero |
| Rendering | Carriers skip terrain masking |

---


