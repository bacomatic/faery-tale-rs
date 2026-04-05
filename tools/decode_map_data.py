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


def cmd_region_map(image_file, region_idx):
    """Display the region map as a sector number grid."""
    if region_idx < 0 or region_idx >= len(FILE_INDEX):
        print(f'Error: region index must be 0-{len(FILE_INDEX)-1}')
        sys.exit(1)

    images, t1, t2, sector_blk, region_blk, name = FILE_INDEX[region_idx]

    print(f'Region Map: {name} (region block={region_blk})')
    print('=' * 70)

    with open(image_file, 'rb') as f:
        region_data = read_blocks(f, region_blk, 8)

    if not region_data:
        print('[region data not available]')
        return

    # Region map is 4096 bytes, indexed as: secy * 128 + secx
    # Display as a grid — outdoor maps use 32 columns, indoor 64
    cols = 64 if sector_blk == 96 else 32
    rows = REGION_MAP_SIZE // 128  # 32 rows max

    print(f'Grid: {rows} rows x {cols} cols (sector numbers)')
    print()

    # Header
    header = '     '
    for c in range(min(cols, 32)):  # Limit display width
        header += f'{c:4d}'
    print(header)

    for r in range(rows):
        row_str = f'{r:3d}: '
        for c in range(min(cols, 32)):
            offset = r * 128 + c
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
