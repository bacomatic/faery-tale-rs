#!/usr/bin/env python3
"""Decode and navigate world map data from the game's binary image file.

Parses the region maps, sector data, and terrain attributes from game/image
to provide spatial context for research. Understands the file_index table
from fmain.c to locate data for each of the 10 world regions.

Usage:
    python tools/decode_map_data.py --terrain-summary
    python tools/decode_map_data.py --terrain-types 8 9
    python tools/decode_map_data.py --region-map 5
    python tools/decode_map_data.py --find-terrain-type 12
    python tools/decode_map_data.py --sector-detail 181
"""

import argparse
import os
import struct
import sys
from collections import Counter, defaultdict
from datetime import date

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
IMAGE_PATH = os.path.join(REPO_ROOT, 'game', 'image')

BLOCK_SIZE = 512

# file_index[10] from fmain.c:615-626
# struct need { USHORT image[4], terra1, terra2, sector, region, setchar; }
FILE_INDEX = [
    # (images[4], terra1, terra2, sector, region, name)
    ((320, 480, 520, 560), 0, 1, 32, 160, 'F1 - snowy region'),
    ((320, 360, 400, 440), 2, 3, 32, 160, 'F2 - witch wood'),
    ((320, 360, 520, 560), 2, 1, 32, 168, 'F3 - swampy region'),
    ((320, 360, 400, 440), 2, 3, 32, 168, 'F4 - plains and rocks'),
    ((320, 480, 520, 600), 0, 4, 32, 176, 'F5 - desert area'),
    ((320, 280, 240, 200), 5, 6, 32, 176, 'F6 - bay / city / farms'),
    ((320, 640, 520, 600), 7, 4, 32, 184, 'F7 - volcanic'),
    ((320, 280, 240, 200), 5, 6, 32, 184, 'F8 - forest and wilderness'),
    ((680, 720, 800, 840), 8, 9, 96, 192, 'F9 - inside of buildings'),
    ((680, 760, 800, 840), 10, 9, 96, 192, 'F10 - dungeons and caves'),
]

# TERRA_BLOCK = 149 (fmain.c:608)
TERRA_BLOCK = 149

# Terrain set names from terrain.c
TERRAIN_SET_NAMES = {
    0: 'wild+palace',        # order[0,1]  = wild(1), palace(9)
    1: 'swamp+mountain2',    # order[2,3]  = swamp(8), mountain2(10)
    2: 'wild+build',         # order[4,5]  = wild(1), build(2)
    3: 'rock+tower',         # order[6,7]  = rock(3), tower(5)
    4: 'swamp+mountain3',    # order[8,9]  = swamp(8), mountain3(12)
    5: 'wild+castle',        # order[10,11]= wild(1), castle(6)
    6: 'field+mountain1',    # order[12,13]= field(7), mountain1(4)
    7: 'wild+doom',          # order[14,15]= wild(1), doom(11)
    8: 'under+furnish',      # order[16,17]= under(13), furnish(15)
    9: 'inside+astral',      # order[18,19]= inside(16), astral(17)
    10: 'under+cave',        # order[20,21]= under(13), cave(14)
}

# Terrain type meanings from prox (fsubs.asm:1588-1613) and fmain.c:685
TERRAIN_TYPE_INFO = {
    0: 'passable',
    1: 'impassable (wall)',
    2: 'sink (water)',
    3: 'slow (brush)',
    4: 'hazard',
    8: 'passable-for-hero (proxcheck special)',
    9: 'passable-for-hero (proxcheck special)',
    12: 'BLOCKED unless stuff[30] (Shard) — fmain.c:1609',
    15: 'door trigger — fmain.c:1607',
}

# Sector data layout:
# sector_mem loaded at nd->sector, 64 blocks = 32768 bytes
# 256 sectors × 128 bytes each
# Each sector is 16×8 tile grid (128 tiles)
SECTOR_SIZE = 128
SECTORS_PER_LOAD = 256

# Region map: 8 blocks = 4096 bytes
# map_mem = sector_mem + SECTOR_OFF (128*256 = 32768)
# Region map is separate: 64×64 grid mapping to sector numbers
REGION_MAP_SIZE = 4096

# Terra data: 1 block = 512 bytes per terrain set pair
# Layout: 128 entries of 4 bytes each (maptag, terrain, tiles, big_colors)
# Two sets per block: first 64 entries = terra1, next 64 = terra2
TERRA_ENTRIES = 128
TERRA_ENTRY_SIZE = 4

# pagecolors[32] from fmain2.c:367-372, converted from 12-bit Amiga RGB
# (0x0RGB, 4 bits per channel) to 24-bit RGB (each nibble * 17).
AMIGA_PALETTE = [
    (  0,   0,   0), (255, 255, 255), (238, 153, 102), (187, 102,  51),
    (102,  51,  17), (119, 187, 255), ( 51,  51,  51), (221, 187, 136),
    ( 34,  34,  51), ( 68,  68,  85), (136, 136, 153), (187, 187, 204),
    ( 85,  34,  17), (153,  68,  17), (255, 136,  34), (255, 204, 119),
    (  0,  68,   0), (  0, 119,   0), (  0, 187,   0), (102, 255, 102),
    (  0,   0,  85), (  0,   0, 153), (  0,   0, 221), ( 51, 119, 255),
    (204,   0,   0), (255,  85,   0), (255, 170,   0), (255, 255, 102),
    (238, 187, 102), (238, 170,  85), (  0,   0, 255), (187, 221, 255),
]

# ---------------------------------------------------------------------------
# Static game data for cross-referencing (source code tables)
# ---------------------------------------------------------------------------

# NPC setfig names by ob_id (when ob_stat is 3 or 4) — fmain.c:22-36
SETFIG_NAMES = {
    0: 'wizard', 1: 'priest', 2: 'guard', 3: 'guard (back)',
    4: 'princess', 5: 'king', 6: 'noble', 7: 'sorceress',
    8: 'bartender', 9: 'witch', 10: 'spectre', 11: 'ghost',
    12: 'ranger', 13: 'beggar',
}

# Item names by ob_id (when ob_stat is 0, 1, 2, 5, or 6) — fmain2.c:967-977
ITEM_NAMES = {
    0: 'special item', 11: 'quiver', 13: 'money (50gp)', 14: 'urn',
    15: 'chest', 16: 'sacks', 17: 'gold ring', 18: 'blue stone',
    19: 'green jewel', 20: 'scrap of paper', 21: 'crystal orb',
    22: 'vial', 23: 'bird totem', 24: 'jade skull', 25: 'gold key',
    26: 'grey key', 27: 'lasso', 28: 'dead brother bones',
    29: 'opened chest', 31: 'footstool', 102: 'turtle eggs',
    114: 'blue key', 138: "king's bone", 145: 'magic wand',
    146: 'meal', 147: 'rose', 148: 'fruit', 149: 'gold statue',
    150: 'book', 151: 'shell', 153: 'green key', 154: 'white key',
    155: 'sunstone', 242: 'red key',
}

OB_STAT_NAMES = {
    0: 'disabled', 1: 'on ground', 2: 'in inventory',
    3: 'NPC (setfig)', 4: 'dead NPC', 5: 'hidden (Look)', 6: 'cabinet',
}

# Door type constants — fmain.c:210-229
DOOR_TYPE_NAMES = {
    1: 'HWOOD', 2: 'VWOOD', 3: 'HSTONE', 4: 'VSTONE',
    5: 'HCITY', 6: 'VCITY', 7: 'CRYST', 8: 'SECRET',
    9: 'BLACK', 10: 'MARBLE', 11: 'LOG', 13: 'HSTON2',
    14: 'VSTON2', 15: 'STAIR', 17: 'DESERT', 18: 'CAVE/VLOG',
}

# doorlist[86] — fmain.c:240-325
# (xc1, yc1, xc2, yc2, type, secs, comment)
DOORLIST = [
    (0x1170, 0x5060, 0x2870, 0x8b60, 1, 1, 'desert fort'),
    (0x1170, 0x5060, 0x2870, 0x8b60, 1, 1, 'desert fort'),
    (0x1170, 0x5060, 0x2870, 0x8b60, 1, 1, 'desert fort'),
    (0x1170, 0x5060, 0x2870, 0x8b60, 1, 1, 'desert fort'),
    (0x1390, 0x1b60, 0x1980, 0x8c60, 18, 2, 'dragon cave'),
    (0x1770, 0x6aa0, 0x2270, 0x96a0, 9, 1, 'pass fort'),
    (0x1970, 0x62a0, 0x1f70, 0x96a0, 9, 1, 'gate fort'),
    (0x1aa0, 0x4ba0, 0x13a0, 0x95a0, 17, 1, 'oasis #1'),
    (0x1aa0, 0x4c60, 0x13a0, 0x9760, 17, 1, 'oasis #4'),
    (0x1b20, 0x4b60, 0x1720, 0x9560, 17, 1, 'oasis #2'),
    (0x1b80, 0x4b80, 0x1580, 0x9580, 17, 1, 'oasis #3'),
    (0x1b80, 0x4c40, 0x1580, 0x9740, 17, 1, 'oasis #5'),
    (0x1e70, 0x3b60, 0x2880, 0x9c60, 3, 1, 'west keep'),
    (0x2480, 0x33a0, 0x2e80, 0x8da0, 1, 1, 'swamp shack'),
    (0x2960, 0x8760, 0x2b00, 0x92c0, 15, 1, 'stargate forwards'),
    (0x2b00, 0x92c0, 0x2960, 0x8780, 15, 2, 'stargate backwards'),
    (0x2c00, 0x7160, 0x2af0, 0x9360, 9, 1, 'doom tower'),
    (0x2f70, 0x2e60, 0x3180, 0x9a60, 3, 1, 'lakeside keep'),
    (0x2f70, 0x63a0, 0x1c70, 0x96a0, 9, 1, 'plain fort'),
    (0x3180, 0x38c0, 0x2780, 0x98c0, 1, 1, "road's end inn"),
    (0x3470, 0x4b60, 0x0470, 0x8ee0, 15, 2, 'tombs'),
    (0x3DE0, 0x1BC0, 0x2EE0, 0x93C0, 7, 1, 'crystal palace'),
    (0x3E00, 0x1BC0, 0x2F00, 0x93C0, 7, 1, 'crystal palace'),
    (0x4270, 0x2560, 0x2e80, 0x9a60, 3, 1, 'coast keep'),
    (0x4280, 0x3bc0, 0x2980, 0x98c0, 1, 1, 'friendly inn'),
    (0x45e0, 0x5380, 0x25d0, 0x9680, 10, 1, 'mountain keep'),
    (0x4780, 0x2fc0, 0x2580, 0x98c0, 1, 1, 'forest inn'),
    (0x4860, 0x6640, 0x1c60, 0x9a40, 18, 1, 'cabin yard #7'),
    (0x4890, 0x66a0, 0x1c90, 0x9aa0, 11, 1, 'cabin #7'),
    (0x4960, 0x5b40, 0x2260, 0x9a40, 18, 1, 'cabin yard #6'),
    (0x4990, 0x5ba0, 0x2290, 0x9aa0, 11, 1, 'cabin #6'),
    (0x49a0, 0x3cc0, 0x0ba0, 0x82c0, 2, 1, 'village #2'),
    (0x49d0, 0x3dc0, 0x0bd0, 0x84c0, 2, 1, 'village #1.a'),
    (0x49d0, 0x3e00, 0x0bd0, 0x8500, 2, 1, 'village #1.b'),
    (0x4a10, 0x3c80, 0x0d10, 0x8280, 1, 1, 'village #3'),
    (0x4a10, 0x3d40, 0x0f10, 0x8340, 1, 1, 'village #5'),
    (0x4a30, 0x3dc0, 0x0e30, 0x85c0, 1, 1, 'village #7'),
    (0x4a60, 0x3e80, 0x1060, 0x8580, 1, 1, 'village #8'),
    (0x4a70, 0x3c80, 0x1370, 0x8280, 1, 1, 'village #4'),
    (0x4a80, 0x3d40, 0x1190, 0x8340, 1, 1, 'village #6'),
    (0x4c70, 0x3260, 0x2580, 0x9c60, 3, 1, 'crag keep'),
    (0x4d60, 0x5440, 0x1f60, 0x9c40, 18, 1, 'cabin #2'),
    (0x4d90, 0x4380, 0x3080, 0x8d80, 13, 1, 'crypt'),
    (0x4d90, 0x54a0, 0x1f90, 0x9ca0, 11, 1, 'cabin yard #2'),
    (0x4de0, 0x6b80, 0x29d0, 0x9680, 10, 1, 'river keep'),
    (0x5360, 0x5840, 0x2260, 0x9840, 18, 1, 'cabin yard #3'),
    (0x5390, 0x58a0, 0x2290, 0x98a0, 11, 1, 'cabin #3'),
    (0x5460, 0x4540, 0x1c60, 0x9840, 18, 1, 'cabin yard #1'),
    (0x5470, 0x6480, 0x2c80, 0x8d80, 3, 1, 'elf glade'),
    (0x5490, 0x45a0, 0x1c90, 0x98a0, 11, 1, 'cabin #1'),
    (0x55f0, 0x52e0, 0x16e0, 0x83e0, 10, 1, 'main castle'),
    (0x56c0, 0x53c0, 0x1bc0, 0x84c0, 13, 1, 'city #15.a'),
    (0x56c0, 0x5440, 0x19c0, 0x8540, 13, 1, 'city #17'),
    (0x56f0, 0x51a0, 0x19f0, 0x82a0, 13, 1, 'city #10'),
    (0x5700, 0x5240, 0x1df0, 0x8340, 14, 1, 'city #12'),
    (0x5710, 0x5440, 0x1c10, 0x8640, 13, 1, 'city #18'),
    (0x5730, 0x5300, 0x1a50, 0x8400, 13, 1, 'city #14'),
    (0x5730, 0x5380, 0x1c30, 0x8480, 14, 1, 'city #15.b'),
    (0x5750, 0x51a0, 0x1c60, 0x82a0, 13, 1, 'city #11'),
    (0x5750, 0x5260, 0x2050, 0x8360, 13, 1, 'city #13'),
    (0x5760, 0x53c0, 0x2060, 0x84c0, 13, 1, 'city #16'),
    (0x5760, 0x5440, 0x1e60, 0x8540, 13, 1, 'city #19'),
    (0x5860, 0x5d40, 0x1c60, 0x9a40, 18, 1, 'cabin yard #4'),
    (0x5890, 0x5da0, 0x1c90, 0x9ca0, 11, 1, 'cabin #4'),
    (0x58c0, 0x2e60, 0x0ac0, 0x8860, 18, 2, 'troll cave'),
    (0x5960, 0x6f40, 0x2260, 0x9a40, 18, 1, 'cabin yard #9'),
    (0x5990, 0x6fa0, 0x2290, 0x9ca0, 11, 1, 'cabin #9'),
    (0x59a0, 0x6760, 0x2aa0, 0x8b60, 15, 1, 'unreachable castle'),
    (0x59e0, 0x5880, 0x27d0, 0x9680, 10, 1, 'farm keep'),
    (0x5e70, 0x1a60, 0x2580, 0x9a60, 3, 1, 'north keep'),
    (0x5ec0, 0x2960, 0x11c0, 0x8b60, 18, 2, 'spider exit'),
    (0x6060, 0x7240, 0x1960, 0x9c40, 18, 1, 'cabin yard #10'),
    (0x6090, 0x72a0, 0x1990, 0x9ca0, 11, 1, 'cabin #10'),
    (0x60f0, 0x32c0, 0x25f0, 0x8bc0, 3, 1, 'mammoth manor'),
    (0x64c0, 0x1860, 0x03c0, 0x8660, 18, 2, 'maze cave 2'),
    (0x6560, 0x5d40, 0x1f60, 0x9a40, 18, 1, 'cabin yard #5'),
    (0x6590, 0x5da0, 0x1f90, 0x98a0, 11, 1, 'cabin #5'),
    (0x65c0, 0x1a20, 0x04b0, 0x8840, 9, 2, 'maze cave 1'),
    (0x6670, 0x2a60, 0x2b80, 0x9a60, 3, 1, 'glade keep'),
    (0x6800, 0x1b60, 0x2af0, 0x9060, 9, 1, "witch's castle"),
    (0x6b50, 0x4380, 0x2850, 0x8d80, 13, 1, 'lighthouse'),
    (0x6be0, 0x7c80, 0x2bd0, 0x9680, 10, 1, 'lonely keep'),
    (0x6c70, 0x2e60, 0x2880, 0x9a60, 3, 1, 'sea keep'),
    (0x6d60, 0x6840, 0x1f60, 0x9a40, 18, 1, 'cabin yard #8'),
    (0x6d90, 0x68a0, 0x1f90, 0x9aa0, 11, 1, 'cabin #8'),
    (0x6ee0, 0x5280, 0x31d0, 0x9680, 10, 1, 'point keep'),
]

# extent_list[23] — fmain.c:338-371
# (x1, y1, x2, y2, etype, v1, v2, v3, comment)
EXTENT_LIST = [
    (2118, 27237, 2618, 27637, 70, 0, 1, 11, 'bird extent'),
    (0, 0, 0, 0, 70, 0, 1, 5, 'turtle extent (dynamic)'),
    (6749, 34951, 7249, 35351, 70, 0, 1, 10, 'dragon extent'),
    (4063, 34819, 4909, 35125, 53, 4, 1, 6, 'spider pit'),
    (9563, 33883, 10144, 34462, 60, 1, 1, 9, 'necromancer'),
    (22945, 5597, 23225, 5747, 61, 3, 2, 4, 'turtle eggs'),
    (10820, 35646, 10877, 35670, 83, 1, 1, 0, 'princess extent'),
    (19596, 17123, 19974, 17401, 48, 8, 8, 2, 'graveyard'),
    (19400, 17034, 20240, 17484, 80, 4, 20, 0, 'around city (peace)'),
    (0x2400, 0x8200, 0x3100, 0x8a00, 52, 3, 1, 8, 'astral plane'),
    (5272, 33300, 6112, 34200, 81, 0, 1, 0, 'king domain (peace)'),
    (11712, 37350, 12416, 38020, 82, 0, 1, 0, 'sorceress domain (peace)'),
    (2752, 33300, 8632, 35400, 80, 0, 1, 0, 'peace 1 - buildings'),
    (10032, 35550, 12976, 40270, 80, 0, 1, 0, 'peace 2 - specials'),
    (4712, 38100, 10032, 40350, 80, 0, 1, 0, 'peace 3 - cabins'),
    (21405, 25583, 21827, 26028, 60, 1, 1, 7, 'hidden valley (DKnight)'),
    (6156, 12755, 12316, 15905, 7, 1, 8, 0, 'swamp encounters'),
    (5140, 34860, 6260, 37260, 8, 1, 8, 0, 'spider region'),
    (660, 33510, 2060, 34560, 8, 1, 8, 0, 'spider region 2'),
    (18687, 15338, 19211, 16136, 80, 0, 1, 0, 'village (peace)'),
    (16953, 18719, 20240, 17484, 3, 1, 3, 0, 'around village'),
    (20593, 18719, 23113, 22769, 3, 1, 3, 0, 'around city'),
    (0, 0, 0x7fff, 0x9fff, 3, 1, 8, 0, 'whole world (fallback)'),
]

EXTENT_TYPE_NAMES = {
    3: 'default encounters', 7: 'swamp encounters', 8: 'spider encounters',
    48: 'graveyard encounters', 52: 'forced group spawn',
    53: 'forced group (spiders)', 60: 'special figure',
    61: 'special figure', 70: 'carrier spawn', 80: 'peace zone',
    81: 'king domain (peace)', 82: 'sorceress domain (peace)',
    83: 'princess rescue trigger',
}

# Object lists — fmain2.c:1001-1178
# (xc, yc, ob_id, ob_stat, comment)
OB_LISTG = [
    (0, 0, 0, 0, 'special item (for give)'),
    (0, 0, 28, 0, 'dead brother 1'), (0, 0, 28, 0, 'dead brother 2'),
    (19316, 15747, 11, 0, 'ghost brother 1'),
    (18196, 15735, 11, 0, 'ghost brother 2'),
    (12439, 36202, 10, 3, 'spectre'),
    (11092, 38526, 149, 1, 'gold statue (seahold)'),
    (25737, 10662, 149, 1, 'gold statue (ogre den)'),
    (2910, 39023, 149, 1, 'gold statue (octal room)'),
    (12025, 37639, 149, 0, 'gold statue (sorceress, hidden)'),
    (6700, 33766, 149, 0, 'gold statue (priest, hidden)'),
]
OB_LIST0 = [
    (3340, 6735, 12, 3, 'ranger west'), (9678, 7035, 12, 3, 'ranger east'),
    (4981, 6306, 12, 3, 'ranger north'),
]
OB_LIST1 = [(23087, 5667, 102, 1, 'turtle eggs')]
OB_LIST2 = [
    (13668, 15000, 0, 3, 'wizard'), (10627, 13154, 0, 3, 'wizard'),
    (4981, 10056, 12, 3, 'ranger'), (13950, 11087, 16, 1, 'sacks'),
    (10344, 36171, 151, 1, 'shell'),
]
OB_LIST3 = [
    (19298, 16128, 15, 1, 'chest'), (18310, 15969, 13, 3, 'beggar'),
    (20033, 14401, 0, 3, 'wizard'), (24794, 13102, 13, 3, 'beggar'),
    (21626, 15446, 18, 1, 'blue stone (stone ring)'),
    (21616, 15456, 13, 1, 'money (stone ring)'),
    (21636, 15456, 17, 1, 'gold ring (stone ring)'),
    (20117, 14222, 19, 1, 'green jewel'),
    (24185, 9840, 16, 1, 'sacks'), (25769, 10617, 13, 1, 'money'),
    (25678, 10703, 18, 1, 'blue stone'), (17177, 10599, 20, 1, 'scrap of paper'),
]
OB_LIST4 = [(0, 0, 0, 0, 'dummy'), (0, 0, 0, 0, 'dummy'),
             (6817, 19693, 13, 3, 'beggar')]
OB_LIST5 = [
    (22184, 21156, 13, 3, 'beggar'), (18734, 17595, 17, 1, 'gold ring'),
    (21294, 22648, 15, 1, 'chest'), (22956, 19955, 0, 3, 'wizard'),
    (28342, 22613, 0, 3, 'wizard'),
]
OB_LIST6 = [(24794, 13102, 13, 3, 'DUMMY')]
OB_LIST7 = [(23297, 5797, 102, 1, 'DUMMY')]
OB_LIST8 = [
    (6700, 33756, 1, 3, 'priest in chapel'),
    (5491, 33780, 5, 3, 'king on throne'),
    (5592, 33764, 6, 3, 'noble'),
    (5514, 33668, 2, 3, 'guard'), (5574, 33668, 2, 3, 'guard'),
    (8878, 38995, 0, 3, 'wizard'), (7776, 34084, 0, 3, 'wizard'),
    (5514, 33881, 3, 3, 'guard (back)'), (5574, 33881, 3, 3, 'guard (back)'),
    (10853, 35656, 4, 3, 'princess'),
    (12037, 37614, 7, 3, 'sorceress'),
    (11013, 36804, 9, 3, 'witch'),
    (9631, 38953, 8, 3, 'bartender'), (10191, 38953, 8, 3, 'bartender'),
    (10649, 38953, 8, 3, 'bartender'), (2966, 33964, 8, 3, 'bartender'),
    (9532, 40002, 31, 1, 'footstool'), (6747, 33751, 31, 1, 'footstool'),
    (11410, 36169, 155, 1, 'sunstone'),
    (9550, 39964, 23, 1, 'bird totem (cabinet)'),
    (9552, 39964, 23, 1, 'bird totem'),
    (9682, 39964, 23, 1, 'bird totem (cabinet)'),
    (9684, 39964, 23, 1, 'bird totem'),
    (9532, 40119, 23, 1, 'bird totem (table)'),
    (9575, 39459, 14, 1, 'urn'), (9590, 39459, 14, 1, 'urn'),
    (9605, 39459, 14, 1, 'urn'),
    (9680, 39453, 22, 1, 'vial'), (9682, 39453, 22, 1, 'vial'),
    (9784, 39453, 22, 1, 'vial'),
    (9668, 39554, 15, 1, 'chest'), (11090, 39462, 13, 1, 'money'),
    (11108, 39458, 23, 1, 'bird totem'), (11118, 39459, 23, 1, 'bird totem'),
    (11128, 39459, 23, 1, 'bird totem'), (11138, 39458, 23, 1, 'bird totem'),
    (11148, 39459, 23, 1, 'bird totem'), (11158, 39459, 23, 1, 'bird totem'),
    (11855, 36206, 31, 1, 'footstool'), (11909, 36198, 15, 1, 'chest'),
    (11918, 36246, 23, 1, 'bird totem (cabinet)'),
    (11928, 36246, 23, 1, 'bird totem'), (11938, 36246, 23, 1, 'bird totem'),
    (12212, 38481, 15, 1, 'chest'), (11652, 38481, 242, 1, 'red key'),
    (10427, 39977, 31, 1, 'footstool'), (10323, 40071, 14, 1, 'urn'),
    (10059, 38472, 16, 1, 'sacks'), (10344, 36171, 151, 1, 'shell'),
    (11936, 36207, 20, 1, 'scrap (spectre note)'),
    (9674, 35687, 14, 1, 'urn'), (5473, 38699, 147, 1, 'rose'),
    (7185, 34342, 148, 1, 'fruit'), (7190, 34342, 148, 1, 'fruit'),
    (7195, 34342, 148, 1, 'fruit'), (7185, 34347, 148, 1, 'fruit'),
    (7190, 34347, 148, 1, 'fruit'), (7195, 34347, 148, 1, 'fruit'),
    (6593, 34085, 148, 1, 'fruit'), (6598, 34085, 148, 1, 'fruit'),
    (6593, 34090, 148, 1, 'fruit'), (6598, 34090, 148, 1, 'fruit'),
    # 'look' items (ob_stat=5, hidden until Look)
    (3872, 33546, 25, 5, 'gold key (hidden)'),
    (3887, 33510, 23, 5, 'bird totem (hidden)'),
    (4495, 33510, 22, 5, 'vial (hidden)'),
    (3327, 33383, 24, 5, 'jade skull (hidden)'),
    (4221, 34119, 11, 5, 'quiver (hidden)'),
    (7610, 33604, 22, 5, 'vial (hidden)'),
    (7616, 33522, 13, 5, 'money (hidden)'),
    (9570, 35768, 18, 5, 'blue stone (hidden)'),
    (9668, 35769, 11, 5, 'quiver (hidden)'),
    (9553, 38951, 17, 5, 'gold ring (hidden)'),
    (10062, 39005, 24, 5, 'jade skull (hidden)'),
    (10577, 38951, 22, 5, 'vial (hidden)'),
    (11062, 39514, 13, 5, 'money (hidden)'),
    (8845, 39494, 154, 5, 'white key (hidden)'),
    (6542, 39494, 19, 5, 'green jewel (hidden)'),
    (7313, 38992, 242, 5, 'red key (hidden)'),
]
OB_LIST9 = [
    (7540, 38528, 145, 1, 'magic wand'), (9624, 36559, 145, 1, 'magic wand'),
    (9624, 37459, 145, 1, 'magic wand'), (8337, 36719, 145, 1, 'magic wand'),
    (8154, 34890, 15, 1, 'chest'), (7826, 35741, 15, 1, 'chest'),
    (3460, 37260, 0, 3, 'wizard'), (8485, 35725, 13, 1, 'money'),
    (3723, 39340, 138, 1, "king's bone"),
]

ALL_OBJECT_LISTS = {
    'global': OB_LISTG, '0': OB_LIST0, '1': OB_LIST1, '2': OB_LIST2,
    '3': OB_LIST3, '4': OB_LIST4, '5': OB_LIST5, '6': OB_LIST6,
    '7': OB_LIST7, '8': OB_LIST8, '9': OB_LIST9,
}

# Place name tables — narr.asm:86-223
# (sector_low, sector_high, message_index) — first match wins
PLACE_TBL = [
    (51, 51, 19), (64, 69, 2), (70, 73, 3), (80, 95, 6),
    (96, 99, 7), (138, 139, 8), (144, 144, 9), (147, 147, 10),
    (148, 148, 20), (159, 162, 17), (163, 163, 18), (164, 167, 12),
    (168, 168, 21), (170, 170, 22), (171, 174, 14), (176, 176, 13),
    (178, 178, 23), (179, 179, 24), (180, 180, 25), (175, 180, 0),
    (208, 221, 11), (243, 243, 16), (250, 252, 0), (255, 255, 26),
    (78, 78, 4), (187, 239, 4), (0, 79, 0), (185, 254, 15),
    (0, 255, 0),
]

INSIDE_TBL = [
    (2, 2, 2), (7, 7, 3), (4, 4, 4), (5, 6, 5), (9, 10, 6),
    (30, 30, 7), (19, 33, 14), (101, 101, 14), (130, 134, 14),
    (36, 36, 13), (37, 42, 12), (46, 46, 0), (43, 59, 11),
    (100, 100, 11), (143, 149, 11), (62, 62, 16), (65, 66, 18),
    (60, 78, 17), (82, 82, 17), (86, 87, 17), (92, 92, 17),
    (94, 95, 17), (97, 99, 17), (120, 120, 17), (116, 119, 17),
    (139, 141, 17), (79, 96, 9), (104, 104, 19), (114, 114, 20),
    (105, 115, 8), (135, 138, 8), (125, 125, 21), (127, 127, 10),
    (142, 142, 22), (121, 129, 22), (150, 161, 15), (0, 255, 0),
]

PLACE_MSG = [
    None, None,
    'village of Tambry', 'Vermillion Manor', 'Mountains of Frost',
    'Plain of Grief', 'city of Marheim', "Witch's castle", 'Graveyard',
    'great stone ring', 'watchtower', 'great Bog (swamp)',
    'Crystal Palace', 'Pixie Grove', 'Citadel of Doom',
    'Burning Waste (desert)', 'oasis', 'hidden city of Azal',
    'outlying fort', 'small keep', 'old castle', 'log cabin',
    'dark stone tower', 'isolated cabin', 'Tombs of Hemsath',
    'Forbidden Keep', 'dragon cave',
]

INSIDE_MSG = [
    None, None,
    'small chamber', 'large chamber', 'long passageway',
    'twisting tunnel', 'forked intersection', 'keep interior',
    'castle interior', 'castle of King Mar', 'sanctuary (temple)',
    'Spirit Plane', 'large room', 'octagonal room', 'stone corridor',
    'stone maze', 'small building', 'building', 'tavern', 'inn',
    'crypt (tomb)', 'cabin interior', 'unlocked/entered',
]


def read_blocks(f, block_offset, block_count):
    """Read block_count blocks starting at block_offset from the image file."""
    f.seek(block_offset * BLOCK_SIZE)
    return f.read(block_count * BLOCK_SIZE)


def decode_terra_block(data):
    """Decode terrain attributes from a 512-byte terra block.

    Returns list of (maptag, terrain_type, tiles_mask, big_colors) for 128 entries.
    terrain_type is the upper nibble of the terrain byte (what px_to_im returns).
    """
    entries = []
    for i in range(TERRA_ENTRIES):
        offset = i * TERRA_ENTRY_SIZE
        if offset + TERRA_ENTRY_SIZE > len(data):
            break
        maptag = data[offset]
        terrain_byte = data[offset + 1]
        tiles_mask = data[offset + 2]
        big_colors = data[offset + 3]
        terrain_type = (terrain_byte >> 4) & 0x0F
        terrain_sub = terrain_byte & 0x0F
        entries.append({
            'index': i,
            'maptag': maptag,
            'terrain_type': terrain_type,
            'terrain_sub': terrain_sub,
            'tiles_mask': tiles_mask,
            'big_colors': big_colors,
        })
    return entries


def decode_region_map(data):
    """Decode a region map (4096 bytes) into a grid of sector numbers.

    The region map is indexed by: secy * 128 + secx + xreg
    For outdoor regions (sector=32, region=160+), it's a 32×32 grid.
    For indoor regions (sector=96, region=192), layout differs.
    """
    return list(data[:REGION_MAP_SIZE])


def decode_sector(data, sector_num):
    """Decode a single sector (128 bytes = 16×8 tile grid)."""
    offset = sector_num * SECTOR_SIZE
    if offset + SECTOR_SIZE > len(data):
        return None
    tiles = []
    for row in range(8):
        row_tiles = []
        for col in range(16):
            idx = offset + row * 16 + col
            row_tiles.append(data[idx])
        tiles.append(row_tiles)
    return tiles


def cmd_terrain_summary(image_file):
    """Show terrain type distribution per region."""
    print('Terrain Type Summary by Region')
    print('=' * 70)
    print()

    with open(image_file, 'rb') as f:
        for region_idx, (images, t1, t2, sector_blk, region_blk, name) in enumerate(FILE_INDEX):
            print(f'Region {region_idx}: {name}')
            print(f'  Terra sets: {t1} ({TERRAIN_SET_NAMES.get(t1, "?")}), '
                  f'{t2} ({TERRAIN_SET_NAMES.get(t2, "?")})')

            # Each terra block covers 128 tile entries (512 bytes = 128 × 4).
            # terra1 block → tiles 0-127 in terra_mem[0..511]
            # terra2 block → tiles 128-255 in terra_mem[512..1023]
            for ti, terra_idx in enumerate([t1, t2]):
                terra_data = read_blocks(f, TERRA_BLOCK + terra_idx, 1)
                if not terra_data or len(terra_data) < 256:
                    print(f'  Terra set {terra_idx}: [data not available]')
                    continue

                entries = decode_terra_block(terra_data)

                type_counts = Counter()
                for e in entries:
                    if e['tiles_mask'] != 0:  # Only count tiles that have a mask
                        type_counts[e['terrain_type']] += 1

                tile_range = '0-127' if ti == 0 else '128-255'
                if type_counts:
                    type_strs = []
                    for t in sorted(type_counts.keys()):
                        info = TERRAIN_TYPE_INFO.get(t, f'type-{t}')
                        type_strs.append(f'    type {t:2d} ({info}): {type_counts[t]} tiles')
                    print(f'  Terra set {terra_idx} (tiles {tile_range}):')
                    for s in type_strs:
                        print(s)
                else:
                    print(f'  Terra set {terra_idx} (tiles {tile_range}): no masked tiles')

            print()


def cmd_terrain_types(image_file, terra1, terra2):
    """Show detailed terrain types for a specific terrain set pair."""
    print(f'Terrain Types for terra sets {terra1}, {terra2}')
    print('=' * 70)

    with open(image_file, 'rb') as f:
        for terra_idx in [terra1, terra2]:
            terra_data = read_blocks(f, TERRA_BLOCK + terra_idx, 1)
            entries = decode_terra_block(terra_data)

            print(f'\nTerra set {terra_idx} ({TERRAIN_SET_NAMES.get(terra_idx, "?")}):'
                  f' {len(entries)} entries')
            print(f'  {"Idx":>4} {"MapTag":>6} {"TType":>5} {"TSub":>4} '
                  f'{"Mask":>6} {"Colors":>6}  Meaning')
            print(f'  {"----":>4} {"------":>6} {"-----":>5} {"----":>4} '
                  f'{"------":>6} {"------":>6}  -------')

            for e in entries:
                if e['tiles_mask'] == 0 and e['terrain_type'] == 0:
                    continue  # Skip empty entries
                info = TERRAIN_TYPE_INFO.get(e['terrain_type'], '')
                print(f"  {e['index']:4d} {e['maptag']:6d} {e['terrain_type']:5d} "
                      f"{e['terrain_sub']:4d} {e['tiles_mask']:6d} "
                      f"{e['big_colors']:6d}  {info}")


def cmd_find_terrain_type(image_file, target_type):
    """Find all terrain sets containing a specific terrain type."""
    print(f'Searching for terrain type {target_type} across all terrain sets')
    print('=' * 70)

    info = TERRAIN_TYPE_INFO.get(target_type, 'unknown')
    print(f'Type {target_type}: {info}')
    print()

    found_in = []

    with open(image_file, 'rb') as f:
        for terra_idx in range(11):  # 0-10 terrain sets
            terra_data = read_blocks(f, TERRA_BLOCK + terra_idx, 1)
            if not terra_data or len(terra_data) < BLOCK_SIZE:
                continue

            entries = decode_terra_block(terra_data)
            matching = [e for e in entries if e['terrain_type'] == target_type
                        and e['tiles_mask'] != 0]

            if matching:
                found_in.append(terra_idx)
                print(f'Terra set {terra_idx} ({TERRAIN_SET_NAMES.get(terra_idx, "?")}): '
                      f'{len(matching)} tiles with type {target_type}')
                for e in matching:
                    print(f"  tile {e['index']:3d}: mask=0x{e['tiles_mask']:02x}")

    print()
    if found_in:
        # Cross-reference with regions
        print('Regions using these terrain sets:')
        for region_idx, (_, t1, t2, _, _, name) in enumerate(FILE_INDEX):
            if t1 in found_in or t2 in found_in:
                which = []
                if t1 in found_in:
                    which.append(f'terra1={t1}')
                if t2 in found_in:
                    which.append(f'terra2={t2}')
                print(f'  Region {region_idx}: {name} ({", ".join(which)})')

        # Check for inventory overrides
        print()
        print('Inventory overrides for this terrain type (from movement code):')
        _check_terrain_overrides(target_type)
    else:
        print(f'Terrain type {target_type} not found in any terrain set.')


def _check_terrain_overrides(target_type):
    """Search fmain.c for conditional terrain overrides matching the target type."""
    fmain_path = os.path.join(REPO_ROOT, 'fmain.c')
    if not os.path.exists(fmain_path):
        print('  [fmain.c not found]')
        return

    import re
    pattern = re.compile(rf'j\s*==\s*{target_type}|j=={target_type}')
    with open(fmain_path, 'r', errors='replace') as f:
        for line_num, line in enumerate(f, 1):
            if pattern.search(line):
                print(f'  fmain.c:{line_num}: {line.strip()}')


def _region_xreg(region_idx):
    """Calculate xreg for a region index.

    Paired outdoor regions share a 128-column-wide map block.
    Even regions (xr=0) use columns 0-63, odd regions (xr=1) use columns
    64-127.  Indoor regions 8/9 both force xr=0 (fmain.c:2985).

    Returns (xreg, col_start, col_count) for display.
    """
    if region_idx > 7:
        # Indoor regions: xr forced to 0, full 64-col width
        return 0, 0, 64
    xr = region_idx & 1
    xreg = xr << 6  # 0 or 64
    return xreg, xreg, 64


def cmd_region_map(image_file, region_idx):
    """Display the region map as a sector number grid."""
    if region_idx < 0 or region_idx >= len(FILE_INDEX):
        print(f'Error: region index must be 0-{len(FILE_INDEX)-1}')
        sys.exit(1)

    images, t1, t2, sector_blk, region_blk, name = FILE_INDEX[region_idx]
    xreg, col_start, col_count = _region_xreg(region_idx)

    print(f'Region Map: {name} (region block={region_blk}, xreg={xreg})')
    print('=' * 70)

    with open(image_file, 'rb') as f:
        region_data = read_blocks(f, region_blk, 8)

    if not region_data:
        print('[region data not available]')
        return

    # Region map is 4096 bytes, indexed as: secy * 128 + secx + xreg
    # Paired outdoor regions occupy different 32-col halves of the same block
    rows = REGION_MAP_SIZE // 128  # 32 rows max

    print(f'Grid: {rows} rows x {col_count} cols, '
          f'map columns {col_start}-{col_start + col_count - 1} (sector numbers)')
    print()

    # Header
    header = '     '
    for c in range(col_count):
        header += f'{col_start + c:4d}'
    print(header)

    for r in range(rows):
        row_str = f'{r:3d}: '
        for c in range(col_count):
            offset = r * 128 + col_start + c
            if offset < len(region_data):
                sector = region_data[offset]
                if sector == 0:
                    row_str += '   .'
                else:
                    row_str += f'{sector:4d}'
            else:
                row_str += '   ?'
        print(row_str)


def cmd_sector_detail(image_file, sector_num, region_idx=0):
    """Show tile-level detail for a specific sector with terrain types."""
    images, t1, t2, sector_blk, region_blk, name = FILE_INDEX[region_idx]

    print(f'Sector {sector_num} detail (Region {region_idx}: {name})')
    print(f'Sector data at block {sector_blk}, terra sets {t1}/{t2}')
    print('=' * 70)

    with open(image_file, 'rb') as f:
        sector_data = read_blocks(f, sector_blk, 64)
        terra1_data = read_blocks(f, TERRA_BLOCK + t1, 1)
        terra2_data = read_blocks(f, TERRA_BLOCK + t2, 1)

    terra1_entries = decode_terra_block(terra1_data)
    terra2_entries = decode_terra_block(terra2_data)

    tiles = decode_sector(sector_data, sector_num)
    if tiles is None:
        print(f'[sector {sector_num} data not available]')
        return

    # Build lookup: tile_index -> terrain_type
    terra_lookup = {}
    for e in terra1_entries[:64]:
        terra_lookup[e['index']] = e
    for e in terra2_entries[64:128]:
        terra_lookup[e['index']] = e

    print(f'\nTile grid (16 cols x 8 rows) — showing tile_index:terrain_type')
    print()
    for row_idx, row in enumerate(tiles):
        cells = []
        for tile_idx in row:
            entry = terra_lookup.get(tile_idx)
            if entry:
                ttype = entry['terrain_type']
                cells.append(f'{tile_idx:3d}:{ttype}')
            else:
                cells.append(f'{tile_idx:3d}:?')
        print(f'  row {row_idx}: ' + ' '.join(cells))

    # Summary of terrain types in this sector
    print()
    type_counts = Counter()
    for row in tiles:
        for tile_idx in row:
            entry = terra_lookup.get(tile_idx)
            if entry:
                type_counts[entry['terrain_type']] += 1

    print('Terrain type distribution in this sector:')
    for t in sorted(type_counts.keys()):
        info = TERRAIN_TYPE_INFO.get(t, '')
        print(f'  type {t:2d}: {type_counts[t]:3d} tiles  {info}')


def cmd_minimap(image_file, region_idx):
    """Generate a minimap PNG for an entire region, mirroring the bird totem.

    The bird totem (fsubs.asm:903-1030 bigdraw/plotsect) renders each sector
    as a 16x8 pixel block.  For each tile in the sector, it reads
    terra_mem[tile_index * 4 + 3] (the big_colors byte) as a 5-bit index
    into the 32-color Amiga palette.

    This generates the same visualization for the full region map:
    64 cols x 32 rows of sectors, each 16x8 tiles = 1024 x 256 pixels.
    """
    from PIL import Image

    if region_idx < 0 or region_idx >= len(FILE_INDEX):
        print(f'Error: region index must be 0-{len(FILE_INDEX)-1}')
        sys.exit(1)

    images, t1, t2, sector_blk, region_blk, name = FILE_INDEX[region_idx]
    xreg, col_start, col_count = _region_xreg(region_idx)

    with open(image_file, 'rb') as f:
        region_data = read_blocks(f, region_blk, 8)
        sector_data = read_blocks(f, sector_blk, 64)
        terra1_raw = read_blocks(f, TERRA_BLOCK + t1, 1)
        terra2_raw = read_blocks(f, TERRA_BLOCK + t2, 1)

    # Combine into a single 1024-byte terra_mem matching the game layout:
    # tiles 0-127 → terra1 (512 bytes), tiles 128-255 → terra2 (512 bytes)
    terra_mem = terra1_raw[:512] + terra2_raw[:512]

    scale = 2  # pixels per tile
    rows = REGION_MAP_SIZE // 128  # 32
    img_w = col_count * 16 * scale
    img_h = rows * 8 * scale
    img = Image.new('RGB', (img_w, img_h), (0, 0, 0))
    pixels = img.load()

    for sec_row in range(rows):
        for sec_col in range(col_count):
            map_offset = sec_row * 128 + col_start + sec_col
            if map_offset >= len(region_data):
                continue
            sector_num = region_data[map_offset]

            sec_offset = sector_num * SECTOR_SIZE
            if sec_offset + SECTOR_SIZE > len(sector_data):
                continue

            # Render 16 cols x 8 rows of tiles for this sector
            px_x = sec_col * 16 * scale
            px_y = sec_row * 8 * scale
            for tile_row in range(8):
                for tile_col in range(16):
                    tile_idx = sector_data[sec_offset + tile_row * 16 + tile_col]
                    # big_colors = terra_mem[tile_index * 4 + 3]
                    terra_offset = tile_idx * 4 + 3
                    if terra_offset < len(terra_mem):
                        color_idx = terra_mem[terra_offset] & 0x1F
                    else:
                        color_idx = 0
                    color = AMIGA_PALETTE[color_idx]
                    bx = px_x + tile_col * scale
                    by = px_y + tile_row * scale
                    for dy in range(scale):
                        for dx in range(scale):
                            pixels[bx + dx, by + dy] = color

    results_dir = os.path.join(REPO_ROOT, 'tools', 'results')
    os.makedirs(results_dir, exist_ok=True)
    out_path = os.path.join(results_dir, f'region_{region_idx}.png')
    img.save(out_path)
    print(f'Region {region_idx}: {name}')
    print(f'  Terra sets: {t1} ({TERRAIN_SET_NAMES.get(t1, "?")}), '
          f'{t2} ({TERRAIN_SET_NAMES.get(t2, "?")})')
    print(f'  Size: {img_w}x{img_h} pixels '
          f'({col_count} sector cols x {rows} sector rows)')
    print(f'  Saved to {out_path}')


def cmd_overworld(image_file):
    """Generate a complete overworld map combining all 8 outdoor regions.

    The outdoor world is a 2-column x 4-row grid of regions derived from
    the hero's sector coordinates (fmain.c:2973-2975):
        xr = (xs>>6) & 1       → column 0 or 1
        yr = (ys>>5) & 3       → row 0..3
        lregion = xr + 2*yr    → region index 0..7

    Paired regions share a region_blk (8 blocks = 4096 bytes of sector
    assignments), with even regions occupying columns 0-63 and odd regions
    columns 64-127.  Each region has its own terra sets, so the same sector
    tile indices map to different colors per region.

    Full map: 128 sector cols x 128 sector rows, each sector 16x8 tiles
    at scale 2 = 4096 x 2048 pixels.
    """
    from PIL import Image

    SCALE = 2
    SECTOR_COLS = 128   # 64 per region, 2 regions side by side
    SECTOR_ROWS = 128   # 32 per region, 4 regions stacked
    TILE_W, TILE_H = 16, 8
    img_w = SECTOR_COLS * TILE_W * SCALE
    img_h = SECTOR_ROWS * TILE_H * SCALE
    img = Image.new('RGB', (img_w, img_h), (0, 0, 0))
    pixels = img.load()

    with open(image_file, 'rb') as f:
        # Shared sector data (all outdoor regions use sector_blk=32)
        sector_data = read_blocks(f, 32, 64)

        # Load per-pair region maps and per-region terra sets
        for pair_row in range(4):
            even_idx = pair_row * 2
            odd_idx = even_idx + 1
            _, t1_e, t2_e, _, region_blk, _ = FILE_INDEX[even_idx]
            _, t1_o, t2_o, _, _,          _ = FILE_INDEX[odd_idx]

            region_data = read_blocks(f, region_blk, 8)
            terra_even = (read_blocks(f, TERRA_BLOCK + t1_e, 1)[:512]
                          + read_blocks(f, TERRA_BLOCK + t2_e, 1)[:512])
            terra_odd = (read_blocks(f, TERRA_BLOCK + t1_o, 1)[:512]
                         + read_blocks(f, TERRA_BLOCK + t2_o, 1)[:512])

            for sec_row in range(32):
                for sec_col in range(SECTOR_COLS):
                    map_offset = sec_row * 128 + sec_col
                    if map_offset >= len(region_data):
                        continue
                    sector_num = region_data[map_offset]

                    sec_offset = sector_num * SECTOR_SIZE
                    if sec_offset + SECTOR_SIZE > len(sector_data):
                        continue

                    terra_mem = terra_odd if sec_col >= 64 else terra_even

                    px_x = sec_col * TILE_W * SCALE
                    px_y = (pair_row * 32 + sec_row) * TILE_H * SCALE
                    for tr in range(TILE_H):
                        for tc in range(TILE_W):
                            tile_idx = sector_data[sec_offset + tr * 16 + tc]
                            terra_off = tile_idx * 4 + 3
                            if terra_off < len(terra_mem):
                                color_idx = terra_mem[terra_off] & 0x1F
                            else:
                                color_idx = 0
                            color = AMIGA_PALETTE[color_idx]
                            bx = px_x + tc * SCALE
                            by = px_y + tr * SCALE
                            for dy in range(SCALE):
                                for dx in range(SCALE):
                                    pixels[bx + dx, by + dy] = color

    out_path = os.path.join(REPO_ROOT, 'docs', 'overworld.png')
    img.save(out_path)
    print(f'Overworld map: {img_w}x{img_h} pixels')
    print(f'  128 sector cols x 128 sector rows (8 regions in 2x4 grid)')
    print(f'  Saved to {out_path}')


# ---------------------------------------------------------------------------
# Spatial cross-referencing helpers
# ---------------------------------------------------------------------------

def pixel_to_outdoor_region(x, y):
    """Compute outdoor region index (0-7) from absolute pixel coordinates.
    Formula: ((y >> 13) & 3) * 2 + ((x >> 14) & 1) — fmain.c:2972-2977
    """
    xs = x >> 8
    ys = y >> 8
    xr = (xs >> 6) & 1
    yr = (ys >> 5) & 3
    return yr * 2 + xr


def region_params(region_idx):
    """Return (xreg, yreg) for a region — fmain.c:2983-2987."""
    yr = region_idx >> 1
    xr = region_idx & 1
    if region_idx > 7:
        xr = 0
    return xr << 6, yr << 5


def pixel_to_grid(x, y, xreg, yreg):
    """Convert pixel coordinates to region grid (col, row).
    col = (x >> 8) - xreg, row = (y >> 8) - yreg — fsubs.asm:561-579
    """
    return (x >> 8) - xreg, (y >> 8) - yreg


def lookup_sector(region_data, col, row):
    """Look up sector number from region grid data."""
    if row < 0 or row >= 32 or col < 0 or col >= 128:
        return None
    offset = row * 128 + col
    if offset >= len(region_data):
        return None
    return region_data[offset]


def lookup_place_name(sector, is_indoor):
    """Look up place name for a sector number (first-match scan)."""
    tbl = INSIDE_TBL if is_indoor else PLACE_TBL
    msgs = INSIDE_MSG if is_indoor else PLACE_MSG
    sector = sector & 0xFF
    for low, high, msg_idx in tbl:
        if low <= sector <= high:
            if msg_idx < len(msgs) and msgs[msg_idx]:
                return msgs[msg_idx]
            return None
    return None


def resolve_location(x, y, list_name, region_maps):
    """Resolve pixel coordinates to region/sector/place for a given object list."""
    if x == 0 and y == 0:
        return {'region': None, 'sector': None, 'grid_col': None,
                'grid_row': None, 'place_name': None}

    if list_name == '8':
        region_idx, is_indoor = 8, True
    elif list_name == '9':
        region_idx, is_indoor = 9, True
    elif list_name == 'global':
        # Global objects: indoor if y > 33000
        is_indoor = y > 33000
        region_idx = 8 if is_indoor else pixel_to_outdoor_region(x, y)
    else:
        list_region = int(list_name)
        if list_region < 8:
            # Outdoor list: use coordinates to determine actual region
            is_indoor = y > 0x8000
            region_idx = 8 if is_indoor else pixel_to_outdoor_region(x, y)
        else:
            region_idx = list_region
            is_indoor = True

    xreg, yreg = region_params(region_idx)
    col, row = pixel_to_grid(x, y, xreg, yreg)
    region_data = region_maps.get(region_idx, [])
    sector = lookup_sector(region_data, col, row)
    place_name = lookup_place_name(sector, is_indoor) if sector is not None else None

    return {'region': region_idx, 'sector': sector, 'grid_col': col,
            'grid_row': row, 'place_name': place_name}


def get_ob_name(ob_id, ob_stat):
    """Determine object name from ob_id + ob_stat."""
    if ob_stat in (3, 4) and ob_id in SETFIG_NAMES:
        return SETFIG_NAMES[ob_id]
    if ob_id in ITEM_NAMES:
        return ITEM_NAMES[ob_id]
    return f'unknown ({ob_id})'


def sector_terrain_summary(region_idx, sector_num, sector_data, terra_sets):
    """Compute terrain type distribution for a sector in a specific region."""
    _, t1, t2, _, _, _ = FILE_INDEX[region_idx]
    terra1 = decode_terra_block(terra_sets[t1])
    terra2 = decode_terra_block(terra_sets[t2])

    offset = sector_num * SECTOR_SIZE
    if offset + SECTOR_SIZE > len(sector_data):
        return None

    type_counts = Counter()
    for i in range(SECTOR_SIZE):
        tile_id = sector_data[offset + i]
        if tile_id < 128:
            entry = terra1[tile_id] if tile_id < len(terra1) else None
        else:
            idx = tile_id - 128
            entry = terra2[idx] if idx < len(terra2) else None

        if entry and entry['tiles_mask'] != 0:
            type_counts[entry['terrain_type']] += 1
        else:
            type_counts[0] += 1  # fully passable

    return {str(k): v for k, v in sorted(type_counts.items())}


def cmd_export_world_db(image_file, output_path):
    """Export unified spatial database as JSON to docs/world_db.json."""
    import json

    # Read all region maps
    region_maps = {}
    with open(image_file, 'rb') as f:
        for ri in range(len(FILE_INDEX)):
            region_blk = FILE_INDEX[ri][4]
            data = read_blocks(f, region_blk, 8)
            region_maps[ri] = list(data[:REGION_MAP_SIZE])

    # Read sector data pools
    with open(image_file, 'rb') as f:
        outdoor_sectors = read_blocks(f, 32, 64)
        indoor_sectors = read_blocks(f, 96, 64)

    # Read all terra sets
    terra_data = {}
    with open(image_file, 'rb') as f:
        for i in range(11):
            terra_data[i] = read_blocks(f, TERRA_BLOCK + i, 1)

    # --- Build regions ---
    regions = []
    for i, (images, t1, t2, sector_blk, region_blk, name) in enumerate(FILE_INDEX):
        xreg, yreg = region_params(i)
        regions.append({
            'index': i, 'name': name,
            'terra_sets': [t1, t2],
            'terra_set_names': [TERRAIN_SET_NAMES.get(t1, '?'),
                                TERRAIN_SET_NAMES.get(t2, '?')],
            'sector_block': sector_blk, 'region_block': region_blk,
            'xreg': xreg, 'yreg': yreg,
            'type': 'indoor' if i >= 8 else 'outdoor',
        })

    # --- Build objects ---
    objects = []
    for list_name, obj_list in ALL_OBJECT_LISTS.items():
        for idx, (xc, yc, ob_id, ob_stat, comment) in enumerate(obj_list):
            loc = resolve_location(xc, yc, list_name, region_maps)
            objects.append({
                'list': list_name, 'index': idx,
                'xc': xc, 'yc': yc,
                'ob_id': ob_id, 'ob_id_name': get_ob_name(ob_id, ob_stat),
                'ob_stat': ob_stat,
                'ob_stat_name': OB_STAT_NAMES.get(ob_stat, '?'),
                'comment': comment, **loc,
            })

    # --- Build doors ---
    doors = []
    for idx, (xc1, yc1, xc2, yc2, dtype, secs, comment) in enumerate(DOORLIST):
        target_region = 8 if secs == 1 else 9

        # Outside endpoint
        out_loc = {'xc': xc1, 'yc': yc1}
        if xc1 > 0 or yc1 > 0:
            # Indoor outside coords (y > 0x8000) — resolve against region 8
            if yc1 > 0x8000:
                xreg, yreg = region_params(8)
                col, row = pixel_to_grid(xc1, yc1, xreg, yreg)
                out_sector = lookup_sector(region_maps.get(8, []), col, row)
                out_loc.update({
                    'region': 8, 'region_name': FILE_INDEX[8][5],
                    'sector': out_sector, 'grid_col': col, 'grid_row': row,
                    'place_name': lookup_place_name(out_sector, True) if out_sector else None,
                })
            else:
                out_region = pixel_to_outdoor_region(xc1, yc1)
                xreg, yreg = region_params(out_region)
                col, row = pixel_to_grid(xc1, yc1, xreg, yreg)
                out_sector = lookup_sector(region_maps.get(out_region, []), col, row)
                out_loc.update({
                    'region': out_region, 'region_name': FILE_INDEX[out_region][5],
                    'sector': out_sector, 'grid_col': col, 'grid_row': row,
                    'place_name': lookup_place_name(out_sector, False) if out_sector else None,
                })

        # Inside endpoint
        in_loc = {'xc': xc2, 'yc': yc2}
        xreg, yreg = region_params(target_region)
        col, row = pixel_to_grid(xc2, yc2, xreg, yreg)
        in_sector = lookup_sector(region_maps.get(target_region, []), col, row)
        in_loc.update({
            'region': target_region, 'sector': in_sector,
            'grid_col': col, 'grid_row': row,
            'place_name': lookup_place_name(in_sector, True) if in_sector else None,
        })

        doors.append({
            'index': idx, 'outside': out_loc, 'inside': in_loc,
            'type': dtype, 'type_name': DOOR_TYPE_NAMES.get(dtype, '?'),
            'secs': secs, 'target_region': target_region,
            'comment': comment,
        })

    # --- Build extents ---
    extents = []
    for idx, (x1, y1, x2, y2, etype, v1, v2, v3, comment) in enumerate(EXTENT_LIST):
        if x1 == 0 and y1 == 0 and x2 == 0 and y2 == 0:
            ext_region = None
        elif y1 > 33000:
            ext_region = 8  # indoor
        elif x2 >= 0x7fff:
            ext_region = 'all'
        else:
            ext_region = pixel_to_outdoor_region((x1 + x2) // 2, (y1 + y2) // 2)

        extents.append({
            'index': idx, 'x1': x1, 'y1': y1, 'x2': x2, 'y2': y2,
            'etype': etype,
            'etype_name': EXTENT_TYPE_NAMES.get(etype, f'type {etype}'),
            'v1': v1, 'v2': v2, 'v3': v3,
            'comment': comment, 'region': ext_region,
        })

    # --- Hardcoded zones ---
    zones = [
        {'name': 'fiery_death',
         'x1': 8802, 'y1': 24744, 'x2': 13562, 'y2': 29544,
         'description': 'Volcanic zone: environ damage, swan dismount blocked',
         'source': 'fmain.c:1384-1385'},
        {'name': 'desert_gate',
         'description': 'DESERT doors blocked unless stuff[STATBASE] >= 5 (5 gold statues)',
         'source': 'fmain.c:1919',
         'affected_door_indices': [i for i, d in enumerate(DOORLIST) if d[4] == 17]},
        {'name': 'sector_181_quicksand',
         'description': 'Deep water in sector 181 teleports to region 9 instead of death',
         'source': 'fmain.c:1784-1793', 'sector': 181},
    ]

    # --- Region grids and terrain summaries ---
    region_grids = {}
    all_sector_terrain = {}

    for ri in range(len(FILE_INDEX)):
        xreg, yreg = region_params(ri)
        rdata = region_maps[ri]
        col_start = xreg
        sector_pool = indoor_sectors if ri >= 8 else outdoor_sectors
        grid = []
        unique_sectors = set()

        for r in range(32):
            row_data = []
            for c in range(64):
                off = r * 128 + col_start + c
                sec = rdata[off] if off < len(rdata) else 0
                row_data.append(sec)
                if sec > 0:
                    unique_sectors.add(sec)
            grid.append(row_data)

        region_grids[str(ri)] = {
            'xreg': xreg, 'yreg': yreg,
            'rows': 32, 'cols': 64, 'grid': grid,
        }

        terrain = {}
        for sec_num in sorted(unique_sectors):
            summary = sector_terrain_summary(ri, sec_num, sector_pool, terra_data)
            if summary:
                terrain[str(sec_num)] = summary
        all_sector_terrain[str(ri)] = terrain

    # --- Assemble database ---
    db = {
        'metadata': {
            'generated': str(date.today()),
            'description': (
                'Unified spatial database for The Faery Tale Adventure '
                '(Amiga, 1987). Cross-references objects, doors, encounters, '
                'terrain, and place names for reverse-engineering research.'
            ),
            'coordinate_system': {
                'unit': 'pixels',
                'tile_size': [16, 32],
                'sector_size_tiles': [16, 8],
                'sector_size_pixels': [256, 256],
                'outdoor_world_size': [32768, 40960],
                'region_grid': '64 cols x 32 rows per region half',
                'outdoor_region_formula': '((y >> 13) & 3) * 2 + ((x >> 14) & 1)',
                'grid_col_formula': 'x >> 8',
                'grid_row_formula': '(y >> 8) - yreg',
                'direction_encoding': '0=NW 1=N 2=NE 3=E 4=SE 5=S 6=SW 7=W',
            },
            'sources': {
                'objects': 'fmain2.c:1001-1178',
                'doors': 'fmain.c:240-325',
                'extents': 'fmain.c:338-371',
                'place_names': 'narr.asm:86-223',
                'terrain': 'game/image binary, terra blocks at offset 149',
                'region_maps': 'game/image binary, region block offsets per file_index',
            },
        },
        'regions': regions,
        'terrain_type_key': {str(k): v for k, v in TERRAIN_TYPE_INFO.items()},
        'place_names': {
            'outdoor': [
                {'sector_low': lo, 'sector_high': hi, 'msg_idx': m,
                 'name': PLACE_MSG[m] if m < len(PLACE_MSG) and PLACE_MSG[m] else None}
                for lo, hi, m in PLACE_TBL],
            'indoor': [
                {'sector_low': lo, 'sector_high': hi, 'msg_idx': m,
                 'name': INSIDE_MSG[m] if m < len(INSIDE_MSG) and INSIDE_MSG[m] else None}
                for lo, hi, m in INSIDE_TBL],
        },
        'objects': objects,
        'doors': doors,
        'extents': extents,
        'zones': zones,
        'region_grids': region_grids,
        'sector_terrain': all_sector_terrain,
    }

    os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)
    with open(output_path, 'w') as f:
        json.dump(db, f, indent=2)

    # Summary
    n_terrain = sum(len(v) for v in all_sector_terrain.values())
    print(f'World database exported to {output_path}')
    print(f'  {len(objects)} objects across {len(ALL_OBJECT_LISTS)} lists')
    print(f'  {len(doors)} doors')
    print(f'  {len(extents)} encounter extents')
    print(f'  {len(zones)} hardcoded zones')
    print(f'  {n_terrain} sector terrain summaries across {len(FILE_INDEX)} regions')


def main():
    parser = argparse.ArgumentParser(
        description='Decode and navigate world map data from game/image')

    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument('--terrain-summary', action='store_true',
                       help='Show terrain type distribution per region')
    group.add_argument('--terrain-types', nargs=2, type=int, metavar=('TERRA1', 'TERRA2'),
                       help='Show detailed terrain types for a set pair')
    group.add_argument('--find-terrain-type', type=int, metavar='TYPE',
                       help='Find all terrain sets containing terrain type N')
    group.add_argument('--region-map', type=int, metavar='REGION_IDX',
                       help='Display region map as sector grid (0-9)')
    group.add_argument('--sector-detail', type=int, metavar='SECTOR_NUM',
                       help='Show tile-level detail for a sector')
    group.add_argument('--minimap', type=int, metavar='REGION_IDX',
                       help='Generate bird-totem-style minimap PNG (0-9)')
    group.add_argument('--overworld', action='store_true',
                       help='Generate complete overworld map PNG (all 8 outdoor regions)')
    group.add_argument('--export-world-db', action='store_true',
                       help='Export unified spatial database to docs/world_db.json')

    parser.add_argument('--region', type=int, default=0,
                        help='Region index for --sector-detail (default: 0)')
    parser.add_argument('--output', help='Write results to this file')

    args = parser.parse_args()

    if not os.path.exists(IMAGE_PATH):
        print(f'Error: {IMAGE_PATH} not found. The game/image binary is required.')
        sys.exit(1)

    # Capture output if requested
    import io
    if args.output:
        old_stdout = sys.stdout
        sys.stdout = buffer = io.StringIO()

    if args.terrain_summary:
        cmd_terrain_summary(IMAGE_PATH)
    elif args.terrain_types:
        cmd_terrain_types(IMAGE_PATH, args.terrain_types[0], args.terrain_types[1])
    elif args.find_terrain_type is not None:
        cmd_find_terrain_type(IMAGE_PATH, args.find_terrain_type)
    elif args.region_map is not None:
        cmd_region_map(IMAGE_PATH, args.region_map)
    elif args.sector_detail is not None:
        cmd_sector_detail(IMAGE_PATH, args.sector_detail, args.region)
    elif args.minimap is not None:
        cmd_minimap(IMAGE_PATH, args.minimap)
    elif args.overworld:
        cmd_overworld(IMAGE_PATH)
        return
    elif args.export_world_db:
        out = os.path.join(REPO_ROOT, 'docs', 'world_db.json')
        cmd_export_world_db(IMAGE_PATH, out)
        return  # export handles its own output

    if args.output:
        output = buffer.getvalue()
        sys.stdout = old_stdout
        print(output)
        outpath = os.path.join(REPO_ROOT, args.output)
        os.makedirs(os.path.dirname(outpath), exist_ok=True)
        with open(outpath, 'w') as f:
            f.write(output)
        print(f'\nResults written to {args.output}')


if __name__ == '__main__':
    main()
