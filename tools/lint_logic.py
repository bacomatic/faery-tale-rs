#!/usr/bin/env python3
"""Lint strict pseudo-code in docs/logic/*.md.

Usage:
    python tools/lint_logic.py [--verbose] [--file PATH]

Exit 0 on clean lint. Non-zero on any failure. Writes a report to
tools/results/lint_logic.txt when invoked without --file.
"""
from __future__ import annotations

import argparse
import ast
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

CITATION_RE = re.compile(r"`([A-Za-z][\w]*\.(?:c|asm|h|i|p)):(\d+)(?:-(\d+))?`")
SOURCE_EXTS = {".c", ".asm", ".h", ".i", ".p"}


def _source_line_counts() -> dict[str, int]:
    counts: dict[str, int] = {}
    for entry in REPO_ROOT.iterdir():
        if entry.is_file() and entry.suffix.lower() in SOURCE_EXTS:
            with entry.open("r", errors="replace") as fh:
                counts[entry.name] = sum(1 for _ in fh)
    return counts


_SOURCE_COUNTS_CACHE: dict[str, int] | None = None


def source_line_counts() -> dict[str, int]:
    global _SOURCE_COUNTS_CACHE
    if _SOURCE_COUNTS_CACHE is None:
        _SOURCE_COUNTS_CACHE = _source_line_counts()
    return _SOURCE_COUNTS_CACHE

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


def check_citations(doc: LogicDoc) -> list[LintIssue]:
    """Check #3: every `file.ext:LINE` citation resolves inside the repo."""
    issues: list[LintIssue] = []
    counts = source_line_counts()
    for lineno, raw in enumerate(doc.lines, 1):
        for m in CITATION_RE.finditer(raw):
            filename = m.group(1)
            start = int(m.group(2))
            end = int(m.group(3)) if m.group(3) else start
            if filename not in counts:
                issues.append(LintIssue(
                    doc.path, lineno, "C001",
                    f"unknown source file '{filename}' in citation {m.group(0)}"))
                continue
            if start < 1 or end > counts[filename] or end < start:
                issues.append(LintIssue(
                    doc.path, lineno, "C002",
                    f"line range out of bounds in {m.group(0)} "
                    f"({filename} has {counts[filename]} lines)"))
    return issues


ALL_CHECKS = [check_file_header, check_function_headers, check_citations]


PRIMITIVES: set[str] = {
    "rand", "chance", "min", "max", "clamp", "abs", "sign",
    "bit", "wrap_u8", "wrap_i16", "wrap_u16",
    "now_ticks", "speak", "play_sound", "play_music",
}

# Built-in type names used in annotations — always resolvable.
BUILTIN_TYPES: set[str] = {
    "int", "str", "bool", "float", "bytes", "list", "dict", "set", "tuple",
    "u8", "u16", "u32", "i8", "i16", "i32",
}


def _preprocess_pseudo(body: str) -> str:
    """Strip Markdown comments from pseudo blocks before AST parsing."""
    return body


def _parse_pseudo(entry: "FunctionEntry", doc: LogicDoc) -> tuple[ast.Module | None, list[LintIssue]]:
    issues: list[LintIssue] = []
    try:
        tree = ast.parse(_preprocess_pseudo(entry.pseudo_body), mode="exec")
    except SyntaxError as exc:
        issues.append(LintIssue(
            doc.path,
            entry.pseudo_start + (exc.lineno or 1) - 1,
            "P001",
            f"pseudo block for '{entry.name}' has syntax error: {exc.msg}"))
        return None, issues
    return tree, issues


def check_pseudo_parses(doc: LogicDoc) -> list[LintIssue]:
    """Check #4: every pseudo block parses."""
    issues: list[LintIssue] = []
    entries, _ = extract_function_entries(doc)
    for entry in entries:
        _, errs = _parse_pseudo(entry, doc)
        issues.extend(errs)
    return issues


def _check_signature(entry: "FunctionEntry", tree: ast.Module, doc: LogicDoc) -> list[LintIssue]:
    """Check #5: single top-level def with annotated args, return type, and docstring."""
    issues: list[LintIssue] = []
    if len(tree.body) != 1 or not isinstance(tree.body[0], ast.FunctionDef):
        issues.append(LintIssue(
            doc.path, entry.pseudo_start, "S001",
            f"pseudo block for '{entry.name}' must contain exactly one top-level def"))
        return issues
    func = tree.body[0]
    if func.name != entry.name:
        issues.append(LintIssue(
            doc.path, entry.pseudo_start, "S002",
            f"function name '{func.name}' does not match H2 '{entry.name}'"))
    if func.returns is None:
        issues.append(LintIssue(
            doc.path, entry.pseudo_start, "S003",
            f"function '{entry.name}' is missing a return annotation"))
    for arg in list(func.args.args) + list(func.args.kwonlyargs):
        if arg.annotation is None:
            issues.append(LintIssue(
                doc.path, entry.pseudo_start, "S004",
                f"argument '{arg.arg}' of '{entry.name}' is missing a type annotation"))
    # Docstring
    first = func.body[0] if func.body else None
    if not (isinstance(first, ast.Expr) and isinstance(first.value, ast.Constant)
            and isinstance(first.value.value, str)):
        issues.append(LintIssue(
            doc.path, entry.pseudo_start, "S005",
            f"function '{entry.name}' must begin with a docstring"))
    return issues


def check_function_signature(doc: LogicDoc) -> list[LintIssue]:
    issues: list[LintIssue] = []
    entries, _ = extract_function_entries(doc)
    for entry in entries:
        tree, _ = _parse_pseudo(entry, doc)
        if tree is None:
            continue
        issues.extend(_check_signature(entry, tree, doc))
    return issues


FORBIDDEN_NODES: list[tuple[type, str]] = [
    (ast.Try, "try"),
    (ast.Raise, "raise"),
    (ast.With, "with"),
    (ast.Lambda, "lambda"),
    (ast.ClassDef, "class"),
    (ast.Import, "import"),
    (ast.ImportFrom, "import"),
    (ast.Global, "global"),
    (ast.Nonlocal, "nonlocal"),
    (ast.ListComp, "list comprehension"),
    (ast.SetComp, "set comprehension"),
    (ast.DictComp, "dict comprehension"),
    (ast.GeneratorExp, "generator expression"),
]


def check_forbidden_constructs(doc: LogicDoc) -> list[LintIssue]:
    """Check #6."""
    issues: list[LintIssue] = []
    entries, _ = extract_function_entries(doc)
    for entry in entries:
        tree, _ = _parse_pseudo(entry, doc)
        if tree is None:
            continue
        for node in ast.walk(tree):
            for node_type, label in FORBIDDEN_NODES:
                if isinstance(node, node_type):
                    issues.append(LintIssue(
                        doc.path,
                        entry.pseudo_start + getattr(node, "lineno", 1) - 1,
                        "F010",
                        f"forbidden construct '{label}' in function '{entry.name}'"))
                    break
    return issues


ALL_CHECKS = [
    check_file_header,
    check_function_headers,
    check_citations,
    check_pseudo_parses,
    check_function_signature,
    check_forbidden_constructs,
]


SYMBOLS_FILE = LOGIC_DIR / "SYMBOLS.md"

# Identifier-assignment inside SYMBOLS.md ```pseudo blocks, e.g.
#   MAXSHAPES = 25
#   DIR_NW = 0
#   struct Shape:
_SYMBOLS_ASSIGN_RE = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)\s*[:=]")
_SYMBOLS_STRUCT_RE = re.compile(r"^struct\s+([A-Za-z_][A-Za-z0-9_]*)\s*:")
_SYMBOLS_TABLE_RE = re.compile(r"`TABLE:([A-Za-z_][\w]*)`")


def load_symbol_registry() -> set[str]:
    """Return the set of identifiers declared in SYMBOLS.md."""
    names: set[str] = set()
    if not SYMBOLS_FILE.exists():
        return names
    text = SYMBOLS_FILE.read_text(encoding="utf-8")
    # All fenced ``` blocks (any language) are treated as declaration zones.
    in_fence = False
    for raw in text.splitlines():
        if raw.startswith("```"):
            in_fence = not in_fence
            continue
        if not in_fence:
            continue
        stripped = raw.strip()
        if not stripped or stripped.startswith("#"):
            continue
        m_struct = _SYMBOLS_STRUCT_RE.match(stripped)
        if m_struct:
            names.add(m_struct.group(1))
            continue
        m_assign = _SYMBOLS_ASSIGN_RE.match(stripped)
        if m_assign:
            names.add(m_assign.group(1))
            continue
    # Table names from the markdown table.
    for m in _SYMBOLS_TABLE_RE.finditer(text):
        names.add(f"TABLE:{m.group(1)}")
    return names


def _parse_calls_list(calls_line: str) -> set[str]:
    body = calls_line.split(":", 1)[1].strip() if ":" in calls_line else ""
    if body.lower() in ("", "none"):
        return set()
    names: set[str] = set()
    for token in body.split(","):
        token = token.strip().strip("`")
        if not token:
            continue
        names.add(token)
    return names


def check_symbol_resolution(doc: LogicDoc) -> list[LintIssue]:
    """Check #7: every referenced name resolves."""
    issues: list[LintIssue] = []
    entries, _ = extract_function_entries(doc)
    registered = load_symbol_registry()
    for entry in entries:
        tree, _ = _parse_pseudo(entry, doc)
        if tree is None:
            continue
        func = tree.body[0] if tree.body and isinstance(tree.body[0], ast.FunctionDef) else None
        if func is None:
            continue
        locals_: set[str] = {arg.arg for arg in func.args.args}
        locals_.update(arg.arg for arg in func.args.kwonlyargs)
        called = _parse_calls_list(entry.calls_text)
        for node in ast.walk(func):
            if isinstance(node, ast.Assign):
                for target in node.targets:
                    if isinstance(target, ast.Name):
                        locals_.add(target.id)
            elif isinstance(node, ast.AugAssign) and isinstance(node.target, ast.Name):
                locals_.add(node.target.id)
            elif isinstance(node, ast.For) and isinstance(node.target, ast.Name):
                locals_.add(node.target.id)
        for node in ast.walk(func):
            if isinstance(node, ast.Name) and isinstance(node.ctx, ast.Load):
                nm = node.id
                if nm in locals_ or nm in called or nm in registered or nm in PRIMITIVES:
                    continue
                if nm in BUILTIN_TYPES:
                    continue
                if nm in {"True", "False", "None"}:
                    continue
                issues.append(LintIssue(
                    doc.path,
                    entry.pseudo_start + node.lineno - 1,
                    "N001",
                    f"unresolved symbol '{nm}' in function '{entry.name}'"))
    return issues


ALL_CHECKS.append(check_symbol_resolution)


_CALLS_TABLE_RE = re.compile(r"TABLE:([A-Za-z_][\w]*)")


def check_table_refs(doc: LogicDoc) -> list[LintIssue]:
    """Check #8: every TABLE:name reference (in Calls: lines or pseudo bodies) is registered."""
    issues: list[LintIssue] = []
    registered = load_symbol_registry()
    entries, _ = extract_function_entries(doc)
    for entry in entries:
        search_targets = [
            (entry.calls_text, entry.h2_line),
            (entry.pseudo_body, entry.pseudo_start),
        ]
        for text, anchor in search_targets:
            for m in _CALLS_TABLE_RE.finditer(text):
                name = f"TABLE:{m.group(1)}"
                if name not in registered:
                    issues.append(LintIssue(
                        doc.path, anchor, "T001",
                        f"unregistered table reference '{name}' in function '{entry.name}'"))
    return issues


ALL_CHECKS.append(check_table_refs)


ALLOWED_LITERALS = {-1, 0, 1, 2}


def check_magic_numbers(doc: LogicDoc) -> list[LintIssue]:
    """Check #9."""
    issues: list[LintIssue] = []
    registered = load_symbol_registry()
    entries, _ = extract_function_entries(doc)
    body_lines = doc.lines
    for entry in entries:
        tree, _ = _parse_pseudo(entry, doc)
        if tree is None:
            continue
        for node in ast.walk(tree):
            if isinstance(node, ast.Constant) and isinstance(node.value, int):
                val = node.value
                if val in ALLOWED_LITERALS:
                    continue
                # Skip values inside "bit(...)" primitive — those are bit indices, harmless.
                parent = getattr(node, "parent", None)
                # We didn't annotate parents; just check the source line for 'bit(' prefix.
                doc_line_idx = entry.pseudo_start + node.lineno - 1
                line_src = body_lines[doc_line_idx] if 0 <= doc_line_idx < len(body_lines) else ""
                if "bit(" in line_src:
                    continue
                if "#" in line_src:
                    continue  # inline comment present — accepted
                # Otherwise require the literal to appear as a registered constant name nearby.
                issues.append(LintIssue(
                    doc.path, doc_line_idx + 1, "M001",
                    f"magic number {val} in '{entry.name}' needs a named constant or inline # comment"))
    return issues


_MD_LINK_RE = re.compile(r"\[[^\]]+\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")


def check_crossrefs(doc: LogicDoc) -> list[LintIssue]:
    """Check #10."""
    issues: list[LintIssue] = []
    for lineno, raw in enumerate(doc.lines, 1):
        for m in _MD_LINK_RE.finditer(raw):
            target = m.group(1).split("#", 1)[0]
            if not target or target.startswith(("http://", "https://", "mailto:")):
                continue
            resolved = (doc.path.parent / target).resolve()
            try:
                resolved.relative_to(REPO_ROOT)
            except ValueError:
                continue  # outside repo; skip
            if not resolved.exists():
                issues.append(LintIssue(
                    doc.path, lineno, "X001",
                    f"broken cross-reference to '{target}'"))
    return issues


_MERMAID_BLOCK_RE = re.compile(r"```mermaid\s*(.*?)```", re.DOTALL)
_STATE_ASSIGN_RE = re.compile(r"\.\w+\s*=\s*(STATE_[A-Z_0-9]+|GOAL_[A-Z_0-9]+|CMODE_[A-Z_0-9]+)")


def check_state_coverage(doc: LogicDoc) -> list[LintIssue]:
    """Check #12: when a Mermaid stateDiagram-v2 follows a function, every
    STATE_* / GOAL_* / CMODE_* assigned in the pseudo block appears in the diagram."""
    issues: list[LintIssue] = []
    entries, _ = extract_function_entries(doc)
    text_after = doc.text
    for entry in entries:
        # Find a mermaid block that appears after this entry's H2 but before the next H2.
        tail = "\n".join(doc.lines[entry.pseudo_end:])
        next_h2 = H2_RE.search(tail, re.MULTILINE) if False else None  # placeholder
        mer = _MERMAID_BLOCK_RE.search(tail)
        if not mer:
            continue
        diagram = mer.group(1)
        if "stateDiagram-v2" not in diagram:
            continue
        assigned = set(_STATE_ASSIGN_RE.findall(entry.pseudo_body))
        for state in assigned:
            if state not in diagram:
                issues.append(LintIssue(
                    doc.path, entry.h2_line, "D001",
                    f"state '{state}' assigned in '{entry.name}' but missing from diagram"))
    return issues


_README_ROW_RE = re.compile(r"\|\s*`?([A-Za-z_][\w]*)`?\s*\|\s*\[[^\]]+\]\(([^)]+)\)\s*\|")


def check_index_completeness(logic_dir: Path, docs: list[LogicDoc]) -> list[LintIssue]:
    """Check #11: README index matches the set of defined functions."""
    readme = logic_dir / "README.md"
    issues: list[LintIssue] = []
    if not readme.exists():
        issues.append(LintIssue(readme, 1, "I001", "docs/logic/README.md is missing"))
        return issues
    readme_text = readme.read_text(encoding="utf-8")

    indexed: dict[str, str] = {}
    for m in _README_ROW_RE.finditer(readme_text):
        indexed[m.group(1)] = m.group(2)

    defined: set[str] = set()
    for doc in docs:
        entries, _ = extract_function_entries(doc)
        for entry in entries:
            defined.add(entry.name)

    for name in defined - set(indexed):
        issues.append(LintIssue(readme, 1, "I002", f"function '{name}' is defined but not in index"))
    for name, target in indexed.items():
        target_path = (readme.parent / target.split("#", 1)[0]).resolve()
        if not target_path.exists():
            issues.append(LintIssue(
                readme, 1, "I003",
                f"index row '{name}' points at missing file '{target}'"))
        if name not in defined:
            issues.append(LintIssue(readme, 1, "I004", f"index row '{name}' has no matching function"))
    return issues


ALL_CHECKS.extend([check_magic_numbers, check_crossrefs, check_state_coverage])


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
    parser.add_argument("--logic-dir", type=Path, default=LOGIC_DIR)
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args(argv)

    logic_dir = args.logic_dir
    if args.file is not None:
        targets = [args.file]
    else:
        targets = sorted(
            p for p in logic_dir.glob("*.md")
            if p.name not in {"README.md", "STYLE.md", "SYMBOLS.md"}
        )

    docs = [load_doc(p) for p in targets]
    issues: list[LintIssue] = []
    for doc in docs:
        for check in ALL_CHECKS:
            issues.extend(check(doc))
    if args.file is None:
        issues.extend(check_index_completeness(logic_dir, docs))

    for issue in issues:
        print(issue.format(), file=sys.stderr)

    if args.file is None:
        write_report(issues, targets)

    if args.verbose:
        print(f"Scanned {len(targets)} file(s); {len(issues)} issue(s).")

    return 0 if not issues else 1


if __name__ == "__main__":
    sys.exit(main())
