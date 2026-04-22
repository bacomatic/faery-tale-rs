#!/usr/bin/env python3
"""Decode and dump the contents of a Faery Tale Adventure savegame file.

Savegame format derived from fmain2.c:1508-1527 (savegame function) and
fmain.c:3621-3631 (mod1save function). All multi-byte values are big-endian
(Motorola 68000).

Savegame layout (sequential):
  1. 80 bytes   — misc variables (map_x through pad1-pad3)
  2.  2 bytes   — region_num
  3.  6 bytes   — anix, anix2, mdex
  4. variable   — anim_list[anix] (22 bytes per entry)
  5. 35+35+35   — julstuff, philstuff, kevstuff (inventory per brother)
  6. variable   — missile_list[6] (10 bytes per entry)
  7. 24 bytes   — extent_list[2] (12 bytes per entry)
  8. 66 bytes   — ob_listg[11] (6 bytes per entry, glbobs=11)
  9. 20 bytes   — mapobs[10] (object counts per region)
 10. 20 bytes   — dstobs[10] (distributed flags per region)
 11. variable   — ob_table[0..9], each mapobs[i] * 6 bytes
"""

import argparse
import struct
import sys
from pathlib import Path

# -- Constants ---------------------------------------------------------------

GLBOBS = 11  # fmain2.c:1180
SIZEOF_SHAPE = 22  # struct shape — ftale.h:56-71
SIZEOF_MISSILE = 10  # struct missile — fmain.c:78-86 (padded to even)
SIZEOF_EXTENT = 12  # struct extent — fmain.c:335-338
SIZEOF_OBJECT = 6  # struct object — ftale.h:90-93

BROTHER_NAMES = ["Julian", "Phillip", "Kevin"]

# Inventory item names from fmain.c:380-416
ITEM_NAMES = [
    "Dirk", "Mace", "Sword", "Bow", "Magic Wand",          # 0-4
    "Golden Lasso", "Sea Shell", "Sun Stone", "Arrows",     # 5-8
    "Blue Stone", "Green Jewel", "Glass Vial", "Crystal Orb",  # 9-12
    "Bird Totem", "Gold Ring", "Jade Skull",                 # 13-15
    "Gold Key", "Green Key", "Blue Key", "Red Key",          # 16-19
    "Grey Key", "White Key",                                 # 20-21
    "Talisman", "Rose", "Fruit",                             # 22-24
    "Gold Statue", "Book", "Herb", "Writ", "Bone", "Shard", # 25-30
    "2 Gold Pieces", "5 Gold Pieces", "10 Gold Pieces",      # 31-33
    "100 Gold Pieces",                                       # 34
]

# Place names extracted from narr.asm:164-191
PLACE_NAMES = {
    0: "(none)",
    1: "(unchanged)",
    2: "Village of Tambry",
    3: "Vermillion Manor",
    4: "Mountains of Frost",
    5: "Plain of Grief",
    6: "City of Marheim",
    7: "Witch's Castle",
    8: "Graveyard",
    9: "Great Stone Ring",
    10: "Watchtower",
    11: "Great Bog",
    12: "Crystal Palace",
    13: "Pixle Grove",
    14: "Citadel of Doom",
    15: "Burning Waste",
    16: "Oasis",
    17: "City of Azal",
    18: "Outlying Fort",
    19: "Small Keep",
    20: "Old Castle",
    21: "Log Cabin",
    22: "Dark Stone Tower",
    23: "Isolated Cabin",
    24: "Tombs of Hemsath",
    25: "Forbidden Keep",
    26: "Hillside Cave",
}

MOTION_STATES = {
    0: "STANDING", 4: "FENCING", 8: "FALLING",
    12: "WALKING", 16: "SWIMMING", 20: "DEAD",
}

GOAL_MODES = {
    0: "STANDING", 1: "WALKING", 2: "FOLLOWING",
    3: "FIGHTING", 4: "GUARDING", 5: "FLEEING",
    6: "SEEKING", 7: "RETURNING", 8: "SHOOT",
    9: "SHOOTFRUST", 10: "EGG_SEEK", 11: "DOOR_SEEK",
    12: "DOOR_LET",
}

# Direction encoding: fmain.c direction convention
DIRECTIONS = {
    0: "NW", 1: "N", 2: "NE", 3: "E",
    4: "SE", 5: "S", 6: "SW", 7: "W",
}

WEAPON_TYPES = {
    0: "none", 1: "dagger", 2: "mace", 3: "sword", 4: "bow", 5: "wand",
}

REGION_NAMES = {
    0: "Snowy Region", 1: "Witch Wood", 2: "Swampy Region",
    3: "Plains/Rocks", 4: "Desert", 5: "Region 5",
    6: "Region 6", 7: "Region 7", 8: "Region 8", 9: "Region 9",
}


# -- Helpers -----------------------------------------------------------------

def u16(data, offset):
    """Read big-endian unsigned 16-bit value."""
    return struct.unpack_from(">H", data, offset)[0]


def s16(data, offset):
    """Read big-endian signed 16-bit value."""
    return struct.unpack_from(">h", data, offset)[0]


def s8(data, offset):
    """Read signed 8-bit value."""
    return struct.unpack_from(">b", data, offset)[0]


def u8(data, offset):
    """Read unsigned 8-bit value."""
    return data[offset]


def hexdump(data, offset, length, prefix="  "):
    """Return a hex dump string for a range of bytes."""
    chunk = data[offset:offset + length]
    hex_str = " ".join(f"{b:02x}" for b in chunk)
    return f"{prefix}[{offset:#06x}] {hex_str}"


def lookup(table, value, default=None):
    """Look up a value in a dict, returning a formatted string."""
    name = table.get(value)
    if name is not None:
        return f"{value} ({name})"
    if default is not None:
        return f"{value} ({default})"
    return str(value)


# -- Section decoders --------------------------------------------------------

def decode_misc(data, offset, show_hex):
    """Decode the 80-byte misc variables block. fmain.c:557-590"""
    print("=" * 60)
    print("MISC VARIABLES (80 bytes at offset {:#06x})".format(offset))
    print("  *** WARNING: field names below assume source declaration")
    print("      order matches BSS layout.  Disassembly shows it does")
    print("      not — only map_x at offset 0 is confirmed.  See")
    print("      reference/PROBLEMS.md §P21. ***")
    print("=" * 60)
    if show_hex:
        print(hexdump(data, offset, 80))
        print()

    fields = [
        # (name, type, description)
        ("map_x", "U", "Map X (pixels)"),
        ("map_y", "U", "Map Y (pixels)"),
        ("hero_x", "U", "Hero X"),
        ("hero_y", "U", "Hero Y"),
        ("safe_x", "U", "Safe zone X"),
        ("safe_y", "U", "Safe zone Y"),
        ("safe_r", "U", "Safe zone region"),
        ("img_x", "U", "Sector X"),
        ("img_y", "U", "Sector Y"),
        # /* 18 */
        ("cheat1", "S", "Cheat flag"),
        ("riding", "S", "Riding"),
        ("flying", "S", "Flying"),
        ("wcarry", "S", "Carry mode"),
        ("turtleprox", "S", "Turtle proximity"),
        ("raftprox", "S", "Raft proximity"),
        ("brave", "S", "Bravery"),
        ("luck", "S", "Luck"),
        ("kind", "S", "Kindness"),
        ("wealth", "S", "Wealth"),
        ("hunger", "S", "Hunger"),
        ("fatigue", "S", "Fatigue"),
        # /* 24 */
        ("brother", "S", "Active brother (1=Julian, 2=Phillip, 3=Kevin)"),
        ("princess", "S", "Princess rescued"),
        ("hero_sector", "S", "Hero sector"),
        ("hero_place", "U", "Hero place"),
        # /* 8 */
        ("daynight", "U", "Day/night counter"),
        ("lightlevel", "U", "Light level"),
        ("actor_file", "S", "Actor file loaded"),
        ("set_file", "S", "Set file loaded"),
        ("active_carrier", "S", "Active carrier"),
        ("xtype", "U", "Extent type"),
        ("leader", "S", "Enemy leader"),
        ("secret_timer", "S", "Secret timer"),
        ("light_timer", "S", "Light timer"),
        ("freeze_timer", "S", "Freeze timer"),
        ("cmode", "S", "Combat mode"),
        ("encounter_type", "U", "Encounter type"),
        # pads to fill 80 bytes
        ("pad1", "U", "Pad 1"),
        ("pad2", "U", "Pad 2"),
        ("pad3", "U", "Pad 3"),
    ]

    pos = offset
    for name, typ, desc in fields:
        if typ == "U":
            val = u16(data, pos)
        else:
            val = s16(data, pos)

        extra = ""
        if name == "hero_place":
            extra = f"  [{lookup(PLACE_NAMES, val, '?')}]"
        elif name == "brother":
            bname = BROTHER_NAMES[val - 1] if 1 <= val <= 3 else "?"
            extra = f"  [{bname}]"
        elif name == "cheat1":
            extra = "  [ENABLED]" if val else "  [disabled]"

        print(f"  {desc:30s} ({name}): {val}{extra}")
        pos += 2

    return offset + 80


def decode_region(data, offset, show_hex):
    """Decode region_num. fmain.c:617"""
    print()
    print("=" * 60)
    print("REGION (2 bytes at offset {:#06x})".format(offset))
    print("=" * 60)
    if show_hex:
        print(hexdump(data, offset, 2))
    val = u16(data, offset)
    print(f"  region_num: {lookup(REGION_NAMES, val, '?')}")
    return offset + 2


def decode_anim_header(data, offset, show_hex):
    """Decode anix, anix2, mdex (6 bytes). fmain.c:75-76"""
    print()
    print("=" * 60)
    print("ANIM HEADER (6 bytes at offset {:#06x})".format(offset))
    print("=" * 60)
    if show_hex:
        print(hexdump(data, offset, 6))
    anix = s16(data, offset)
    anix2 = s16(data, offset + 2)
    mdex = s16(data, offset + 4)
    print(f"  anix  (actor count): {anix}")
    print(f"  anix2 (alloc index): {anix2}")
    print(f"  mdex  (missile idx): {mdex}")
    return offset + 6, anix


def decode_anim_list(data, offset, count, show_hex):
    """Decode anim_list entries. struct shape — ftale.h:56-71"""
    size = count * SIZEOF_SHAPE
    print()
    print("=" * 60)
    print(f"ANIM LIST ({count} entries, {size} bytes at offset {offset:#06x})")
    print("=" * 60)

    for i in range(count):
        pos = offset + i * SIZEOF_SHAPE
        if show_hex:
            print(hexdump(data, pos, SIZEOF_SHAPE))

        abs_x = u16(data, pos)
        abs_y = u16(data, pos + 2)
        rel_x = u16(data, pos + 4)
        rel_y = u16(data, pos + 6)
        atype = s8(data, pos + 8)
        race = u8(data, pos + 9)
        index = s8(data, pos + 10)
        visible = s8(data, pos + 11)
        weapon = s8(data, pos + 12)
        environ = s8(data, pos + 13)
        goal = s8(data, pos + 14)
        tactic = s8(data, pos + 15)
        state = s8(data, pos + 16)
        facing = s8(data, pos + 17)
        vitality = s16(data, pos + 18)
        vel_x = s8(data, pos + 20)
        vel_y = s8(data, pos + 21)

        label = "HERO" if i == 0 else f"Actor {i}"
        print(f"  [{i}] {label}:")
        print(f"      pos=({abs_x},{abs_y}) rel=({rel_x},{rel_y})")
        print(f"      type={atype} race={race} index={index} visible={visible}")
        print(f"      weapon={lookup(WEAPON_TYPES, weapon, '?')}")
        print(f"      environ={environ}")
        print(f"      goal={lookup(GOAL_MODES, goal, '?')}")
        print(f"      tactic={tactic}")
        print(f"      state={lookup(MOTION_STATES, state, '?')}")
        print(f"      facing={lookup(DIRECTIONS, facing, '?')}")
        print(f"      vitality={vitality}")
        print(f"      velocity=({vel_x},{vel_y})")

    return offset + size


def decode_inventory(data, offset, label, show_hex):
    """Decode a 35-byte inventory array (stuff[]). fmain.c:428-432"""
    print()
    print(f"  --- {label} Inventory (35 bytes at offset {offset:#06x}) ---")
    if show_hex:
        print(hexdump(data, offset, 35))

    has_items = False
    for i in range(35):
        val = u8(data, offset + i)
        if val > 0:
            name = ITEM_NAMES[i] if i < len(ITEM_NAMES) else f"item[{i}]"
            print(f"    [{i:2d}] {name:20s}: {val}")
            has_items = True
    if not has_items:
        print("    (empty)")

    return offset + 35


def decode_mod1save(data, offset, show_hex):
    """Decode mod1save block: 3x35 inventory + 6 missiles. fmain.c:3621-3631"""
    print()
    print("=" * 60)
    print("BROTHER INVENTORIES + MISSILES (offset {:#06x})".format(offset))
    print("=" * 60)

    for i, name in enumerate(BROTHER_NAMES):
        offset = decode_inventory(data, offset, name, show_hex)

    # Missile list: 6 entries
    missile_size = 6 * SIZEOF_MISSILE
    print()
    print(f"  --- Missiles (6 entries, {missile_size} bytes at offset {offset:#06x}) ---")

    for i in range(6):
        pos = offset + i * SIZEOF_MISSILE
        if show_hex:
            print(hexdump(data, pos, SIZEOF_MISSILE))

        abs_x = u16(data, pos)
        abs_y = u16(data, pos + 2)
        mtype = s8(data, pos + 4)
        tof = s8(data, pos + 5)
        speed = s8(data, pos + 6)
        direction = s8(data, pos + 7)
        archer = s8(data, pos + 8)

        if mtype == 0 and abs_x == 0 and abs_y == 0:
            continue  # empty slot
        print(f"    [{i}] pos=({abs_x},{abs_y}) type={mtype} "
              f"tof={tof} speed={speed} "
              f"dir={lookup(DIRECTIONS, direction, '?')} archer={archer}")

    return offset + missile_size


def decode_extents(data, offset, show_hex):
    """Decode extent_list[2]. fmain.c:335-338"""
    size = 2 * SIZEOF_EXTENT
    print()
    print("=" * 60)
    print(f"EXTENT LIST (2 entries, {size} bytes at offset {offset:#06x})")
    print("=" * 60)

    for i in range(2):
        pos = offset + i * SIZEOF_EXTENT
        if show_hex:
            print(hexdump(data, pos, SIZEOF_EXTENT))

        x1 = u16(data, pos)
        y1 = u16(data, pos + 2)
        x2 = u16(data, pos + 4)
        y2 = u16(data, pos + 6)
        etype = u8(data, pos + 8)
        v1 = u8(data, pos + 9)
        v2 = u8(data, pos + 10)
        v3 = u8(data, pos + 11)

        print(f"  [{i}] rect=({x1},{y1})-({x2},{y2}) "
              f"etype={etype} v1={v1} v2={v2} v3={v3}")

    return offset + size


def decode_objects(data, offset, count, label, show_hex):
    """Decode an object list. struct object — ftale.h:90-93"""
    size = count * SIZEOF_OBJECT
    print()
    print(f"  --- {label} ({count} objects, {size} bytes at offset {offset:#06x}) ---")

    for i in range(count):
        pos = offset + i * SIZEOF_OBJECT
        xc = u16(data, pos)
        yc = u16(data, pos + 2)
        ob_id = s8(data, pos + 4)
        ob_stat = s8(data, pos + 5)

        if show_hex:
            print(hexdump(data, pos, SIZEOF_OBJECT))
        print(f"    [{i:2d}] pos=({xc},{yc}) id={ob_id} stat={ob_stat}")

    return offset + size


def decode_global_objects(data, offset, show_hex):
    """Decode ob_listg (global objects). fmain2.c:1180,1522"""
    print()
    print("=" * 60)
    print(f"GLOBAL OBJECTS (ob_listg, {GLBOBS} entries at offset {offset:#06x})")
    print("=" * 60)
    return decode_objects(data, offset, GLBOBS, "ob_listg", show_hex)


def decode_region_objects(data, offset, show_hex):
    """Decode mapobs, dstobs, and per-region object tables. fmain2.c:1178-1179"""
    print()
    print("=" * 60)
    print(f"REGION OBJECT TABLES (offset {offset:#06x})")
    print("=" * 60)

    # mapobs[10] — 10 shorts
    if show_hex:
        print(hexdump(data, offset, 20, "  mapobs: "))
    mapobs = []
    for i in range(10):
        mapobs.append(s16(data, offset + i * 2))
    print(f"  mapobs (obj counts): {mapobs}")
    offset += 20

    # dstobs[10] — 10 shorts
    if show_hex:
        print(hexdump(data, offset, 20, "  dstobs: "))
    dstobs = []
    for i in range(10):
        dstobs.append(s16(data, offset + i * 2))
    print(f"  dstobs (distributed): {dstobs}")
    offset += 20

    # Per-region object lists
    for i in range(10):
        count = mapobs[i]
        rname = REGION_NAMES.get(i, f"Region {i}")
        if count > 0:
            offset = decode_objects(data, offset, count,
                                    f"Region {i} ({rname})", show_hex)
        else:
            print(f"\n  --- Region {i} ({rname}): 0 objects ---")

    return offset


# -- Main --------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="Decode and dump a Faery Tale Adventure savegame file.",
        epilog="Savegame format from fmain2.c:1508-1527. "
               "All values are big-endian (68000).",
    )
    parser.add_argument("savefile", help="Path to savegame file")
    parser.add_argument("--hex", action="store_true",
                        help="Show hex dumps alongside decoded values")
    parser.add_argument("--section", choices=[
        "misc", "region", "anim", "inventory", "missiles",
        "extents", "objects", "all"
    ], default="all", help="Decode only a specific section (default: all)")
    args = parser.parse_args()

    path = Path(args.savefile)
    if not path.exists():
        print(f"Error: file not found: {path}", file=sys.stderr)
        sys.exit(1)

    data = path.read_bytes()
    print(f"File: {path}")
    print(f"Size: {len(data)} bytes")

    show_all = args.section == "all"
    offset = 0

    # 1. Misc variables (80 bytes)
    if show_all or args.section == "misc":
        offset = decode_misc(data, offset, args.hex)
    else:
        offset += 80

    # 2. Region (2 bytes)
    if show_all or args.section == "region":
        offset = decode_region(data, offset, args.hex)
    else:
        offset += 2

    # 3. Anim header (6 bytes) + anim list (variable)
    offset, anix = decode_anim_header(data, offset, args.hex)
    if show_all or args.section == "anim":
        offset = decode_anim_list(data, offset, anix, args.hex)
    else:
        offset += anix * SIZEOF_SHAPE

    # 4. mod1save: inventories + missiles
    if show_all or args.section == "inventory" or args.section == "missiles":
        offset = decode_mod1save(data, offset, args.hex)
    else:
        offset += 35 * 3 + 6 * SIZEOF_MISSILE

    # 5. Extents (2 * 12 bytes)
    if show_all or args.section == "extents":
        offset = decode_extents(data, offset, args.hex)
    else:
        offset += 2 * SIZEOF_EXTENT

    # 6. Global objects + region objects
    if show_all or args.section == "objects":
        offset = decode_global_objects(data, offset, args.hex)
        offset = decode_region_objects(data, offset, args.hex)

    print()
    print(f"Decoded {offset} of {len(data)} bytes.")
    if offset != len(data):
        remaining = len(data) - offset
        print(f"  ({remaining} bytes remaining — possible trailing data or "
              f"size mismatch)")


if __name__ == "__main__":
    main()
