#!/usr/bin/env python3
"""Lint strict pseudo-code in docs/logic/*.md.

Usage:
    python tools/lint_logic.py [--verbose] [--file PATH]

Exit 0 on clean lint. Non-zero on any failure. Writes a report to
tools/results/lint_logic.txt when invoked without --file.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
LOGIC_DIR = REPO_ROOT / "docs" / "logic"
RESULTS_DIR = REPO_ROOT / "tools" / "results"
RESULTS_FILE = RESULTS_DIR / "lint_logic.txt"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="lint_logic",
        description="Lint strict pseudo-code in docs/logic/*.md.",
    )
    parser.add_argument("--file", type=Path, default=None,
                        help="Lint a single markdown file instead of the whole directory.")
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args(argv)

    # Placeholder: actual check dispatch arrives in later tasks.
    _ = args
    return 0


if __name__ == "__main__":
    sys.exit(main())
