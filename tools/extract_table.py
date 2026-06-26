#!/usr/bin/env python3
"""Extract data tables from source files and display them.

Parses arrays, lookup tables, and constant definitions from the original
1987 source code. Supports both C arrays and 68000 assembly dc.b/dc.w/dc.l
directives.

Usage:
    python tools/extract_table.py <source_file> <label1> [label2 ...]
    python tools/extract_table.py fsubs.asm xdir ydir
    python tools/extract_table.py fsubs.asm com2
    python tools/extract_table.py fmain.c diroffs
    python tools/extract_table.py --list <source_file>
"""

import argparse
import os
import re
import sys
from datetime import date

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Standard C single-char escape sequences -> code point.
C_ESCAPES = {
    'a': 7, 'b': 8, 'f': 12, 'n': 10, 'r': 13, 't': 9, 'v': 11,
    '0': 0, '\\': 92, "'": 39, '"': 34, '?': 63,
}


def parse_c_value(token):
    """Parse a single C scalar literal: hex, decimal, char, or fall back to str."""
    t = token.strip()
    if not t:
        return None
    # Character literal: 'A', '\n', '\x1b', '\033'
    if len(t) >= 3 and t[0] == "'" and t[-1] == "'":
        body = t[1:-1]
        if body.startswith('\\'):
            esc = body[1:]
            if esc and esc[0] in ('x', 'X'):
                return int(esc[1:], 16)
            if esc and esc[0].isdigit():
                return int(esc, 8)
            if len(esc) == 1 and esc in C_ESCAPES:
                return C_ESCAPES[esc]
            return ord(esc[0]) if esc else 0
        return ord(body[0])
    # Numeric literal (hex with optional sign, or decimal/negative).
    try:
        if 'x' in t.lower():
            return int(t, 16)
        return int(t, 10)
    except ValueError:
        return t


def _split_top_level(text):
    """Split *text* on commas at brace depth 0, respecting char literals."""
    items = []
    buf = []
    depth = 0
    i = 0
    n = len(text)
    while i < n:
        c = text[i]
        if c == "'":  # char literal — copy verbatim incl. escaped chars
            buf.append(c)
            i += 1
            while i < n:
                buf.append(text[i])
                if text[i] == '\\' and i + 1 < n:
                    buf.append(text[i + 1])
                    i += 2
                    continue
                if text[i] == "'":
                    i += 1
                    break
                i += 1
            continue
        if c == '{':
            depth += 1
            buf.append(c)
        elif c == '}':
            depth -= 1
            buf.append(c)
        elif c == ',' and depth == 0:
            items.append(''.join(buf))
            buf = []
        else:
            buf.append(c)
        i += 1
    if ''.join(buf).strip():
        items.append(''.join(buf))
    return items


def parse_c_initializer(text):
    """Recursively parse a brace-delimited C initializer into nested ints.

    A ``{...}`` group becomes a list; scalar tokens become parsed values.
    Handles arbitrary nesting (1-D and N-D) and hex/decimal/char literals.
    """
    t = text.strip()
    if t.startswith('{') and t.endswith('}'):
        inner = t[1:-1]
        return [parse_c_initializer(p) for p in _split_top_level(inner) if p.strip()]
    return parse_c_value(t)

# Assembly data directives: label dc.b/dc.w/dc.l values
ASM_LABEL_RE = re.compile(r'^(\w+)\s+dc\.(b|w|l)\s+(.+)', re.IGNORECASE)
ASM_CONT_RE = re.compile(r'^\s+dc\.(b|w|l)\s+(.+)', re.IGNORECASE)

# C array initializer: type name[] = { ... };  (may span multiple lines)
C_ARRAY_START_RE = re.compile(
    r'(?:(?:static|extern|unsigned|signed|short|long|int|char|BYTE|UBYTE|WORD|UWORD|LONG|ULONG|USHORT)\s+)+'
    r'(\w+)\s*\[\s*\d*\s*\]\s*=\s*\{',
    re.IGNORECASE
)


def parse_asm_values(text):
    """Parse comma-separated assembly values, stripping comments."""
    # Remove comments (everything after ; or *)
    text = re.sub(r'[;*].*$', '', text).strip().rstrip(',')
    if not text:
        return []
    values = []
    for v in text.split(','):
        v = v.strip().strip("'\"")
        if not v:
            continue
        try:
            if v.startswith('$'):
                values.append(int(v[1:], 16))
            elif v.startswith('0x'):
                values.append(int(v, 16))
            else:
                values.append(int(v))
        except ValueError:
            values.append(v)  # Keep as string if not numeric
    return values


def extract_asm_tables(filepath):
    """Extract all labeled dc.b/dc.w/dc.l tables from an assembly file."""
    tables = {}
    current_label = None
    current_size = None
    current_values = []

    with open(filepath, 'r', errors='replace') as f:
        for line_num, line in enumerate(f, 1):
            # Check for labeled data directive
            m = ASM_LABEL_RE.match(line)
            if m:
                # Save previous table if any
                if current_label and current_values:
                    tables[current_label] = {
                        'type': f'dc.{current_size}',
                        'values': current_values,
                        'line': tables[current_label]['line']
                    }
                current_label = m.group(1)
                current_size = m.group(2).lower()
                current_values = parse_asm_values(m.group(3))
                tables[current_label] = {'type': f'dc.{current_size}', 'values': [], 'line': line_num}
                continue

            # Check for continuation line (dc.x without label)
            m = ASM_CONT_RE.match(line)
            if m and current_label:
                cont_size = m.group(1).lower()
                if cont_size == current_size:
                    current_values.extend(parse_asm_values(m.group(2)))
                    continue

            # Non-data line — finalize current table
            if current_label and current_values:
                tables[current_label] = {
                    'type': f'dc.{current_size}',
                    'values': current_values,
                    'line': tables[current_label]['line']
                }
                current_label = None
                current_values = []

    # Finalize last table
    if current_label and current_values:
        tables[current_label] = {
            'type': f'dc.{current_size}',
            'values': current_values,
            'line': tables[current_label]['line']
        }

    return tables


def extract_c_arrays(filepath):
    """Extract C array initializers from a C source file."""
    tables = {}

    with open(filepath, 'r', errors='replace') as f:
        content = f.read()
        lines = content.split('\n')

    for line_num, line in enumerate(lines, 1):
        m = C_ARRAY_START_RE.search(line)
        if not m:
            continue

        name = m.group(1)
        # Find the opening brace and collect everything to the closing brace
        start_pos = content.index(line)
        brace_start = content.index('{', start_pos)
        depth = 1
        pos = brace_start + 1
        while pos < len(content) and depth > 0:
            if content[pos] == '{':
                depth += 1
            elif content[pos] == '}':
                depth -= 1
            pos += 1

        inner = content[brace_start + 1:pos - 1]
        # Remove comments
        inner = re.sub(r'/\*.*?\*/', '', inner, flags=re.DOTALL)
        inner = re.sub(r'//.*$', '', inner, flags=re.MULTILINE)
        # Recursively parse into nested ints (preserves N-D structure).
        values = parse_c_initializer('{' + inner + '}')

        tables[name] = {
            'type': 'c_array',
            'values': values,
            'shape': _shape(values),
            'line': line_num,
        }

    return tables


def _shape(values):
    """Return the dimensions of a (possibly ragged) nested list as a tuple."""
    if not isinstance(values, list):
        return ()
    dims = [len(values)]
    if values and all(isinstance(v, list) for v in values):
        sub = [_shape(v) for v in values]
        if len(set(sub)) == 1 and sub[0]:
            dims.extend(sub[0])
    return tuple(dims)


def extract_tables(filepath):
    """Extract tables based on file type."""
    _, ext = os.path.splitext(filepath)
    if ext.lower() == '.asm':
        return extract_asm_tables(filepath)
    elif ext.lower() in ('.c', '.h'):
        return extract_c_arrays(filepath)
    else:
        print(f"Unsupported file type: {ext}")
        sys.exit(1)


def format_table(name, table):
    """Format a table for display."""
    lines = []
    lines.append(f"  Label: {name}")
    lines.append(f"  Type:  {table['type']}")
    lines.append(f"  Line:  {table['line']}")
    if 'shape' in table:
        lines.append(f"  Shape: {table['shape']}")
    lines.append(f"  Count: {len(table['values'])}")
    lines.append(f"  Values: {table['values']}")

    # For small numeric tables, show indexed view
    if len(table['values']) <= 32 and all(isinstance(v, int) for v in table['values']):
        lines.append(f"  Indexed:")
        for i, v in enumerate(table['values']):
            lines.append(f"    [{i}] = {v}")

    return '\n'.join(lines)


def write_results(source_file, labels, tables):
    """Write structured results to tools/results/."""
    results_dir = os.path.join(REPO_ROOT, 'tools', 'results')
    os.makedirs(results_dir, exist_ok=True)
    base = os.path.splitext(os.path.basename(source_file))[0]
    out_path = os.path.join(results_dir, f'extract_{base}_{"_".join(labels)}.txt')

    found = [l for l in labels if l in tables]
    missing = [l for l in labels if l not in tables]
    status = 'PASS' if not missing else ('PARTIAL' if found else 'FAIL')

    with open(out_path, 'w') as f:
        f.write(f"Experiment: extract_table\n")
        f.write(f"Date: {date.today().isoformat()}\n")
        f.write(f"Command: python tools/extract_table.py {source_file} {' '.join(labels)}\n")
        f.write(f"Status: {status}\n")
        f.write(f"\nFindings:\n")
        f.write(f"- {len(found)}/{len(labels)} labels found\n")
        if missing:
            f.write(f"- Missing: {', '.join(missing)}\n")
        f.write(f"\nDetails:\n")
        for label in found:
            f.write(f"\n{format_table(label, tables[label])}\n")

    return out_path


def main():
    parser = argparse.ArgumentParser(description='Extract data tables from source files')
    parser.add_argument('source', help='Source file to parse (relative to repo root)')
    parser.add_argument('labels', nargs='*', help='Table/array labels to extract')
    parser.add_argument('--list', action='store_true', help='List all tables found in the file')
    parser.add_argument('--no-results', action='store_true', help='Skip writing results file')
    parser.add_argument('--json', metavar='PATH',
                        help='Write extracted labels as deterministic JSON to PATH')
    args = parser.parse_args()

    filepath = os.path.join(REPO_ROOT, args.source)
    if not os.path.isfile(filepath):
        print(f"File not found: {args.source}")
        sys.exit(1)

    tables = extract_tables(filepath)

    if args.list:
        print(f"Tables found in {args.source} ({len(tables)}):\n")
        for name, table in sorted(tables.items(), key=lambda x: x[1]['line']):
            print(f"  {name:20s}  {table['type']}  line {table['line']:>5d}  ({len(table['values'])} values)")
        sys.exit(0)

    if not args.labels:
        parser.error("Specify label(s) to extract, or use --list to see available labels")

    found = []
    missing = []
    for label in args.labels:
        if label in tables:
            found.append(label)
            print(f"\n{format_table(label, tables[label])}")
        else:
            missing.append(label)
            print(f"\n  Label '{label}' not found in {args.source}")

    if args.json and found:
        from asset_common import write_json
        payload = {
            label: {
                'source': args.source,
                'line': tables[label]['line'],
                'shape': list(tables[label].get('shape', ())),
                'values': tables[label]['values'],
            }
            for label in found
        }
        write_json(args.json, payload)
        print(f"\nJSON written to: {args.json}")

    if not args.no_results:
        out_path = write_results(args.source, args.labels, tables)
        print(f"\nResults written to: {os.path.relpath(out_path, REPO_ROOT)}")

    if missing:
        sys.exit(1 if not found else 2)
    sys.exit(0)


if __name__ == '__main__':
    main()
