#!/usr/bin/env python3
"""Validate source citations in documentation files.

Scans docs/**/*.md for backtick-wrapped citations like `file.c:LINE` or
`file.c:START-END`, then verifies each referenced file exists and the
line numbers are within range.

Usage:
    python tools/validate_citations.py [--file docs/RESEARCH.md] [--verbose]
"""

import argparse
import glob
import os
import re
import sys
from datetime import date

# Citation patterns:
#   `file.ext:LINE`        -> single line
#   `file.ext:START-END`   -> line range
CITATION_RE = re.compile(
    r'`([A-Za-z]\w*\.\w+):(\d+)(?:-(\d+))?`'
)

# Source file extensions we expect citations to reference
SOURCE_EXTENSIONS = {'.c', '.asm', '.h', '.i', '.p'}

# Repo root is parent of tools/
REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def find_source_files():
    """Build a map of filename -> (full_path, line_count) for all source files."""
    sources = {}
    for entry in os.listdir(REPO_ROOT):
        full = os.path.join(REPO_ROOT, entry)
        if not os.path.isfile(full):
            continue
        _, ext = os.path.splitext(entry)
        if ext.lower() in SOURCE_EXTENSIONS:
            with open(full, 'r', errors='replace') as f:
                line_count = sum(1 for _ in f)
            sources[entry] = (full, line_count)
    return sources


def find_doc_files(specific_file=None):
    """Return list of documentation markdown files to scan."""
    if specific_file:
        return [os.path.join(REPO_ROOT, specific_file)]
    docs_dir = os.path.join(REPO_ROOT, 'docs')
    return sorted(glob.glob(os.path.join(docs_dir, '**', '*.md'), recursive=True))


def extract_citations(filepath):
    """Extract all citations from a markdown file.

    Returns list of (line_num_in_doc, filename, start_line, end_line_or_None, raw_text).
    """
    citations = []
    with open(filepath, 'r', errors='replace') as f:
        for doc_line_num, line in enumerate(f, 1):
            for m in CITATION_RE.finditer(line):
                filename = m.group(1)
                start = int(m.group(2))
                end = int(m.group(3)) if m.group(3) else None
                citations.append((doc_line_num, filename, start, end, m.group(0)))
    return citations


def validate(doc_files, sources, verbose=False):
    """Validate all citations. Returns (pass_count, fail_count, failures)."""
    passes = 0
    failures = []

    for doc_path in doc_files:
        rel_doc = os.path.relpath(doc_path, REPO_ROOT)
        citations = extract_citations(doc_path)

        for doc_line, filename, start, end, raw in citations:
            if filename not in sources:
                # Check case-insensitive match
                matches = [k for k in sources if k.lower() == filename.lower()]
                if matches:
                    failures.append((rel_doc, doc_line, raw,
                                     f"case mismatch: found '{matches[0]}' but cited '{filename}'"))
                else:
                    failures.append((rel_doc, doc_line, raw,
                                     f"file '{filename}' not found in source tree"))
                continue

            _, line_count = sources[filename]
            ref_end = end if end else start

            if start < 1:
                failures.append((rel_doc, doc_line, raw,
                                 f"line {start} < 1 (invalid)"))
            elif ref_end > line_count:
                failures.append((rel_doc, doc_line, raw,
                                 f"line {ref_end} exceeds {filename} which has {line_count} lines"))
            elif end and start > end:
                failures.append((rel_doc, doc_line, raw,
                                 f"range reversed: {start} > {end}"))
            else:
                passes += 1
                if verbose:
                    print(f"  PASS  {rel_doc}:{doc_line}  {raw}")

    return passes, len(failures), failures


def write_results(passes, fail_count, failures, doc_files):
    """Write structured results to tools/results/."""
    results_dir = os.path.join(REPO_ROOT, 'tools', 'results')
    os.makedirs(results_dir, exist_ok=True)
    out_path = os.path.join(results_dir, 'validate_citations.txt')

    status = 'PASS' if fail_count == 0 else 'FAIL'
    scanned = ', '.join(os.path.relpath(f, REPO_ROOT) for f in doc_files)

    with open(out_path, 'w') as f:
        f.write(f"Experiment: validate_citations\n")
        f.write(f"Date: {date.today().isoformat()}\n")
        f.write(f"Command: python tools/validate_citations.py\n")
        f.write(f"Status: {status}\n")
        f.write(f"\nScanned: {scanned}\n")
        f.write(f"\nFindings:\n")
        f.write(f"- {passes} citations valid\n")
        f.write(f"- {fail_count} citations invalid\n")

        if failures:
            f.write(f"\nDetails:\n")
            for doc, line, raw, reason in failures:
                f.write(f"  {doc}:{line}  {raw}  -- {reason}\n")

    return out_path


def main():
    parser = argparse.ArgumentParser(description='Validate source citations in documentation')
    parser.add_argument('--file', help='Specific doc file to check (relative to repo root)')
    parser.add_argument('--verbose', '-v', action='store_true', help='Show passing citations too')
    parser.add_argument('--no-results', action='store_true', help='Skip writing results file')
    args = parser.parse_args()

    sources = find_source_files()
    doc_files = find_doc_files(args.file)

    if not doc_files:
        print("No documentation files found to scan.")
        sys.exit(1)

    print(f"Scanning {len(doc_files)} doc file(s) against {len(sources)} source files...\n")

    passes, fail_count, failures = validate(doc_files, sources, args.verbose)

    # Summary
    total = passes + fail_count
    print(f"\nResults: {passes}/{total} citations valid")

    if failures:
        print(f"\nFailed citations ({fail_count}):")
        for doc, line, raw, reason in failures:
            print(f"  {doc}:{line}  {raw}  -- {reason}")

    if not args.no_results:
        out_path = write_results(passes, fail_count, failures, doc_files)
        print(f"\nResults written to: {os.path.relpath(out_path, REPO_ROOT)}")

    sys.exit(0 if fail_count == 0 else 1)


if __name__ == '__main__':
    main()
