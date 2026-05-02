# AI Goals & Tactics Checklist

Source scope:
- `fmain.c:2109-2183` (main AI loop)
- `fmain.c:2146-2171` (hostile AI detail, melee engagement, DKnight)
- `fmain.c:1655-1665` (frustration on blocked movement)
- `fmain2.c:62-225` (`set_course` implementation)
- `ftale.h` (goal/tactic enum values)

Purpose:
- Ensure ports implement all goal modes, tactic execution, frustration cycle, and override conditions with parity.

## A. Goal Mode Enumeration

- [ ] Implement all goal modes: ATTACK1=1, ATTACK2=2, ARCHER1=3, ARCHER2=4, FLEE=5(?), FOLLOWER, STAND, WAIT, CONFUSED=10 — verify exact values against `ftale.h`.
- [ ] Implement all tactic values: FRUST=0, FOLLOW=2, BUMBLE_SEEK=3, RANDOM=4, BACKUP=5, EVADE=6, HIDE=7, EGG_SEEK=8, SHOOTFRUST=9 — `fmain.c:2141-2161`.
- [ ] Tactic values HIDE(7), DOOR_SEEK(11), DOOR_LET(12) are declared but NOT handled in `do_tactic()` — fall through silently (`fmain.c:185`).
- [ ] FRUST(0) and SHOOTFRUST(9) are NOT handled in `do_tactic()` — intercepted before dispatch in the main loop (`fmain.c:2141-2143`).

## B. Main Loop Structure (`fmain.c:2109-2183`)

- [ ] Skip indices 0 (player) and 1 (raft) — loop starts at `i=2` (`fmain.c:2109`).
- [ ] Break immediately if `goodfairy > 0 && goodfairy < 120` — entire AI suspended during fairy resurrection (`fmain.c:2112`).
- [ ] CARRIER type: every 16 ticks (`daynight & 15 == 0`), face player with `set_course(i,hero_x,hero_y,5)`, then skip AI (`fmain.c:2114-2117`).
- [ ] SETFIG type: skip entirely — no AI processing (`fmain.c:2119`).
- [ ] Compute `mode` and `tactic` as local copies; write `mode` back as `an->goal` at loop end (`fmain.c:2120-2121`, `fmain.c:2182`).
- [ ] Update `leader = i` when `leader == 0` at loop end (first non-player actor seen becomes leader) — `fmain.c:2183`.

## C. Override Checks (Applied Before Goal Dispatch, `fmain.c:2133-2140`)

- [ ] Hero dead or in fall state: if `leader == 0`, set `mode = FLEE`; if `leader != 0`, set `mode = FOLLOWER` — `fmain.c:2133-2136`.
- [ ] `vitality < 2`: unconditionally set `mode = FLEE` — `fmain.c:2137-2138`.
- [ ] Special encounter mismatch: if `xtype > 59 && an->race != extn->v3`: set `mode = FLEE` — independently of vitality (`fmain.c:2138-2140`).
- [ ] Overrides apply before the frustration handler and before goal-mode dispatch.

## D. Frustration Handler (`fmain.c:2141-2143`)

- [ ] Check `tactic == FRUST || tactic == SHOOTFRUST` for ALL goal modes before mode dispatch.
- [ ] If ranged weapon (`an->weapon & 4`): call `do_tactic(i, rand4()+2)` → random from {FOLLOW(2), BUMBLE_SEEK(3), RANDOM(4), BACKUP(5)}.
- [ ] If melee weapon: call `do_tactic(i, rand2()+3)` → random from {BUMBLE_SEEK(3), RANDOM(4)}.
- [ ] Frustration is set by the movement blocked handler: `an->tactic = FRUST` at `fmain.c:1660-1661`.

## E. Hostile AI (`mode ≤ ARCHER2`, `fmain.c:2146-2171`)

- [ ] Reconsider frequency: for `mode & 2 == 0` (ATTACK1, ARCHER2): `r = !rand4()` (1/4 chance); others keep `r = !bitrand(15)` (1/16 chance) — `fmain.c:2148`.
- [ ] When reconsidering (`r == true`), apply tactic selection table:
  - `race==4 && turtle_eggs`: EGG_SEEK — `fmain.c:2150`
  - `weapon < 1`: RANDOM, then set `mode = CONFUSED` — `fmain.c:2151-2152`
  - `vitality < 6 && rand2()`: EVADE — `fmain.c:2153-2154`
  - Archer (`weapon & 4`), `xd < 40 && yd < 30`: BACKUP — `fmain.c:2156`
  - Archer, `xd < 70 && yd < 70`: SHOOT — `fmain.c:2157`
  - Archer, far: PURSUE — `fmain.c:2158`
  - Melee default: PURSUE — `fmain.c:2160`
- [ ] Melee engagement threshold: `thresh = 14 - mode`; DKnight (race 7) always uses thresh=16 — `fmain.c:2162-2163`.
- [ ] Melee engagement: if `!(weapon & 4) && xd < thresh && yd < thresh`: `set_course(i, hero_x, hero_y, 0)` then `state = FIGHTING` — `fmain.c:2164-2166`.
- [ ] DKnight override: if `race == 7 && vitality > 0` and NOT in melee range: set `state = STILL, facing = 5` — overrides `do_tactic()` — `fmain.c:2168-2169`.

## F. Non-Hostile Modes

- [ ] FLEE: calls `do_tactic(i, BACKUP)` unconditionally — `fmain.c:2172`.
- [ ] FOLLOWER: calls `do_tactic(i, FOLLOW)` unconditionally — `fmain.c:2173`.
- [ ] STAND: calls `set_course(i, hero_x, hero_y, 0)` then forces `state = STILL` — faces player but never walks — `fmain.c:2174-2176`.
- [ ] WAIT: sets `state = STILL` only — no facing change — `fmain.c:2178`.
- [ ] CONFUSED and unrecognized modes: fall through with no processing — actor retains prior motion state — `fmain.c:2180`.

## G. `do_tactic()` Implementations (`fmain.c:84-185` approx)

- [ ] FOLLOW(2): `set_course(i, hero_x, hero_y, 1)` — approach with close-range wobble.
- [ ] BUMBLE_SEEK(3): `set_course(i, hero_x, hero_y, 4)` — diagonal-biased approach.
- [ ] RANDOM(4): randomize direction and set `state = WALKING`.
- [ ] BACKUP(5): `set_course(i, hero_x, hero_y, 3)` — REVERSE mode (moves away from player).
- [ ] EVADE(6): `set_course(i, hero_x, hero_y, 2)` — close-proximity evasion with wobble.
- [ ] PURSUE: `set_course(i, hero_x, hero_y, 0)` — SMART_SEEK toward player.
- [ ] SHOOT: launch missile if available, else PURSUE.

## H. `set_course()` Modes (`fmain2.c:62-225`)

- [ ] Mode 0 (SMART_SEEK): suppress minor axis when dominant (`xabs/2 > yabs` → clear ydir). No wobble.
- [ ] Mode 1 (CLOSE_APPROACH): like mode 0, add ±1 wobble when total distance < 40.
- [ ] Mode 2 (CLOSE_PROXIMITY): like mode 0, add ±1 wobble when total distance < 30.
- [ ] Mode 3 (REVERSE): negate both direction components before lookup — moves AWAY from target.
- [ ] Mode 4 (BUMBLE): skip axis suppression — both axes always contribute.
- [ ] Mode 5 (FACE_ONLY): compute direction but do NOT set `state = WALKING` — actor faces but stays still — `fmain2.c:219-221`.
- [ ] Mode 6 (DIRECT_VECTOR): use `target_x/target_y` directly as xdif/ydif — `fmain2.c:82-84`.
- [ ] Direction lookup: `com2` table `{0,1,2,7,9,3,6,5,4}` at `fmain2.c:57`; value 9 means no valid direction → `state = STILL`.

## I. Minimum Parity Test Matrix

- [ ] Unarmed enemy (weapon=0): sets mode=CONFUSED; actor falls through AI with no movement update.
- [ ] Low-HP actor (vitality<2): always FLEE regardless of mode; uses BACKUP tactic.
- [ ] DKnight (race 7) in field: stays STILL facing south at direction 5; never calls `do_tactic()`.
- [ ] Blocked movement → FRUST tactic → randomized new tactic on next tick.
- [ ] Special encounter mismatch (`xtype > 59`, wrong race): FLEE fires independently of HP override.
- [ ] CARRIER actor: faces player every 16 ticks via FACE_ONLY; never enters goal-mode dispatch.
