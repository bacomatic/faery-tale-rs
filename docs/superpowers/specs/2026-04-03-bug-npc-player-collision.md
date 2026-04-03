# Bug: NPC-player actor collision missing — enemy NPCs swarm onto player — [#160](https://github.com/bacomatic/faery-tale-rs/issues/160)

**Filed:** 2026-04-03
**Status:** designed

## Description

Enemy NPCs chase and walk directly on top of the player character. There is no actor-vs-actor collision — NPCs and the player can occupy the same pixel position. In the original game, actors could not overlap; movement that would cause overlap was blocked.

## Investigation

- **Affected files:**
  - `src/game/collision.rs` — `proxcheck()` only checks terrain, missing actor collision loop
  - `src/game/npc.rs` — `Npc::tick()` only calls terrain `proxcheck()`, no actor awareness
  - `src/game/gameplay_scene.rs` — `update_actors()` movement pass has no actor collision

- **Root cause (confirmed):**
  The original game's `proxcheck()` (fmain2.c:395–427) had two phases:
  1. Terrain collision via `prox()` — **this is ported**
  2. Actor-actor bounding box collision loop — **this is NOT ported**

  The original's actor collision loop iterated all actors and returned 16 if:
  - `|dx| < 11 && |dy| < 9` (pixel distance)
  - Skipped self (`i != j`), companion slot (`j != 1`), objects (`type != 5`), and dead actors

  The Rust port's `proxcheck()` only does terrain checks. NPCs call `npc.tick()` which calls `proxcheck()` for terrain only. Nothing prevents NPCs from moving to the player's position.

- **Reproduction path:** Run the game, encounter any enemy group. Enemies walk directly onto the hero's position and stack up.

- **Evidence:**
  - Original C: `fmain2.c` lines 395–427 — `proxcheck()` has explicit actor collision loop returning 16
  - Original C: `fmain2.c` lines 467–477 — `move_figure()` blocks movement on any non-zero `proxcheck()` result
  - Rust port: `collision.rs` `proxcheck()` — no actor iteration, only terrain
  - Rust port: `npc.rs` `Npc::tick()` — no actor position awareness

## Fix Design

### Approach: Add actor collision to NPC movement loop

Match the original `proxcheck()` behavior by adding actor-vs-actor bounding box collision.

**Changes:**
1. **`collision.rs`**: Add `actor_blocks(x, y, others: &[(i32, i32)]) -> bool` that checks the original's bounding box: `|dx| < 11 && |dy| < 9`.
2. **`npc.rs`**: Add `Npc::tick_with_collision()` that accepts `&[(i32, i32)]` of other actor positions and checks both terrain + actor collision for the proposed move and wall-slide alternatives.
3. **`gameplay_scene.rs`**: In `update_actors()`, replace the simple `npc.tick()` movement loop with an indexed loop that builds live actor positions (hero + other NPCs) and calls the collision-aware tick. Positions update sequentially (matching original behavior where later actors see updated positions of earlier ones).

**Collision box** (from original):
- X range: `dx > -11 && dx < 11` (21px wide)
- Y range: `dy > -9 && dy < 9` (17px tall)

**Skip conditions** (from original):
- Dead actors (walkable)
- The moving actor itself

## Plan Reference

TBD — will be produced by writing-plans.
