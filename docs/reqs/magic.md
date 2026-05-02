## 16. Magic

### Requirements

| ID | Requirement |
|----|-------------|
| R-MAGIC-001 | 7 magic items shall be usable from the MAGIC menu, each requiring the item in inventory (`stuff[4 + hit] > 0`); otherwise event(21) "if only I had some Magic!". |
| R-MAGIC-002 | Magic shall be blocked in the Necromancer arena (extent `v3 == 9`): `speak(59)`. |
| R-MAGIC-003 | Blue Stone (`stuff[9]`, hit 5): teleport via stone ring network. Requires `hero_sector == 144`. Destination = `(current_stone + facing + 1) % 11`. |
| R-MAGIC-004 | Green Jewel (`stuff[10]`, hit 6): `light_timer += 760`. Outdoor-only warm amber tint via `day_fade()`. |
| R-MAGIC-005 | Glass Vial (`stuff[11]`, hit 7): heal `vitality += rand8() + 4` (4–11 HP), capped at `15 + brave / 4`. |
| R-MAGIC-006 | Crystal Orb (`stuff[12]`, hit 8): `secret_timer += 360`. Reveals hidden passages while active. In dungeons (region 9), color 31 turns bright green (`0x00f0`). |
| R-MAGIC-007 | Bird Totem (`stuff[13]`, hit 9): renders overhead map with hero position marker. Sets `viewstatus = 1`. |
| R-MAGIC-008 | Gold Ring (`stuff[14]`, hit 10): `freeze_timer += 100`. Freezes all enemies, stops daynight advance, suppresses encounters. Blocked when `riding > 1`. |
| R-MAGIC-009 | Jade Skull (`stuff[15]`, hit 11): kill all visible enemies with `vitality > 0`, `type == ENEMY`, `race < 7`. Brave −1 per kill (counterbalances normal combat brave++). |
| R-MAGIC-010 | After successful use: `stuff[4 + hit]--`. If depleted (reaches 0), rebuild menu via `set_options()`. Failed uses (wrong location, blocked) do NOT consume a charge. |

### User Stories

- As a player, I can use magic items from the menu to aid exploration and combat.
- As a player, magic items have limited charges that deplete with successful use.
- As a player, magic is blocked in the final boss arena.
- As a player, the Jade Skull kills enemies but reduces my bravery.

---


