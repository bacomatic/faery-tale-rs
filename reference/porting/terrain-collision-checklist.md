# Terrain & Collision Checklist

Source scope:
- `fsubs.asm:542-620` (`_px_to_im` — pixel-to-terrain lookup)
- `fmain.c:1081-1095` (`mapxy` usage and tile lookup)
- `fmain2.c:277-293` (`proxcheck`)
- `fmain.c:1580-1603` (speed table, ice momentum)
- `fmain.c:1760-1800` (environ effects: drowning, sinking, fire)
- `fmain.c:1830-1850` (fiery_death and rose override)

Purpose:
- Ensure ports implement terrain type resolution, collision, and all environ-driven gameplay effects correctly.

## A. `_px_to_im` Pipeline (`fsubs.asm:542-620`)

- [ ] Convert pixel to image coords: `imx = x >> 4`, `imy = y >> 5` (16px wide tiles, 32px tall tiles).
- [ ] Convert image to sector coords: `secx = (imx >> 4) - xreg`, `secy = (imy >> 3) - yreg` (sectors are 16×8 images).
- [ ] Look up sector number: `sector_id = map_mem[secy * 128 + secx + xreg]`.
- [ ] Look up tile ID: `tile_id = sector_mem[sector_id * 128 + (imy & 7) * 16 + (imx & 15)]`.
- [ ] Look up terrain type: `type = terra_mem[tile_id * 4 + 1] >> 4` (upper nibble of byte 1 in terra data).
- [ ] This is the authoritative terrain type used by all collision and environ systems.

## B. `proxcheck()` Semantics (`fmain2.c:277-293`)

- [ ] Returns terrain type (0–15) at the given pixel coordinates using `_px_to_im`.
- [ ] Returns -1 (or equivalent sentinel) when coordinates are outside valid bounds.
- [ ] Solid terrain types (block movement): 1 (solid wall), 9 (solid 2), 10, 11, 12, 13, 14, 15 (door) — verify exact set from source.
- [ ] Passable terrain types (walk-through with effects): 2–7 (water/ice — never blocked by `_prox`).
- [ ] Type 15 (door tile) returns 15 from `proxcheck` to trigger door interaction for player (`fmain.c:1607`).

## C. Terrain Type Table and Effects (`fmain.c:1760-1800`)

- [ ] Type 0: Open ground — no environ effect.
- [ ] Type 1: Solid wall — movement blocked by `_prox`; environ unchanged.
- [ ] Type 2: Shallow water (slow) — `environ` increments toward 2; no drowning yet (`fmain.c:1773`).
- [ ] Type 3: Water (medium) — `environ` increments toward 5 (`fmain.c:1774`).
- [ ] Type 4: Deep water — `environ` increments toward 10; drowning begins at `environ > 15` (`fmain.c:1775-1795`).
- [ ] Type 5: Very deep water — `environ` increments toward 30; at 30 player enters SINK → death; sector 181 teleports to region 9 (`fmain.c:1775-1793`).
- [ ] Type 6: Ice (slippery) — `environ = -1`, speed forced to 4 (`fmain.c:1771`, `fmain.c:1601`).
- [ ] Type 7: Ice (velocity) — `environ = -2`, momentum-based physics (`fmain.c:1772`, `fmain.c:1580-1595`).
- [ ] Type 8: Lava/fire — `environ = -3`, player walks backwards at speed -2 (`fmain.c:1770`, `fmain.c:1600`).
- [ ] Type 15: Door tile — handled as passable by `_prox`; triggers door interaction from `proxcheck` return.

## D. `environ` Field — State Accumulator

- [ ] Each actor has its own `environ` field tracking water depth or slide state.
- [ ] For water types (2–5): `environ` accumulates each tick the player stands in water; cleared when leaving water.
- [ ] For ice types (6,7): `environ` set to fixed negative sentinel each tick on ice; cleared when leaving ice.
- [ ] For fire type (8): `environ = -3` each tick; player pushed backward at speed -2 (`fmain.c:1600`).
- [ ] Drowning thresholds (`fmain.c:1780-1795`):
  - `environ > 2`: `vitality--` (gradual drowning)
  - `environ > 15`: instant death (`vitality = 0`)
- [ ] Rose override (`stuff[23]`): if player has Rose, force `environ = 0` in fiery_death zones — `fmain.c:1844`.
- [ ] Turtle shell (`stuff[23]` — verify exact item): if player has it, `environ` forced to 0 — `fmain.c:1844` (confirm item index).

## E. Speed Table by Environ (`fmain.c:1599-1603`)

- [ ] Normal (`environ >= 0` and not ice): speed from stats formula.
- [ ] Ice slippery (`environ == -1`): speed = 4 — `fmain.c:1601`.
- [ ] Ice velocity (`environ == -2`): momentum-based — `fmain.c:1580-1595`.
- [ ] Fire/lava (`environ == -3`): speed = -2 (backwards push) — `fmain.c:1600`.
- [ ] Ice momentum: velocity preserved across ticks; direction only adjustable at edges of inertia (`fmain.c:1580-1595`).

## F. Movement Collision Loop (`fmain.c:1560-1665`)

- [ ] Compute test position `(newx, newy)` from current direction and speed.
- [ ] Call `proxcheck(newx, newy)` on test position.
- [ ] On solid return: zero velocity, set `state = STILL`, set `tactic = FRUST` for non-player actors (`fmain.c:1660-1661`).
- [ ] On passable (0 or 2–8): update `hero_x/hero_y`; apply environ effect for type.
- [ ] Player movement: tested at pixel level; NPC movement: same pipeline via actor loop.

## G. Item-Gated Terrain (`fmain.c:1830-1920`)

- [ ] Crystal Shard (`stuff[30]`): required to pass certain tile types in dungeon — verify exact check (`fmain.c:3590-3595`).
- [ ] Statues (`stuff[25]`, count=5 Gold Statues): required to enter desert gate region — `fmain.c:1919`, `fmain.c:3594`.
- [ ] Rose (`stuff[23]`): grants fire immunity (forces `environ=0` in `fiery_death` box) — `fmain.c:1844`.
- [ ] Stone circle teleport: specific sector triggers world transfer; check sector 181 and whirlpool path — `fmain.c:1789-1793`.

## H. Desert Gate Dual-Check (`fmain.c:1915-1925`)

- [ ] Desert gate requires two independent checks: item-count check (`stuff[25] >= 5`) AND proximity check to gate tile — `fmain.c:1919`.
- [ ] Both checks must pass; only one is not sufficient.

## I. Known Quirks To Preserve (or Deliberately Normalize)

- [ ] `environ` is set to a sentinel negative value each tick on ice — NOT accumulated; any exit from ice immediately clears it.
- [ ] `proxcheck` returns 15 for door tiles regardless of whether the door is locked or open — door state lives only in the map tile ID.
- [ ] Water speed reduction applies per-tick while standing in water, even without moving.
- [ ] Fire terrain forces backward movement at `speed = -2` rather than blocking movement.

## J. Minimum Parity Test Matrix

- [ ] Walking into type 1 tile: movement stops; non-player sets `tactic = FRUST`.
- [ ] Standing in type 4 water: `environ` grows; at `environ > 2` vitality decrements each tick.
- [ ] Standing in type 5 water: death or teleport to region 9 when `environ >= 30`.
- [ ] Standing on ice type 7: momentum physics — direction change lag proportional to velocity.
- [ ] Standing on fire type 8: pushed backward at speed -2 regardless of joystick input.
- [ ] Player with Rose in fire zone: `environ` stays 0, no backward push, no damage.
- [ ] Door tile (type 15) at `proxcheck` triggers `doorfind` at `i==0`; does NOT block movement at terrain level.
