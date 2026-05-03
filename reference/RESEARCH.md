# The Faery Tale Adventure — Game Mechanics Research

Ground-truth documentation of every game mechanic, derived exclusively from the original 1987 source code. All claims are backed by file-and-line citations. Open questions are logged in [PROBLEMS.md](PROBLEMS.md).

> **Citation format**: `file.c:LINE` or `file.c:START-END`. Speech references: `speak(N)`.

This document was split into focused sub-documents for readability. The section numbering is preserved; all original `§N` anchors below link to the corresponding sub-document.

---

## Section Index

| §  | Topic | File |
|----|-------|------|
| 1  | Core Data Structures | [RESEARCH-data-structures.md](RESEARCH-data-structures.md) |
| 2  | Actor State Machine | [RESEARCH-data-structures.md](RESEARCH-data-structures.md) |
| 3  | Random Number Generation | [RESEARCH-data-structures.md](RESEARCH-data-structures.md) |
| 4  | Input System | [RESEARCH-input-movement.md](RESEARCH-input-movement.md) |
| 5  | Movement & Direction | [RESEARCH-input-movement.md](RESEARCH-input-movement.md) |
| 6  | Terrain & Collision | [RESEARCH-terrain-combat.md](RESEARCH-terrain-combat.md) |
| 7  | Combat System | [RESEARCH-terrain-combat.md](RESEARCH-terrain-combat.md) |
| 8  | AI System | [RESEARCH-ai-encounters.md](RESEARCH-ai-encounters.md) |
| 9  | Encounter & Spawning | [RESEARCH-ai-encounters.md](RESEARCH-ai-encounters.md) |
| 10 | Inventory & Items | [RESEARCH-items-world.md](RESEARCH-items-world.md) |
| 11 | World Objects | [RESEARCH-items-world.md](RESEARCH-items-world.md) |
| 12 | Door System | [RESEARCH-items-world.md](RESEARCH-items-world.md) |
| 13 | NPC Dialogue & Quests | [RESEARCH-npcs-quests.md](RESEARCH-npcs-quests.md) |
| 14 | Hunger, Fatigue & Stats | [RESEARCH-npcs-quests.md](RESEARCH-npcs-quests.md) |
| 15 | Brother Succession | [RESEARCH-npcs-quests.md](RESEARCH-npcs-quests.md) |
| 16 | Win Condition & Princess Rescue | [RESEARCH-npcs-quests.md](RESEARCH-npcs-quests.md) |
| 17 | Main Game Loop | [RESEARCH-systems.md](RESEARCH-systems.md) |
| 18 | Menu System | [RESEARCH-systems.md](RESEARCH-systems.md) |
| 19 | Day/Night Cycle | [RESEARCH-systems.md](RESEARCH-systems.md) |
| 20 | Text & Message Display | [RESEARCH-systems.md](RESEARCH-systems.md) |
| 21 | Save/Load System | [RESEARCH-systems.md](RESEARCH-systems.md) |

---

## Anchor Stubs

The anchors below preserve backward compatibility with existing `RESEARCH.md#section-anchor` links throughout the documentation. Each stub links to the full content in the appropriate sub-document.

### [1. Core Data Structures](RESEARCH-data-structures.md#1-core-data-structures)

See [RESEARCH-data-structures.md](RESEARCH-data-structures.md#1-core-data-structures).

### [2. Actor State Machine](RESEARCH-data-structures.md#2-actor-state-machine)

See [RESEARCH-data-structures.md](RESEARCH-data-structures.md#2-actor-state-machine).

#### [2.4 statelist — Animation Frame Lookup](RESEARCH-data-structures.md#24-statelist--animation-frame-lookup)

See [RESEARCH-data-structures.md §2.4](RESEARCH-data-structures.md#24-statelist--animation-frame-lookup).

### [3. Random Number Generation](RESEARCH-data-structures.md#3-random-number-generation)

See [RESEARCH-data-structures.md](RESEARCH-data-structures.md#3-random-number-generation).

### [4. Input System](RESEARCH-input-movement.md#4-input-system)

See [RESEARCH-input-movement.md](RESEARCH-input-movement.md#4-input-system).

### [5. Movement & Direction](RESEARCH-input-movement.md#5-movement--direction)

See [RESEARCH-input-movement.md](RESEARCH-input-movement.md#5-movement--direction).

#### [5. Rendering & Display](RESEARCH-input-movement.md#5-movement--direction)

Legacy anchor — content is in [RESEARCH-input-movement.md §5](RESEARCH-input-movement.md#5-movement--direction).

### [6. Terrain & Collision](RESEARCH-terrain-combat.md#6-terrain--collision)

See [RESEARCH-terrain-combat.md](RESEARCH-terrain-combat.md#6-terrain--collision).

### [7. Combat System](RESEARCH-terrain-combat.md#7-combat-system)

See [RESEARCH-terrain-combat.md](RESEARCH-terrain-combat.md#7-combat-system).

#### [7.9 Good Fairy & Brother Succession](RESEARCH-terrain-combat.md#79-good-fairy--brother-succession)

See [RESEARCH-terrain-combat.md §7.9](RESEARCH-terrain-combat.md#79-good-fairy--brother-succession).

### [8. AI System](RESEARCH-ai-encounters.md#8-ai-system)

See [RESEARCH-ai-encounters.md](RESEARCH-ai-encounters.md#8-ai-system).

#### [8.4 Frustration Cycle](RESEARCH-ai-encounters.md#84-frustration-cycle)

See [RESEARCH-ai-encounters.md §8.4](RESEARCH-ai-encounters.md#84-frustration-cycle).

### [9. Encounter & Spawning](RESEARCH-ai-encounters.md#9-encounter--spawning)

See [RESEARCH-ai-encounters.md](RESEARCH-ai-encounters.md#9-encounter--spawning).

#### [9.6 Special Extents](RESEARCH-ai-encounters.md#96-special-extents)

See [RESEARCH-ai-encounters.md §9.6](RESEARCH-ai-encounters.md#96-special-extents).

#### [9.8 Dark Knight (DKnight)](RESEARCH-ai-encounters.md#98-dark-knight-dknight)

See [RESEARCH-ai-encounters.md §9.8](RESEARCH-ai-encounters.md#98-dark-knight-dknight).

### [10. Inventory & Items](RESEARCH-items-world.md#10-inventory--items)

See [RESEARCH-items-world.md](RESEARCH-items-world.md#10-inventory--items).

### [11. World Objects](RESEARCH-items-world.md#11-world-objects)

See [RESEARCH-items-world.md](RESEARCH-items-world.md#11-world-objects).

#### [11.9 Save/Load of Object State](RESEARCH-items-world.md#119-saveload-of-object-state)

See [RESEARCH-items-world.md §11.9](RESEARCH-items-world.md#119-saveload-of-object-state).

### [12. Door System](RESEARCH-items-world.md#12-door-system)

See [RESEARCH-items-world.md](RESEARCH-items-world.md#12-door-system).

### [13. NPC Dialogue & Quests](RESEARCH-npcs-quests.md#13-npc-dialogue--quests)

See [RESEARCH-npcs-quests.md](RESEARCH-npcs-quests.md#13-npc-dialogue--quests).

#### [13.4 Quest Progression Flags](RESEARCH-npcs-quests.md#134-quest-progression-flags)

See [RESEARCH-npcs-quests.md §13.4](RESEARCH-npcs-quests.md#134-quest-progression-flags).

#### [13.6 Shop System](RESEARCH-npcs-quests.md#136-shop-system)

See [RESEARCH-npcs-quests.md §13.6](RESEARCH-npcs-quests.md#136-shop-system).

### [14. Hunger, Fatigue & Stats](RESEARCH-npcs-quests.md#14-hunger-fatigue--stats)

See [RESEARCH-npcs-quests.md](RESEARCH-npcs-quests.md#14-hunger-fatigue--stats).

### [15. Brother Succession](RESEARCH-npcs-quests.md#15-brother-succession)

See [RESEARCH-npcs-quests.md](RESEARCH-npcs-quests.md#15-brother-succession).

### [16. Win Condition & Princess Rescue](RESEARCH-npcs-quests.md#16-win-condition--princess-rescue)

See [RESEARCH-npcs-quests.md](RESEARCH-npcs-quests.md#16-win-condition--princess-rescue).

### [17. Main Game Loop](RESEARCH-systems.md#17-main-game-loop)

See [RESEARCH-systems.md](RESEARCH-systems.md#17-main-game-loop).

### [18. Menu System](RESEARCH-systems.md#18-menu-system)

See [RESEARCH-systems.md](RESEARCH-systems.md#18-menu-system).

### [19. Day/Night Cycle](RESEARCH-systems.md#19-daynight-cycle)

See [RESEARCH-systems.md](RESEARCH-systems.md#19-daynight-cycle).

### [20. Text & Message Display](RESEARCH-systems.md#20-text--message-display)

See [RESEARCH-systems.md](RESEARCH-systems.md#20-text--message-display).

### [21. Save/Load System](RESEARCH-systems.md#21-saveload-system)

See [RESEARCH-systems.md §21](RESEARCH-systems.md#21-saveload-system).
