#!/usr/bin/env python3
"""Lint strict pseudo-code in docs/logic/*.md.

Usage:
    python tools/lint_logic.py [--verbose] [--file PATH]

Exit 0 on clean lint. Non-zero on any failure. Writes a report to
tools/results/lint_logic.txt when invoked without --file.
"""
from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable

REPO_ROOT = Path(__file__).resolve().parent.parent
LOGIC_DIR = REPO_ROOT / "docs" / "logic"
RESULTS_DIR = REPO_ROOT / "tools" / "results"
RESULTS_FILE = RESULTS_DIR / "lint_logic.txt"

HEADER_FIDELITY_RE = re.compile(r"^>\s*Fidelity:\s*behavioral\b", re.MULTILINE)
HEADER_CROSSREF_RE = re.compile(r"^>\s*Cross-refs:\s*", re.MULTILINE)
HEADER_SOURCES_RE = re.compile(r"Source files:\s*", re.MULTILINE)

# ---------------------------------------------------------------------------
# Data model
# ---------------------------------------------------------------------------


@dataclass
class LintIssue:
    path: Path
    line: int
    code: str
    message: str

    def format(self) -> str:
        rel = self.path.relative_to(REPO_ROOT) if self.path.is_absolute() else self.path
        return f"{rel}:{self.line}: [{self.code}] {self.message}"


@dataclass
class LogicDoc:
    path: Path
    text: str
    lines: list[str] = field(init=False)

    def __post_init__(self) -> None:
        self.lines = self.text.splitlines()


# ---------------------------------------------------------------------------
# Checks
# ---------------------------------------------------------------------------


def check_file_header(doc: LogicDoc) -> list[LintIssue]:
    """Check #1: File begins with the required fidelity/sources/cross-refs block."""
    issues: list[LintIssue] = []
    head = "\n".join(doc.lines[:10])
    if not HEADER_FIDELITY_RE.search(head):
        issues.append(LintIssue(
            doc.path, 1, "H001",
            "missing '> Fidelity: behavioral' header line in first 10 lines"))
    if not HEADER_SOURCES_RE.search(head):
        issues.append(LintIssue(
            doc.path, 1, "H002",
            "missing 'Source files:' header line in first 10 lines"))
    if not HEADER_CROSSREF_RE.search(head):
        issues.append(LintIssue(
            doc.path, 1, "H003",
            "missing '> Cross-refs:' header line in first 10 lines"))
    return issues


ALL_CHECKS = [check_file_header]


# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------


def load_doc(path: Path) -> LogicDoc:
    return LogicDoc(path=path, text=path.read_text(encoding="utf-8"))


def collect_targets(file_arg: Path | None) -> list[Path]:
    if file_arg is not None:
        return [file_arg]
    if not LOGIC_DIR.exists():
        return []
    return sorted(
        p for p in LOGIC_DIR.glob("*.md")
        if p.name not in {"README.md", "STYLE.md", "SYMBOLS.md"}
    )


def lint_files(paths: Iterable[Path]) -> list[LintIssue]:
    issues: list[LintIssue] = []
    for path in paths:
        doc = load_doc(path)
        for check in ALL_CHECKS:
            issues.extend(check(doc))
    return issues


def write_report(issues: list[LintIssue], targets: list[Path]) -> None:
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    lines = [
        f"# lint_logic.py report",
        f"# targets: {len(targets)} file(s)",
        f"# issues:  {len(issues)}",
        "",
    ]
    for issue in issues:
        lines.append(issue.format())
    if not issues:
        lines.append("OK — no issues.")
    RESULTS_FILE.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="lint_logic",
        description="Lint strict pseudo-code in docs/logic/*.md.",
    )
    parser.add_argument("--file", type=Path, default=None)
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args(argv)

    targets = collect_targets(args.file)
    issues = lint_files(targets)

    for issue in issues:
        print(issue.format(), file=sys.stderr)

    if args.file is None:
        write_report(issues, targets)

    if args.verbose:
        print(f"Scanned {len(targets)} file(s); {len(issues)} issue(s).")

    return 0 if not issues else 1


if __name__ == "__main__":
    sys.exit(main())
