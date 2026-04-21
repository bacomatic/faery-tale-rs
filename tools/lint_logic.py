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


# Section header used in logic docs; everything else at H2 is a function entry.
RESERVED_H2 = {"Overview", "Symbols", "Notes", "Mermaid"}

H2_RE = re.compile(r"^##\s+(?P<name>\S+)\s*$")
SOURCE_LINE_RE = re.compile(r"^Source:\s+`")
CALLED_BY_LINE_RE = re.compile(r"^Called by:\s+")
CALLS_LINE_RE = re.compile(r"^Calls:\s+")
PSEUDO_FENCE_RE = re.compile(r"^```pseudo\s*$")


@dataclass
class FunctionEntry:
    name: str
    h2_line: int                # 1-based line of the "## name" header
    source_text: str
    called_by_text: str
    calls_text: str
    pseudo_start: int           # 1-based line of the opening ```pseudo fence
    pseudo_end: int             # 1-based line of the closing ``` fence
    pseudo_body: str            # Content between the fences (exclusive)


def extract_function_entries(doc: LogicDoc) -> tuple[list[FunctionEntry], list[LintIssue]]:
    """Locate every ## <name> block and parse its header + fenced body."""
    issues: list[LintIssue] = []
    entries: list[FunctionEntry] = []
    i = 0
    n = len(doc.lines)
    while i < n:
        m = H2_RE.match(doc.lines[i])
        if not m:
            i += 1
            continue
        name = m.group("name")
        h2_line = i + 1
        if name in RESERVED_H2:
            i += 1
            continue
        # Collect the 3 header lines (may have blank lines between).
        source_text = called_by_text = calls_text = ""
        j = i + 1
        while j < n and not PSEUDO_FENCE_RE.match(doc.lines[j]) and not H2_RE.match(doc.lines[j]):
            line = doc.lines[j]
            if SOURCE_LINE_RE.match(line):
                source_text = line
            elif CALLED_BY_LINE_RE.match(line):
                called_by_text = line
            elif CALLS_LINE_RE.match(line):
                calls_text = line
            j += 1

        missing = []
        if not source_text:
            missing.append("Source")
        if not called_by_text:
            missing.append("Called by")
        if not calls_text:
            missing.append("Calls")
        if missing:
            issues.append(LintIssue(
                doc.path, h2_line, "F001",
                f"function '{name}' missing header line(s): {', '.join(missing)}"))

        if j >= n or not PSEUDO_FENCE_RE.match(doc.lines[j]):
            issues.append(LintIssue(
                doc.path, h2_line, "F002",
                f"function '{name}' has no ```pseudo fenced block before next section"))
            i = j
            continue

        pseudo_start = j + 1
        k = j + 1
        while k < n and not re.match(r"^```\s*$", doc.lines[k]):
            k += 1
        if k >= n:
            issues.append(LintIssue(
                doc.path, pseudo_start, "F003",
                f"function '{name}' pseudo block is not closed"))
            i = k
            continue
        pseudo_body = "\n".join(doc.lines[j + 1 : k])
        entries.append(FunctionEntry(
            name=name,
            h2_line=h2_line,
            source_text=source_text,
            called_by_text=called_by_text,
            calls_text=calls_text,
            pseudo_start=pseudo_start,
            pseudo_end=k + 1,
            pseudo_body=pseudo_body,
        ))
        i = k + 1
    return entries, issues


def check_function_headers(doc: LogicDoc) -> list[LintIssue]:
    """Check #2: every function entry has well-formed Source/Called by/Calls lines."""
    _, issues = extract_function_entries(doc)
    return issues


ALL_CHECKS = [check_file_header, check_function_headers]


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
