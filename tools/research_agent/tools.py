"""LangChain tools for the FTA research agent.

All file access is path-safety checked against repo_root.
Errors are returned as strings (not exceptions) so the model can recover.
"""
from __future__ import annotations

import re
import subprocess
from pathlib import Path

from langchain_core.tools import tool as lc_tool

# Allowed extensions for read_source_file
_SOURCE_EXTENSIONS = {".c", ".asm", ".h", ".i", ".p"}


def _safe_resolve(repo_root: Path, raw_path: str, must_be_under: Path | None = None) -> Path | str:
    """Resolve raw_path relative to repo_root.

    Returns the resolved Path if safe, or an error string if not.
    """
    try:
        resolved = (repo_root / raw_path).resolve()
    except Exception as exc:
        return f"Error: could not resolve path: {exc}"
    if not str(resolved).startswith(str(repo_root.resolve())):
        return "Error: path not allowed — must be within the repository"
    if must_be_under is not None:
        if not str(resolved).startswith(str(must_be_under.resolve())):
            return f"Error: path not allowed — must be under {must_be_under.relative_to(repo_root)}"
    return resolved


def make_tools(repo_root: Path) -> list:
    """Return LangChain tools bound to repo_root."""

    reference_dir = repo_root / "reference"

    @lc_tool
    def list_directory(path: str) -> str:
        """List files in a directory under reference/. path is relative to repo root."""
        resolved = _safe_resolve(repo_root, path, must_be_under=reference_dir)
        if isinstance(resolved, str):
            return resolved
        if not resolved.exists():
            return f"Error: path does not exist: {path}"
        if resolved.is_file():
            return resolved.name
        entries = sorted(p.name for p in resolved.iterdir())
        return "\n".join(entries) if entries else "(empty directory)"

    @lc_tool
    def read_file(path: str) -> str:
        """Read a file under reference/. path is relative to repo root."""
        resolved = _safe_resolve(repo_root, path, must_be_under=reference_dir)
        if isinstance(resolved, str):
            return resolved
        if not resolved.exists():
            return f"Error: file does not exist: {path}"
        if not resolved.is_file():
            return f"Error: not a file: {path}"
        try:
            return resolved.read_text(encoding="utf-8", errors="replace")
        except Exception as exc:
            return f"Error reading file: {exc}"

    @lc_tool
    def search_text(pattern: str, path: str = "reference") -> str:
        """Search reference docs for a regex pattern.

        path: file or directory under reference/ (relative to repo root).
              Defaults to all of reference/.
        Returns matching lines with file:line context, capped at 50 results.
        """
        resolved = _safe_resolve(repo_root, path, must_be_under=reference_dir)
        if isinstance(resolved, str):
            return resolved
        if not resolved.exists():
            return f"Error: path does not exist: {path}"
        try:
            result = subprocess.run(
                ["grep", "-rn", "--include=*.md", "--include=*.json", "-m", "50", pattern,
                 str(resolved)],
                capture_output=True,
                text=True,
                cwd=str(repo_root),
            )
        except FileNotFoundError:
            # grep not available — fall back to Python
            return _python_grep(pattern, resolved, repo_root, cap=50)
        if result.returncode == 1:
            return "No matches found."
        if result.returncode > 1:
            return f"Error running search: {result.stderr.strip()}"
        lines = result.stdout.strip().splitlines()
        # Make paths relative to repo_root for cleaner output
        relative_lines = []
        for line in lines:
            try:
                file_part, rest = line.split(":", 1)
                rel = Path(file_part).relative_to(repo_root)
                relative_lines.append(f"{rel}:{rest}")
            except (ValueError, TypeError):
                relative_lines.append(line)
        return "\n".join(relative_lines)

    @lc_tool
    def read_source_file(path: str) -> str:
        """Read an original 1987 source file (.c, .asm, .h, .i, .p).

        Only call this when the user explicitly asks to check the source code.
        path is relative to repo root. Must be a source file extension.
        """
        resolved = _safe_resolve(repo_root, path)
        if isinstance(resolved, str):
            return resolved
        if resolved.suffix not in _SOURCE_EXTENSIONS:
            return (
                f"Error: not allowed — read_source_file only accepts "
                f"{', '.join(sorted(_SOURCE_EXTENSIONS))} files"
            )
        if not resolved.exists():
            return f"Error: file does not exist: {path}"
        if not resolved.is_file():
            return f"Error: not a file: {path}"
        try:
            return resolved.read_text(encoding="utf-8", errors="replace")
        except Exception as exc:
            return f"Error reading file: {exc}"

    return [list_directory, read_file, search_text, read_source_file]


def _python_grep(pattern: str, root: Path, repo_root: Path, cap: int = 50) -> str:
    """Pure-Python fallback grep across .md and .json files."""
    try:
        regex = re.compile(pattern)
    except re.error as exc:
        return f"Error: invalid pattern: {exc}"
    results = []
    paths = [root] if root.is_file() else list(root.rglob("*.md")) + list(root.rglob("*.json"))
    for p in paths:
        if not p.is_file():
            continue
        try:
            for i, line in enumerate(p.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
                if regex.search(line):
                    try:
                        rel = p.relative_to(repo_root)
                    except ValueError:
                        rel = p
                    results.append(f"{rel}:{i}:{line}")
                    if len(results) >= cap:
                        return "\n".join(results)
        except Exception:
            continue
    return "\n".join(results) if results else "No matches found."
