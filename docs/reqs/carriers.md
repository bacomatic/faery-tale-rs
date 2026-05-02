## 18. Carriers (Raft, Turtle, Bird)

### Requirements

| ID | Requirement |
|----|-------------|
| R-CARRY-001 | Four carrier types shall be implemented: Raft (`riding=1`, RAFT type, cfiles[4]), Turtle (`riding=5`, CARRIER type, cfiles[5]), Swan (`riding=11`, CARRIER type, cfiles[11]), Dragon (DRAGON type, cfiles[10], hostile, not rideable). |
| R-CARRY-002 | Raft shall activate automatically when within 9px proximity, hero on water/shore terrain (px_to_im 3–5), and `wcarry==1`. It shall snap to hero position each frame. Dismount occurs automatically when conditions fail. |
| R-CARRY-003 | Turtle shall be summoned via USE menu (turtle item, `stuff[6]`). Boarding requires within 16px proximity and `wcarry==3`. Ridden speed shall be forced to 3 pixels/frame. The `raftprox` flag shall force `environ = 0` while riding (no drowning). When the hero moves >16px from the turtle, `raftprox` shall drop to 0 and the rider shall dismount automatically. While riding, hero movement shall use standard `proxcheck()` (the rider may walk onto land), but the turtle's `abs_x`/`abs_y` shall only update when the hero stands on terrain 5 — the turtle sprite stays at the water's edge when the rider walks onto land. |
| R-CARRY-004 | Turtle cannot be summoned in central region bounds (11194–21373 X, 10205–16208 Y). |
| R-CARRY-005 | When unridden, the turtle shall run its autonomous movement handler every tick at speed 3 using `px_to_im()` (not `proxcheck()`), committing position updates only when the probed terrain is exactly type 5 (very deep water). Types 2–4 and all land shall be impassable to the autonomous turtle. Each tick it shall probe 4 directions in priority order from current facing `d`: `d`, `(d+1)&7`, `(d-1)&7`, `(d-2)&7` — the first landing on terrain 5 is selected; if none succeed, the turtle does not move. The handler shall not persist the chosen direction to `an->facing`; facing shall instead be updated every 16 ticks by the CARRIER AI path (`set_course(i, hero_x, hero_y, 5)`), producing a slow hero-seeking drift along water. The extent-drift bug where `move_extent(1, xtest, ytest)` fires with stale probe coordinates on failed frames SHOULD be fixed (do not reproduce). |
| R-CARRY-006 | Swan shall require Golden Lasso (`stuff[5]`) to board. Boarding requires within 16px proximity and `wcarry==3`. Sets `riding=11`, `environ=-2` (airborne). |
| R-CARRY-007 | Swan movement shall use inertial flight physics: velocity accumulates via directional acceleration, max horizontal velocity ~32, max vertical ~40, position updates by `vel/4` per frame. No terrain collision (`proxcheck` skipped). |
| R-CARRY-008 | Swan dismount shall require: hero action button + velocity < 15 + clear ground below + not fiery terrain. Dismount blocked in lava zone (event 32: "Ground is too hot") and at high velocity (event 33: "Flying too fast"). |
| R-CARRY-009 | Swan on ground shall render using RAFT sprite. Auto-face into wind via `set_course(0,-nvx,-nvy,6)`. |
| R-CARRY-010 | Dragon shall be stationary, shoot fire missiles (type 2) with 25% chance per frame at speed 5, always face south. HP: 50, can be killed. |
| R-CARRY-011 | Carriers and enemies shall share the ENEMY shape memory slot — they cannot coexist. Carriers always occupy `anim_list[3]`. Loading sets `anix=4`. |
| R-CARRY-012 | While riding: door entry blocked, random encounters suppressed, carriers skip melee/missile hit detection, freeze spell blocked when `riding > 1`. |
| R-CARRY-013 | Stone circle teleport shall move carrier with hero. |
| R-CARRY-014 | Carriers shall skip terrain masking during rendering. |
| R-CARRY-015 | Mounted-turtle melee recoil can push the rider across invalid terrain (original behavior — do not fix). |

### User Stories

- As a player, I can ride a raft across water by walking near it on shore.
- As a player, I can summon and ride a turtle to navigate ocean areas.
- As a player, I can lasso and ride a swan to fly over any terrain.
- As a player, I encounter a hostile dragon that breathes fire.
- As a player, I cannot enter doors or use freeze while riding.

---


