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
