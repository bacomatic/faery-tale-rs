#!/usr/bin/env bash
# Run a tools/ script using the project's .toolenv virtual environment.
# Usage: tools/run <script.py> [args...]
#   e.g. tools/run verify_asm.py -c "moveq #42,d0" --trace

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VENV="$REPO_ROOT/.toolenv"
TOOLS="$REPO_ROOT/tools"

if [[ ! -d "$VENV" ]]; then
    echo "Creating .toolenv virtual environment..." >&2
    python3 -m venv "$VENV"
    "$VENV/bin/pip" install -q -r "$TOOLS/requirements.txt"
    echo "Done." >&2
fi

script="$1"
shift

# Allow both "verify_asm.py" and "tools/verify_asm.py"
if [[ "$script" != /* && ! -f "$script" ]]; then
    script="$TOOLS/$script"
fi

exec "$VENV/bin/python" "$script" "$@"
