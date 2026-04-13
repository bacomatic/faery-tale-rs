#!/usr/bin/env python3
"""Patch the fmain executable to enable all cheat keys unconditionally.

The original game gates cheat key handlers behind a `cheat1` flag that must
be activated by pressing a specific key sequence.  This tool NOPs out the
conditional branch instructions that skip cheat handlers when cheat1 == 0,
making all cheat keys work immediately without activation.

Additionally patches the encounter-suppression check (site #11) so that
encounters are skipped when cheats would normally suppress them.

Technical details
-----------------
The Aztec C compiler placed cheat1 at A4-relative offset -16946 (0xBDCE).
There are 11 `tst.w (-16946,a4)` instructions in the CODE hunk:

  Sites 1-10:  tst.w cheat1 ; beq.w <skip>   (input handler)
  Site 11:     tst.w cheat1 ; bne.w <skip>   (encounter suppression)

Each branch is a 4-byte beq.w (0x6700 xxxx) or bne.w (0x6600 xxxx).
The patch replaces each branch with two 68000 NOP instructions (0x4E71 4E71).
"""

import argparse
import shutil
import struct
import sys
from pathlib import Path

# Each entry: (file_offset, expected_original_bytes, description)
# File offset = code_hunk_start (0x28) + code_offset_of_branch
PATCHES = [
    (0x17C6, bytes.fromhex("67000046"), "input handler cheat check #1"),
    (0x181C, bytes.fromhex("67000024"), "input handler cheat check #2"),
    (0x1A82, bytes.fromhex("6700000a"), "input handler cheat check #3"),
    (0x1A9C, bytes.fromhex("67000010"), "input handler cheat check #4"),
    (0x1ABC, bytes.fromhex("67000010"), "input handler cheat check #5"),
    (0x1ADC, bytes.fromhex("6700000c"), "input handler cheat check #6"),
    (0x1AF8, bytes.fromhex("67000012"), "input handler cheat check #7"),
    (0x1B1A, bytes.fromhex("67000012"), "input handler cheat check #8"),
    (0x1B3C, bytes.fromhex("67000012"), "input handler cheat check #9"),
    (0x1B5E, bytes.fromhex("67000012"), "input handler cheat check #10"),
    (0x7EE4, bytes.fromhex("6600000e"), "encounter suppression check"),
]

NOP2 = bytes.fromhex("4e714e71")  # two 68000 NOPs


def verify_original(data: bytes) -> list[str]:
    """Check that all patch sites contain the expected original bytes."""
    errors = []
    for offset, expected, desc in PATCHES:
        actual = data[offset : offset + len(expected)]
        if actual != expected:
            errors.append(
                f"  {desc} at 0x{offset:04X}: "
                f"expected {expected.hex()}, found {actual.hex()}"
            )
    return errors


def is_already_patched(data: bytes) -> bool:
    """Check if all sites already contain NOPs."""
    return all(data[off : off + 4] == NOP2 for off, _, _ in PATCHES)


def apply_patches(data: bytearray) -> int:
    """Replace branch instructions with NOPs. Returns number of patches applied."""
    count = 0
    for offset, _, _ in PATCHES:
        data[offset : offset + 4] = NOP2
        count += 1
    return count


def main():
    parser = argparse.ArgumentParser(
        description="Patch fmain to enable cheat keys unconditionally."
    )
    parser.add_argument(
        "input",
        nargs="?",
        default="fmain",
        help="Path to original fmain executable (default: fmain)",
    )
    parser.add_argument(
        "-o",
        "--output",
        help="Output path (default: <input>.patched)",
    )
    parser.add_argument(
        "--in-place",
        action="store_true",
        help="Patch the file in place (no backup created)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would be patched without writing",
    )
    parser.add_argument(
        "--verify",
        action="store_true",
        help="Check patch status of an existing file",
    )
    parser.add_argument(
        "--revert",
        action="store_true",
        help="Restore original branch instructions (undo patch)",
    )
    args = parser.parse_args()

    input_path = Path(args.input)
    if not input_path.exists():
        print(f"Error: {input_path} not found", file=sys.stderr)
        sys.exit(1)

    data = bytearray(input_path.read_bytes())

    if args.verify:
        if is_already_patched(data):
            print(f"{input_path}: all 11 cheat checks are patched (NOPs)")
        else:
            errors = verify_original(data)
            if not errors:
                print(f"{input_path}: unpatched (original bytes intact)")
            else:
                print(f"{input_path}: partially patched or unknown state")
                for e in errors:
                    print(e)
        return

    if args.revert:
        if not is_already_patched(data):
            errs = verify_original(data)
            if not errs:
                print("File is already in original (unpatched) state.")
                return
            print("Error: file is in an unknown state, cannot safely revert.",
                  file=sys.stderr)
            sys.exit(1)
        for offset, original, desc in PATCHES:
            data[offset : offset + 4] = original
        out = input_path if args.in_place else Path(args.output or str(input_path) + ".reverted")
        if not args.dry_run:
            out.write_bytes(data)
            print(f"Reverted 11 patches -> {out}")
        else:
            print("Dry run: would revert 11 patches")
            for offset, original, desc in PATCHES:
                print(f"  0x{offset:04X}: 4e714e71 -> {original.hex()}  ({desc})")
        return

    # Normal patch flow
    if is_already_patched(data):
        print(f"{input_path} is already patched.")
        return

    errors = verify_original(data)
    if errors:
        print("Error: unexpected bytes at patch sites:", file=sys.stderr)
        for e in errors:
            print(e, file=sys.stderr)
        sys.exit(1)

    if args.dry_run:
        print(f"Dry run: would patch 11 cheat checks in {input_path}")
        for offset, expected, desc in PATCHES:
            print(f"  0x{offset:04X}: {expected.hex()} -> {NOP2.hex()}  ({desc})")
        return

    count = apply_patches(data)

    if args.in_place:
        out_path = input_path
    else:
        out_path = Path(args.output) if args.output else input_path.with_suffix(".patched")

    out_path.write_bytes(data)
    print(f"Patched {count} cheat checks -> {out_path}")


if __name__ == "__main__":
    main()
