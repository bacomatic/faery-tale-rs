"""Tests for tools/lint_logic.py."""
from pathlib import Path
import subprocess
import sys

REPO_ROOT = Path(__file__).parent.parent.parent
LINTER = REPO_ROOT / "tools" / "lint_logic.py"


def run_linter(*args: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(LINTER), *args],
        capture_output=True,
        text=True,
        cwd=REPO_ROOT,
    )


def test_linter_runs_and_reports_help():
    result = run_linter("--help")
    assert result.returncode == 0
    assert "lint_logic" in result.stdout.lower()


def test_check_file_header_passes_on_valid_fixture(fixtures_dir):
    result = run_linter("--file", str(fixtures_dir / "valid_minimal.md"))
    assert result.returncode == 0, result.stdout + result.stderr


def test_check_file_header_fails_on_missing_header(fixtures_dir):
    result = run_linter("--file", str(fixtures_dir / "missing_header.md"))
    assert result.returncode != 0
    assert "fidelity" in (result.stdout + result.stderr).lower()


def test_function_header_missing_calls_fails(fixtures_dir):
    result = run_linter("--file", str(fixtures_dir / "bad_function_header.md"))
    assert result.returncode != 0
    combined = (result.stdout + result.stderr).lower()
    assert "called by" in combined or "calls" in combined


def test_bad_citation_fails(fixtures_dir):
    result = run_linter("--file", str(fixtures_dir / "bad_citation.md"))
    assert result.returncode != 0
    assert "99999999" in (result.stdout + result.stderr)


def test_bad_syntax_fails(fixtures_dir):
    result = run_linter("--file", str(fixtures_dir / "bad_syntax.md"))
    assert result.returncode != 0
    assert "syntax" in (result.stdout + result.stderr).lower()


def test_forbidden_try_fails(fixtures_dir):
    result = run_linter("--file", str(fixtures_dir / "forbidden_try.md"))
    assert result.returncode != 0
    assert "try" in (result.stdout + result.stderr).lower()


def test_bad_signature_fails(fixtures_dir):
    result = run_linter("--file", str(fixtures_dir / "bad_signature.md"))
    assert result.returncode != 0
    combined = (result.stdout + result.stderr).lower()
    assert "annotation" in combined or "docstring" in combined


def test_unknown_symbol_fails(fixtures_dir):
    result = run_linter("--file", str(fixtures_dir / "unknown_symbol.md"))
    assert result.returncode != 0
    assert "undefined_global" in (result.stdout + result.stderr)


def test_unknown_table_ref_fails(fixtures_dir):
    result = run_linter("--file", str(fixtures_dir / "unknown_table.md"))
    assert result.returncode != 0
    assert "this_does_not_exist" in (result.stdout + result.stderr)


def test_magic_number_fails(fixtures_dir):
    result = run_linter("--file", str(fixtures_dir / "magic_number.md"))
    assert result.returncode != 0
    assert "42" in (result.stdout + result.stderr)


def test_bad_crossref_fails(fixtures_dir):
    result = run_linter("--file", str(fixtures_dir / "bad_crossref.md"))
    assert result.returncode != 0
    assert "nowhere.md" in (result.stdout + result.stderr)


def test_state_coverage_fails(fixtures_dir):
    result = run_linter("--file", str(fixtures_dir / "state_coverage.md"))
    assert result.returncode != 0
    assert "STATE_B" in (result.stdout + result.stderr)


def test_index_orphan_fails(fixtures_dir, tmp_path, monkeypatch):
    # Exercises Check #11 against a mini logic dir.
    import os
    orphan_dir = fixtures_dir / "index_orphan"
    result = subprocess.run(
        [sys.executable, str(LINTER), "--logic-dir", str(orphan_dir)],
        capture_output=True, text=True, cwd=REPO_ROOT,
    )
    assert result.returncode != 0
    combined = result.stdout + result.stderr
    assert "orphan_function" in combined or "ghost_function" in combined
