## 19. Magic System

### 19.1 Preconditions

1. Must have the item (`stuff[4 + hit] > 0`); otherwise event(21) "if only I had some Magic!"
2. Cannot use in Necromancer arena (`extn->v3 == 9`); `speak(59)`

### 19.2 Magic Items

| hit | Item | stuff[] | Effect |
|-----|------|---------|--------|
| 5 | Blue Stone | 9 | Teleport via standing stones. Requires `hero_sector == 144`, uses `stone_list[]`. Destination = `(current_stone + facing + 1) % 11`. |
| 6 | Green Jewel | 10 | `light_timer += 760`. Temporary light-magic effect: adds 200 to red channel in `day_fade()`, producing warm amber glow outdoors. |
| 7 | Glass Vial | 11 | Heal: `vitality += rand8() + 4` (4–11 HP), capped at `15 + brave / 4`. |
| 8 | Crystal Orb | 12 | `secret_timer += 360`. Reveals hidden passages while countdown active. In dungeons (region 9), changes color 31 to bright green (`0x00f0`). |
| 9 | Bird Totem | 13 | Renders overhead map with hero position marker. Sets `viewstatus = 1`. |
| 10 | Gold Ring | 14 | `freeze_timer += 100`. Freezes all enemies, stops daynight advance, suppresses encounters. Blocked if `riding > 1`. |
| 11 | Jade Skull | 15 | Kill spell: kills all visible enemies with `vitality > 0`, `type == ENEMY`, `race < 7`. `brave--` per kill (counterbalances normal combat `brave++`). |

### 19.3 Timer Effects

| Timer | Declared | While > 0 |
|-------|----------|-----------|
| `freeze_timer` | short | All non-hero actors skip movement (see §9.5); `daynight` frozen; encounters suppressed. AI tactic selection may still run for non-hostile NPCs, but none move. |
| `light_timer` | short | Green Jewel warm amber glow in `day_fade()` |
| `secret_timer` | short | Secret passages visible; dungeon color 31 = bright green |

All timers decrement by 1 each tick (when nonzero). All reset to 0 on brother succession.

### 19.4 Charge Depletion

After use: `stuff[4 + hit]--`. If the count reaches 0, `set_options()` rebuilds the menu to remove the depleted item. Failed uses (Blue Stone position check fails, Gold Ring blocked by riding) do NOT consume a charge.

---


