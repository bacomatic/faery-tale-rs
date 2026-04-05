#!/usr/bin/env python3
"""Extract a cross-reference map of all inventory item (stuff[N]) effects.

Scans all .c source files for every reference to stuff[N], classifies each
as a write (acquisition/consumption) or read (passive check/display), and
identifies cross-cutting mechanics — cases where an item is checked in a
different subsystem than where it's acquired.

Usage:
    python tools/extract_item_effects.py
    python tools/extract_item_effects.py --item 30
    python tools/extract_item_effects.py --cross-cutting-only
"""

import argparse
import os
import re
import sys
from collections import defaultdict
from datetime import date

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Match stuff[N] with numeric index — captures the index and surrounding context
STUFF_RE = re.compile(r'stuff\[(\d+)\]')

# Match stuff[SYMBOLIC] — captures the symbol name
STUFF_SYM_RE = re.compile(r'stuff\[([A-Z_]+)\]')

# Known symbolic constants from fmain.c
SYMBOLIC_CONSTANTS = {
    'MAGICBASE': 9,
    'KEYBASE': 16,
    'STATBASE': 25,
    'GOLDBASE': 31,
    'ARROWBASE': 35,
}

# Item names from inv_item table (fmain.c:388-431)
ITEM_NAMES = {
    0: 'Sword', 1: 'Shield', 2: 'Vitality', 3: 'Luck', 4: 'Kindness',
    5: 'Turtle', 6: 'Sea Shell', 7: 'Sun Stone', 8: 'Arrows',
    9: 'Blue Stone', 10: 'Green Jewel', 11: 'Glass Vial', 12: 'Crystal Orb',
    13: 'Bird Totem', 14: 'Gold Ring', 15: 'Jade Skull',
    16: 'Gold Key', 17: 'Green Key', 18: 'Blue Key', 19: 'Red Key',
    20: 'Grey Key', 21: 'White Key', 22: 'Talisman', 23: 'Rose',
    24: 'Fruit', 25: 'Gold Statue', 26: 'Book', 27: 'Herb', 28: 'Writ',
    29: 'Bone', 30: 'Shard',
    31: '2 Gold', 32: '5 Gold', 33: '10 Gold', 34: '100 Gold',
    35: 'Quiver',
}

# Write patterns: stuff[N] on the left side of assignment or increment/decrement
WRITE_PATTERNS = [
    re.compile(r'stuff\[\d+\]\s*='),
    re.compile(r'stuff\[\d+\]\s*\+='),
    re.compile(r'stuff\[\d+\]\s*-='),
    re.compile(r'stuff\[\d+\]\s*\|='),
    re.compile(r'stuff\[\d+\]\s*&='),
    re.compile(r'stuff\[\d+\]\+\+'),
    re.compile(r'stuff\[\d+\]--'),
]

# Code regions based on approximate line ranges in fmain.c
# These are heuristic — based on the structure of the game loop
FMAIN_REGIONS = [
    (1, 700, 'globals/init'),
    (700, 1050, 'setup/allocation'),
    (1050, 1270, 'main-loop-start'),
    (1270, 1400, 'menu-handling'),
    (1400, 1520, 'carrier/mounting'),
    (1520, 1700, 'movement/collision'),
    (1700, 1900, 'combat/shooting'),
    (1900, 2100, 'environment/doors'),
    (2100, 2300, 'daynight/survival'),
    (2300, 2600, 'rendering'),
    (2600, 2900, 'sector-loading'),
    (2900, 3100, 'item-management'),
    (3100, 3300, 'item-use/pickup'),
    (3300, 3600, 'save/load/misc'),
    (3600, 9999, 'disk-io/cleanup'),
]

FMAIN2_REGIONS = [
    (1, 200, 'npc-dialogue'),
    (200, 400, 'movement-helpers'),
    (400, 700, 'quest-logic'),
    (700, 1000, 'shops/trading'),
    (1000, 9999, 'save/load/win'),
]


def classify_region(filename, line_num):
    """Classify a code reference into a subsystem region."""
    base = os.path.basename(filename)
    if base == 'fmain.c':
        for start, end, name in FMAIN_REGIONS:
            if start <= line_num < end:
                return name
    elif base == 'fmain2.c':
        for start, end, name in FMAIN2_REGIONS:
            if start <= line_num < end:
                return name
    return os.path.splitext(base)[0]


def is_write(line_text, match_start):
    """Determine if a stuff[] reference is a write (assignment) or read."""
    # Check the text after the match for assignment operators
    after = line_text[match_start:]
    for pat in WRITE_PATTERNS:
        if pat.search(after):
            return True
    return False


def scan_file(filepath):
    """Scan a source file for all stuff[] references.

    Returns list of (index, line_num, is_write, line_text, region).
    """
    refs = []
    with open(filepath, 'r', errors='replace') as f:
        for line_num, line in enumerate(f, 1):
            # Skip comments-only lines
            stripped = line.strip()
            if stripped.startswith('/*') or stripped.startswith('*') or stripped.startswith('//'):
                continue

            # Find numeric stuff[N] references
            for m in STUFF_RE.finditer(line):
                idx = int(m.group(1))
                write = is_write(line, m.start())
                region = classify_region(filepath, line_num)
                refs.append((idx, line_num, write, stripped, region))

            # Find symbolic stuff[CONST] references
            for m in STUFF_SYM_RE.finditer(line):
                sym = m.group(1)
                if sym in SYMBOLIC_CONSTANTS:
                    idx = SYMBOLIC_CONSTANTS[sym]
                    write = is_write(line, m.start())
                    region = classify_region(filepath, line_num)
                    refs.append((idx, line_num, write, stripped, region))

    return refs


def find_source_files():
    """Find all C source files in the repo root."""
    sources = []
    for entry in os.listdir(REPO_ROOT):
        full = os.path.join(REPO_ROOT, entry)
        if os.path.isfile(full) and entry.endswith('.c'):
            sources.append(full)
    return sorted(sources)


def find_loop_refs(filepath):
    """Find stuff[] references inside for/while loops that iterate over indices.

    These indicate bulk operations (e.g., clearing all items) rather than
    specific item checks.
    """
    loop_ranges = []
    with open(filepath, 'r', errors='replace') as f:
        lines = f.readlines()

    brace_depth = 0
    in_loop = False
    loop_start = 0

    for i, line in enumerate(lines, 1):
        if re.search(r'\bfor\s*\(', line) or re.search(r'\bwhile\s*\(', line):
            # Check if the loop iterates with a variable index into stuff[]
            if re.search(r'stuff\[\w+\]', line) or 'GOLDBASE' in line or 'ARROWBASE' in line:
                in_loop = True
                loop_start = i

        if in_loop:
            brace_depth += line.count('{') - line.count('}')
            if brace_depth <= 0:
                loop_ranges.append((loop_start, i))
                in_loop = False
                brace_depth = 0

    return loop_ranges


def main():
    parser = argparse.ArgumentParser(description='Extract inventory item cross-reference map')
    parser.add_argument('--item', type=int, help='Show references for a specific stuff[N] index only')
    parser.add_argument('--cross-cutting-only', action='store_true',
                        help='Only show items with passive checks outside their acquisition subsystem')
    parser.add_argument('--verbose', action='store_true', help='Show full line text for each reference')
    parser.add_argument('--output', help='Write detailed results to this file')
    args = parser.parse_args()

    sources = find_source_files()
    all_refs = defaultdict(list)  # idx -> [(filename, line_num, is_write, line_text, region)]

    for filepath in sources:
        basename = os.path.basename(filepath)
        refs = scan_file(filepath)
        for idx, line_num, write, line_text, region in refs:
            all_refs[idx].append((basename, line_num, write, line_text, region))

    # Build per-item summary
    items = {}
    for idx in sorted(all_refs.keys()):
        if args.item is not None and idx != args.item:
            continue

        writes = [(f, ln, txt, reg) for f, ln, w, txt, reg in all_refs[idx] if w]
        reads = [(f, ln, txt, reg) for f, ln, w, txt, reg in all_refs[idx] if not w]

        write_regions = set(reg for _, _, _, reg in writes)
        read_regions = set(reg for _, _, _, reg in reads)
        cross_cutting = read_regions - write_regions

        name = ITEM_NAMES.get(idx, f'unknown_{idx}')
        items[idx] = {
            'name': name,
            'writes': writes,
            'reads': reads,
            'write_regions': write_regions,
            'read_regions': read_regions,
            'cross_cutting': cross_cutting,
        }

    # Filter cross-cutting only
    if args.cross_cutting_only:
        items = {k: v for k, v in items.items() if v['cross_cutting']}

    # Output
    lines = []
    lines.append(f'Inventory Cross-Reference Map (stuff[N])')
    lines.append(f'Generated: {date.today().isoformat()}')
    lines.append(f'Source files scanned: {", ".join(os.path.basename(f) for f in sources)}')
    lines.append('')

    cross_cutting_count = sum(1 for v in items.values() if v['cross_cutting'])
    lines.append(f'Items with cross-cutting passive checks: {cross_cutting_count}')
    lines.append('')

    for idx in sorted(items.keys()):
        item = items[idx]
        flag = ' *** CROSS-CUTTING ***' if item['cross_cutting'] else ''
        lines.append(f"stuff[{idx}] ({item['name']}){flag}")

        if item['writes']:
            lines.append('  WRITES:')
            for f, ln, txt, reg in item['writes']:
                line_info = f'    {f}:{ln} [{reg}]'
                if args.verbose:
                    line_info += f'  {txt}'
                lines.append(line_info)

        if item['reads']:
            lines.append('  READS:')
            for f, ln, txt, reg in item['reads']:
                line_info = f'    {f}:{ln} [{reg}]'
                if args.verbose:
                    line_info += f'  {txt}'
                lines.append(line_info)

        if item['cross_cutting']:
            lines.append(f'  CROSS-CUTTING REGIONS: {", ".join(sorted(item["cross_cutting"]))}')

        lines.append('')

    output = '\n'.join(lines)
    print(output)

    if args.output:
        outpath = os.path.join(REPO_ROOT, args.output)
        os.makedirs(os.path.dirname(outpath), exist_ok=True)
        with open(outpath, 'w') as f:
            f.write(output)
        print(f'\nDetailed results written to {args.output}')

    # Exit code: 0 if no cross-cutting found, 2 if cross-cutting items exist (needs review)
    sys.exit(2 if cross_cutting_count > 0 else 0)


if __name__ == '__main__':
    main()
