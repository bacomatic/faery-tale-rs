#!/usr/bin/env python3
"""DEPRECATED — cheat1 is NOT at save offset 18.

Disassembly of fmain confirmed that the Aztec C compiler scattered BSS
variables, placing cheat1 at A4-relative -16946 — 474 bytes below the
saveload(&map_x, 80) block starting at A4-16472.  Byte offset 18 in the
save file is an unrelated variable.  See docs/PROBLEMS.md §P21.

To enable cheat keys, use tools/patch_cheats.py to NOP the cheat1 branch
instructions in the fmain executable instead.
"""

import sys


def main():
    print(
        "DEPRECATED: cheat1 is not at save offset 18 (see docs/PROBLEMS.md §P21).\n"
        "Use tools/patch_cheats.py to enable cheats in the fmain executable.",
        file=sys.stderr,
    )
    sys.exit(1)


if __name__ == "__main__":
    main()

