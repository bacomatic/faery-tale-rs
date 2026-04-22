#!/usr/bin/env python3
"""Check markdown links in reference/ for validity.

Focuses on cross-document links within reference/.
Reports: broken file links, broken anchor links.
"""
import os
import re
import sys
from pathlib import Path
from urllib.parse import unquote

REPO_ROOT = Path(__file__).resolve().parent.parent
REF_DIR = REPO_ROOT / "reference"

# [text](target) — non-greedy, skip images ![...]
LINK_RE = re.compile(r'(?<!\!)\[([^\]]+)\]\(([^)]+)\)')
HEADING_RE = re.compile(r'^(#{1,6})\s+(.+?)\s*$')
# Explicit anchor {#custom-id}
EXPLICIT_ANCHOR_RE = re.compile(r'\{#([a-zA-Z0-9_\-]+)\}\s*$')


def slugify(text: str) -> str:
    """GitHub-style slug: lowercase, strip punctuation (keep - _), spaces→-."""
    text = text.strip().lower()
    # Remove backticks and other markdown
    text = re.sub(r'`', '', text)
    # Strip markdown link syntax from heading
    text = re.sub(r'\[([^\]]+)\]\([^)]+\)', r'\1', text)
    # Keep alnum (word chars), space, hyphen. GitHub removes other punctuation including '.'
    text = re.sub(r'[^\w\- ]', '', text)
    # Replace each space with hyphen (do NOT collapse — GitHub preserves run length)
    text = text.replace(' ', '-')
    return text


def extract_anchors(md_path: Path) -> set:
    anchors = set()
    try:
        lines = md_path.read_text(encoding='utf-8', errors='replace').splitlines()
    except Exception:
        return anchors
    in_code = False
    for line in lines:
        stripped = line.strip()
        if stripped.startswith('```'):
            in_code = not in_code
            continue
        if in_code:
            continue
        m = HEADING_RE.match(line)
        if m:
            heading_text = m.group(2)
            # Check for explicit anchor
            explicit = EXPLICIT_ANCHOR_RE.search(heading_text)
            if explicit:
                anchors.add(explicit.group(1))
                heading_text = EXPLICIT_ANCHOR_RE.sub('', heading_text).strip()
            anchors.add(slugify(heading_text))
        # Also collect HTML anchors <a id="..."> or <a name="...">
        for m in re.finditer(r'<a\s+(?:id|name)="([^"]+)"', line):
            anchors.add(m.group(1))
    return anchors


def main():
    md_files = sorted(REF_DIR.rglob('*.md'))
    # Build anchor index for all md files in repo (for cross-refs outside reference/)
    anchor_index = {}
    for mf in md_files:
        anchor_index[mf.resolve()] = extract_anchors(mf)

    errors = []  # (severity, source, link_text, target, reason)

    for src in md_files:
        try:
            content = src.read_text(encoding='utf-8', errors='replace')
        except Exception as e:
            errors.append(('READ', src, '', '', str(e)))
            continue
        # Strip fenced code blocks (line-based, handles inline ``` in prose)
        out_lines = []
        in_fence = False
        for line in content.splitlines():
            if re.match(r'^\s{0,3}```', line):
                in_fence = not in_fence
                continue
            if in_fence:
                continue
            out_lines.append(line)
        content_no_code = '\n'.join(out_lines)
        # Strip inline code
        content_no_code = re.sub(r'`[^`\n]*`', '', content_no_code)

        for m in LINK_RE.finditer(content_no_code):
            text = m.group(1)
            target = m.group(2).strip()
            # Strip title: [text](url "title")
            target = re.split(r'\s+', target, 1)[0]
            if not target:
                continue
            # Skip external and special schemes
            if re.match(r'^(https?|ftp|mailto|tel|file|vscode|javascript):', target):
                continue
            if target.startswith('#'):
                # Same-file anchor
                anchor = unquote(target[1:])
                if anchor and anchor not in anchor_index.get(src.resolve(), set()):
                    errors.append(('ANCHOR', src, text, target, f'anchor not found in {src.name}'))
                continue

            # Split path#anchor
            if '#' in target:
                path_part, anchor = target.split('#', 1)
            else:
                path_part, anchor = target, ''
            path_part = unquote(path_part)
            anchor = unquote(anchor)

            # Resolve path relative to source file's directory
            resolved = (src.parent / path_part).resolve()
            if not resolved.exists():
                errors.append(('FILE', src, text, target, f'file not found: {resolved.relative_to(REPO_ROOT) if REPO_ROOT in resolved.parents else resolved}'))
                continue

            if anchor and resolved.suffix == '.md':
                anchors = anchor_index.get(resolved)
                if anchors is None:
                    anchors = extract_anchors(resolved)
                    anchor_index[resolved] = anchors
                if anchor not in anchors:
                    rel = resolved.relative_to(REPO_ROOT)
                    errors.append(('ANCHOR', src, text, target, f'anchor "#{anchor}" not found in {rel}'))

    # Report
    cross_ref_errors = [e for e in errors if e[0] in ('FILE', 'ANCHOR')]
    # Classify cross-document within reference/
    def is_cross_ref_within_reference(err):
        sev, src, text, target, reason = err
        if target.startswith('#'):
            return False  # same-file
        path_part = target.split('#', 1)[0]
        if not path_part:
            return False
        return True

    if not errors:
        print("All links valid.")
        return 0

    # Sort by source
    errors.sort(key=lambda e: (str(e[1]), e[3]))
    current_src = None
    for sev, src, text, target, reason in errors:
        rel_src = src.relative_to(REPO_ROOT)
        if src != current_src:
            print(f"\n{rel_src}")
            current_src = src
        print(f"  [{sev}] [{text}]({target}) — {reason}")

    print(f"\nTotal issues: {len(errors)}")
    return 1 if errors else 0


if __name__ == '__main__':
    sys.exit(main())
