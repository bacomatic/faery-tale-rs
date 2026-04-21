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
