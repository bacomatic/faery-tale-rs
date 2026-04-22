#!/usr/bin/env python3
"""Set or clear the cheat flag (cheat1) in a Faery Tale Adventure save file.

The cheat1 field is a big-endian signed short at byte offset 18 in the save
file.  No code in the original source ever sets it to TRUE — the only way to
enable debug keys is to patch the save file directly.

When enabled (non-zero), cheat1 gates the following debug keys in the main
loop (fmain.c:1293-1340):

  'B'   — Summon bird (auto-grants Golden Lasso if bird active)
  '.'   — Randomize 3 inventory slots, clear Talisman
  'R'   — Trigger rescue() (princess rescue sequence)
  '='   — Run prq(2) (quest progress)
  ^S    — Run prq(3) (quest progress)
  ^R    — Advance daynight counter by 1000
  ^A/^B — Teleport hero ±150 pixels vertically
  ^C/^D — Teleport hero ±280 pixels horizontally

It also lifts the map spell's region restriction (fmain.c:3310), allowing the
overhead map to work inside buildings and in all regions.

Source references:
  - cheat1 declaration: fmain.c:562
  - Initialization to FALSE: fmain.c:1269
  - Cheat key handlers: fmain.c:1293-1340
  - Map region gate: fmain.c:3310
  - Save format (80-byte misc block): fmain2.c:1507
  - Offset table: reference/_discovery/save-load.md:186
"""

import argparse
import struct
import sys
from pathlib import Path

CHEAT_OFFSET = 18  # byte offset of cheat1 within the save file


def read_cheat(data: bytes) -> int:
    """Read the current cheat1 value from save data."""
    return struct.unpack_from(">h", data, CHEAT_OFFSET)[0]


def write_cheat(data: bytearray, value: int) -> None:
    """Write a new cheat1 value into save data."""
    struct.pack_into(">h", data, CHEAT_OFFSET, value)


def main():
    parser = argparse.ArgumentParser(
        description="Set or clear the cheat flag in an FTA save file.",
        epilog=(
            "The cheat1 field is at byte offset 18 (big-endian signed short). "
            "With no action flag, prints the current value."
        ),
    )
    parser.add_argument("savefile", help="Path to save file")

    group = parser.add_mutually_exclusive_group()
    group.add_argument(
        "--set", action="store_true",
        help="Enable cheat mode (set cheat1 to 1)",
    )
    group.add_argument(
        "--clear", action="store_true",
        help="Disable cheat mode (set cheat1 to 0)",
    )

    parser.add_argument(
        "--dry-run", action="store_true",
        help="Show what would change without writing the file",
    )

    args = parser.parse_args()
    path = Path(args.savefile)

    if not path.exists():
        print(f"Error: file not found: {path}", file=sys.stderr)
        sys.exit(1)

    data = bytearray(path.read_bytes())

    if len(data) < CHEAT_OFFSET + 2:
        print(f"Error: file too small ({len(data)} bytes)", file=sys.stderr)
        sys.exit(1)

    current = read_cheat(data)

    if not args.set and not args.clear:
        status = "ENABLED" if current else "disabled"
        print(f"cheat1 = {current}  [{status}]")
        sys.exit(0)

    new_value = 1 if args.set else 0

    if current == new_value:
        status = "ENABLED" if current else "disabled"
        print(f"cheat1 already {status} ({current}), no change needed.")
        sys.exit(0)

    write_cheat(data, new_value)

    old_status = "ENABLED" if current else "disabled"
    new_status = "ENABLED" if new_value else "disabled"

    if args.dry_run:
        print(f"Would change cheat1: {current} [{old_status}] -> "
              f"{new_value} [{new_status}]")
        print("(dry run — file not modified)")
    else:
        path.write_bytes(bytes(data))
        print(f"cheat1: {current} [{old_status}] -> "
              f"{new_value} [{new_status}]")
        print(f"Wrote {path}")


if __name__ == "__main__":
    main()
