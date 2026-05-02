## 13. Doors & Buildings

### Requirements

| ID | Requirement |
|----|-------------|
| R-DOOR-001 | 86 door entries (`DOORCOUNT = 86`) shall be supported, each with outdoor coordinates (`xc1`, `yc1`), indoor coordinates (`xc2`, `yc2`), type (visual/orientation), and secs (1 = region 8 buildings, 2 = region 9 dungeons). The table shall be sorted by `xc1` ascending. |
| R-DOOR-002 | Outdoor-to-indoor transitions shall use O(log n) binary search on the X-sorted door table. Hero position shall be aligned to 16×32 tile grid (`xc1 = hero_x & 0xfff0`, `yc1 = hero_y & 0xffe0`) before lookup. |
| R-DOOR-003 | Indoor-to-outdoor transitions shall use O(n) linear scan matching on `xc2`/`yc2` with a wider hit zone for horizontal doors. |
| R-DOOR-004 | Door orientation: odd type = horizontal, even type = vertical. Horizontal doors shall skip entry if `hero_y & 0x10` is set. Vertical doors shall skip entry if `(hero_x & 15) > 6`. |
| R-DOOR-005 | Locked doors (terrain tile type 15) shall be opened using the `open_list[17]` table matching sector tile ID, region image block, and required key type. |
| R-DOOR-006 | DESERT door type shall be blocked unless `stuff[STATBASE] >= 5` (5 gold statues). |
| R-DOOR-007 | The `xfer()` teleport function shall: adjust map scroll by the same delta as hero position, set hero position to destination, clear encounters, recalculate region from coordinates (on exit only), load region data, regenerate minimap, force full screen redraw, update music mood, and nudge hero downward if colliding with a solid object at destination. |
| R-DOOR-008 | Players shall not be able to enter doors while mounted (`riding` check). |
| R-DOOR-009 | Door entry destination offsets shall vary by type: CAVE/VLOG → (`xc2 + 24`, `yc2 + 16`); horizontal → (`xc2 + 16`, `yc2`); vertical → (`xc2 - 1`, `yc2 + 16`). |
| R-DOOR-010 | Door exit destination offsets shall vary by type: CAVE/VLOG → (`xc1 - 4`, `yc1 + 16`); horizontal → (`xc1 + 16`, `yc1 + 34`); vertical → (`xc1 + 20`, `yc1 + 16`). |
| R-DOOR-011 | Entering a building shall use a visual fade transition (`fade_page(100,100,100,TRUE,pagecolors)`). Exiting shall be instant (no fade). |
| R-DOOR-012 | Quicksand-to-dungeon transition: when `environ == 30` at `hero_sector == 181`, the player shall teleport to `(0x1080, 34950)` in region 9. NPCs in the same quicksand shall die. |
| R-DOOR-013 | Door tile changes from `doorfind()` shall be transient — they modify live `sector_mem` only. Changes are lost when the sector reloads from disk. No save mechanism shall preserve opened door tiles. |
| R-DOOR-014 | The `doorfind()` algorithm shall: locate terrain type 15 at 3 X-offsets (x, x+4, x−8), find the top-left corner by scanning left (up to 32px) and down (32px), convert to image coordinates (`x >>= 4; y >>= 5`), determine sector/region IDs, search `open_list[17]` for a matching entry (map_id and door_id, with key check `keytype == 0 || keytype == open_list[j].keytype`), and replace tiles on success or print "It's locked." on failure (suppressed by `bumped` flag). |
| R-DOOR-015 | Collision-triggered door opening: when the player bumps terrain type 15, `doorfind(xtest, ytest, 0)` shall be called automatically, opening only NOKEY doors. |
| R-DOOR-016 | CAVE and VLOG door types shall share value 18; code checking for CAVE shall also match VLOG entries. Both shall use the same teleportation offset. |
| R-DOOR-017 | Key types for locked doors: NOKEY (0), GOLD (1), GREEN (2), KBLUE (3), RED (4), GREY (5), WHITE (6). Keys shall be consumed on successful use (`stuff[hit + KEYBASE]--`). |
| R-DOOR-018 | The Stargate portal shall be implemented as the door pair at `doorlist[14..15]` using the generic `STAIR` door type (value 15, horizontal) with **no** stargate-specific code path. Entry 14 transitions outdoor Citadel-of-Doom approach → region 8 Doom castle interior (sectors 135–138); entry 15 transitions region 8 interior → region 9 Spirit Plane (astral sectors 43–59, 100, 143–149). The Stargate doors themselves shall have no key requirement, no statue requirement, and no item check — progression is gated upstream by the Rose (lava immunity to reach the Citadel) and downstream by the Crystal Shard (terrain-12 walls inside the Spirit Plane), never by the door. Fade-on-enter / instant-on-exit follows the standard door transition rules. |

### User Stories

- As a player, I can enter buildings through doors and transition seamlessly between outdoor and indoor areas.
- As a player, I need specific colored keys to open locked doors, and the key is consumed on use.
- As a player, I cannot enter the desert city door until I have collected 5 golden statues.
- As a player, I must dismount my carrier before I can enter any building.
- As a player, I can bump into unlocked doors to open them automatically.
- As a player, sinking fully in quicksand at the correct location teleports me to the dungeon.
- As a player, once I can cross the lava to the Citadel of Doom, the Stargate inside transports me to the Spirit Plane — the door itself requires no key or item, but the path to it requires the Rose for lava immunity.

---


