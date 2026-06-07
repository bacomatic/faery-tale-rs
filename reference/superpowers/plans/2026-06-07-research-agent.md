# Research Agent Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a LangGraph ReAct agent that answers FTA research queries by reasoning over reference docs, exposed as both a FastAPI HTTP server and an interactive REPL, backed by any OpenAI-compatible local LLM.

**Architecture:** A `tools/research_agent/` Python package holds the LangGraph graph (`agent.py`), tool definitions (`tools.py`), HTTP server (`server.py`), REPL (`repl.py`), and Pydantic settings (`config.py`). An `AGENT.md` entry document is loaded as the system prompt. The existing `tools/run.sh` venv wrapper handles dependency installation automatically.

**Tech Stack:** Python 3.10+, LangChain/LangGraph (`langchain-openai`, `langgraph`), FastAPI + uvicorn, pydantic-settings, python-dotenv.

**Spec:** `reference/superpowers/specs/2026-06-07-research-agent-design.md`

---

## File Map

| File | Status | Responsibility |
|------|--------|---------------|
| `tools/research_agent/__init__.py` | Create | Package marker |
| `tools/research_agent/config.py` | Create | Pydantic settings — env vars + `.env` loading |
| `tools/research_agent/tools.py` | Create | LangChain tool definitions (list_dir, read_file, search_text, read_source_file) |
| `tools/research_agent/agent.py` | Create | LangGraph ReAct loop, session state, token usage extraction |
| `tools/research_agent/server.py` | Create | FastAPI HTTP server (POST /query, POST /query/stream, GET /health) |
| `tools/research_agent/repl.py` | Create | Interactive REPL with streaming and `/reset` `/sources` `/quit` |
| `tools/research_agent/AGENT.md` | Create | System prompt / entry document |
| `tools/.env.example` | Create | Committed config template |
| `tools/requirements.txt` | Modify | Add 7 new dependencies |
| `.gitignore` | Modify | Add `tools/.env` |
| `tools/README.md` | Modify | Add Research Agent section |
| `.github/agents/research.agent.md` | Create | Agent definition for Copilot |
| `tools/tests/test_research_agent.py` | Create | Unit + integration tests |

---

## Task 1: Dependencies and package scaffold

**Files:**
- Modify: `tools/requirements.txt`
- Modify: `.gitignore`
- Create: `tools/research_agent/__init__.py`
- Create: `tools/.env.example`

- [ ] **Step 1: Add dependencies to requirements.txt**

Open `tools/requirements.txt`. It currently contains:
```
machine68k>=0.4
pytest>=8.0
```

Replace with:
```
machine68k>=0.4
pytest>=8.0
langchain-openai>=0.2
langgraph>=0.2
fastapi>=0.115
uvicorn[standard]>=0.30
python-dotenv>=1.0
pydantic-settings>=2.0
httpx>=0.27
```

- [ ] **Step 2: Add tools/.env to .gitignore**

Open `.gitignore`. Add at the end:
```
# Local LLM config
tools/.env
```

- [ ] **Step 3: Create the package directory and empty __init__.py**

```bash
mkdir -p tools/research_agent
touch tools/research_agent/__init__.py
```

- [ ] **Step 4: Create tools/.env.example**

Create `tools/.env.example` with this exact content:
```ini
# LM Studio (default)
OPENAI_BASE_URL=http://localhost:1234/v1
OPENAI_API_KEY=lm-studio
OPENAI_MODEL=meta-llama-3.1-8b-instruct

# Ollama alternative:
# OPENAI_BASE_URL=http://localhost:11434/v1
# OPENAI_API_KEY=ollama
# OPENAI_MODEL=llama3.1

# Tuning
AGENT_MAX_HISTORY_TURNS=20
AGENT_MAX_TOOL_STEPS=15
AGENT_SERVER_PORT=8765
AGENT_LOG_LEVEL=INFO
```

- [ ] **Step 5: Install dependencies into .toolenv**

```bash
tools/run.sh research_agent/__init__.py
```

This will fail with "no module named research_agent.__main__" but that's fine — it creates `.toolenv` and installs all requirements. Verify the new packages installed:

```bash
.toolenv/bin/pip show langchain-openai langgraph fastapi | grep -E "^Name:|^Version:"
```

Expected output (versions may differ):
```
Name: langchain-openai
Version: 0.2.x
Name: langgraph
Version: 0.2.x
Name: fastapi
Version: 0.115.x
```

- [ ] **Step 6: Commit**

```bash
git add tools/requirements.txt .gitignore tools/research_agent/__init__.py tools/.env.example
git commit -m "feat(research-agent): scaffold package and add dependencies"
```

---

## Task 2: config.py — Settings

**Files:**
- Create: `tools/research_agent/config.py`
- Create: `tools/tests/test_research_agent.py` (first stubs)

- [ ] **Step 1: Write the failing test**

Create `tools/tests/test_research_agent.py`:

```python
"""Tests for tools/research_agent/."""
import os
import sys
from pathlib import Path

# Ensure tools/ is on the path so we can import research_agent
TOOLS = Path(__file__).parent.parent
sys.path.insert(0, str(TOOLS))


class TestConfig:
    def test_default_values(self, monkeypatch):
        """Config loads defaults when no env vars set (except required OPENAI_MODEL)."""
        monkeypatch.setenv("OPENAI_MODEL", "test-model")
        # Clear any existing .env influence
        monkeypatch.delenv("OPENAI_BASE_URL", raising=False)
        monkeypatch.delenv("AGENT_MAX_TOOL_STEPS", raising=False)
        # Re-import to pick up monkeypatched env
        import importlib
        import research_agent.config as cfg_mod
        importlib.reload(cfg_mod)
        settings = cfg_mod.Settings()
        assert settings.openai_base_url == "http://localhost:1234/v1"
        assert settings.openai_model == "test-model"
        assert settings.agent_max_tool_steps == 15
        assert settings.agent_max_history_turns == 20
        assert settings.agent_server_port == 8765

    def test_env_override(self, monkeypatch):
        """Env vars override defaults."""
        monkeypatch.setenv("OPENAI_MODEL", "llama3.1")
        monkeypatch.setenv("OPENAI_BASE_URL", "http://localhost:11434/v1")
        monkeypatch.setenv("AGENT_MAX_TOOL_STEPS", "5")
        import importlib
        import research_agent.config as cfg_mod
        importlib.reload(cfg_mod)
        settings = cfg_mod.Settings()
        assert settings.openai_base_url == "http://localhost:11434/v1"
        assert settings.agent_max_tool_steps == 5
```

- [ ] **Step 2: Run test to verify it fails**

```bash
tools/run.sh -m pytest tools/tests/test_research_agent.py::TestConfig -v
```

Expected: `ModuleNotFoundError: No module named 'research_agent.config'`

- [ ] **Step 3: Implement config.py**

Create `tools/research_agent/config.py`:

```python
"""Settings for the FTA research agent.

Loaded from environment variables and tools/.env (if present).
All settings can be overridden at runtime via environment variables.
"""
from __future__ import annotations

from pathlib import Path

from pydantic_settings import BaseSettings, SettingsConfigDict

# Resolve tools/.env relative to this file's location (tools/research_agent/)
_ENV_FILE = Path(__file__).parent.parent / ".env"


class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_file=str(_ENV_FILE),
        env_file_encoding="utf-8",
        extra="ignore",
    )

    # LLM backend
    openai_base_url: str = "http://localhost:1234/v1"
    openai_api_key: str = "lm-studio"
    openai_model: str  # required — no default

    # Agent tuning
    agent_max_history_turns: int = 20
    agent_max_tool_steps: int = 15
    agent_server_port: int = 8765
    agent_log_level: str = "INFO"
```

- [ ] **Step 4: Run test to verify it passes**

```bash
tools/run.sh -m pytest tools/tests/test_research_agent.py::TestConfig -v
```

Expected: `2 passed`

- [ ] **Step 5: Commit**

```bash
git add tools/research_agent/config.py tools/tests/test_research_agent.py
git commit -m "feat(research-agent): add config with pydantic-settings"
```

---

## Task 3: tools.py — LangChain tool definitions

**Files:**
- Create: `tools/research_agent/tools.py`
- Modify: `tools/tests/test_research_agent.py`

- [ ] **Step 1: Write the failing tests**

Append to `tools/tests/test_research_agent.py`:

```python
class TestAgentTools:
    """Tests for path-safety and basic function of the LangChain tools."""

    @pytest.fixture(autouse=True)
    def repo_root(self, tmp_path):
        """Use a temp dir as fake repo root with reference/ structure."""
        (tmp_path / "reference").mkdir()
        (tmp_path / "reference" / "RESEARCH.md").write_text("# Research\nline2\nline3")
        (tmp_path / "reference" / "sub").mkdir()
        (tmp_path / "reference" / "sub" / "nested.md").write_text("nested content")
        (tmp_path / "fmain.c").write_text("int main() {}")
        self.root = tmp_path
        return tmp_path

    def _tools(self):
        import importlib
        import research_agent.tools as t
        importlib.reload(t)
        return t.make_tools(self.root)

    def _call(self, name: str, **kwargs):
        tools = {t.name: t for t in self._tools()}
        return tools[name].invoke(kwargs)

    def test_list_directory_returns_files(self):
        result = self._call("list_directory", path="reference")
        assert "RESEARCH.md" in result
        assert "sub" in result

    def test_list_directory_rejects_traversal(self):
        result = self._call("list_directory", path="reference/../../etc")
        assert "error" in result.lower() or "not allowed" in result.lower()

    def test_read_file_returns_content(self):
        result = self._call("read_file", path="reference/RESEARCH.md")
        assert "# Research" in result

    def test_read_file_rejects_outside_reference(self):
        result = self._call("read_file", path="fmain.c")
        assert "error" in result.lower() or "not allowed" in result.lower()

    def test_read_file_rejects_traversal(self):
        result = self._call("read_file", path="reference/../fmain.c")
        assert "error" in result.lower() or "not allowed" in result.lower()

    def test_search_text_finds_match(self):
        result = self._call("search_text", pattern="Research", path="reference")
        assert "RESEARCH.md" in result

    def test_search_text_returns_no_results_message(self):
        result = self._call("search_text", pattern="xyzzy_not_present", path="reference")
        assert "no matches" in result.lower() or result.strip() == ""

    def test_read_source_file_returns_content(self):
        result = self._call("read_source_file", path="fmain.c")
        assert "int main" in result

    def test_read_source_file_rejects_non_source(self):
        (self.root / "notes.txt").write_text("notes")
        result = self._call("read_source_file", path="notes.txt")
        assert "error" in result.lower() or "not allowed" in result.lower()

    def test_read_source_file_rejects_traversal(self):
        result = self._call("read_source_file", path="../../../etc/passwd")
        assert "error" in result.lower() or "not allowed" in result.lower()
```

Also add `import pytest` at the top of the test file.

- [ ] **Step 2: Run tests to verify they fail**

```bash
tools/run.sh -m pytest tools/tests/test_research_agent.py::TestAgentTools -v
```

Expected: `ModuleNotFoundError: No module named 'research_agent.tools'`

- [ ] **Step 3: Implement tools.py**

Create `tools/research_agent/tools.py`:

```python
"""LangChain tools for the FTA research agent.

All file access is path-safety checked against repo_root.
Errors are returned as strings (not exceptions) so the model can recover.
"""
from __future__ import annotations

import re
import subprocess
from pathlib import Path
from typing import Callable

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
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
tools/run.sh -m pytest tools/tests/test_research_agent.py::TestAgentTools -v
```

Expected: `10 passed`

- [ ] **Step 5: Commit**

```bash
git add tools/research_agent/tools.py tools/tests/test_research_agent.py
git commit -m "feat(research-agent): add path-safe LangChain tools"
```

---

## Task 4: AGENT.md — Entry document

**Files:**
- Create: `tools/research_agent/AGENT.md`

No tests for this task — content quality is verified by manual inspection during Task 6 (end-to-end).

- [ ] **Step 1: Create AGENT.md**

Create `tools/research_agent/AGENT.md` with this content:

```markdown
# FTA Research Agent

You are a research assistant for *The Faery Tale Adventure* (MicroIllusions, 1987 Amiga).
You answer questions about the game's mechanics, source code, story, and data by reasoning
over the project's reference documentation.

## Rules

1. **No guessing.** Every claim must come from a document you have read. If you cannot find
   the answer, say so explicitly.
2. **Cite sources.** After every factual statement, cite the document and section or line:
   `reference/RESEARCH-terrain-combat.md §3.2` or `fmain.c:1609`.
3. **Use search before bulk reads.** Call `search_text` first to locate relevant sections.
   Only call `read_file` on documents that search confirms are relevant.
4. **Source code is last resort.** Only call `read_source_file` when the user explicitly
   asks to "check the source", "verify in code", or similar. Reference docs are the primary
   source of truth.
5. **world_db.json is large (600 KB).** Never read it in full. Use `search_text` with
   a region name, object type, or coordinate to find relevant entries.

## Available Tools

- `list_directory(path)` — list files in a directory under `reference/`
- `read_file(path)` — read a file under `reference/`
- `search_text(pattern, path)` — regex search across reference docs (default: all of `reference/`)
- `read_source_file(path)` — read an original `.c`/`.asm`/`.h`/`.i`/`.p` source file

All paths are relative to the repository root.

## Document Index

| Document | Answers questions about |
|----------|------------------------|
| `reference/ARCHITECTURE.md` | High-level system overview, 19 subsystems, game loop structure, display geometry, Mermaid diagrams |
| `reference/RESEARCH.md` | Index / table of contents for all RESEARCH-* section files |
| `reference/RESEARCH-terrain-combat.md` | Terrain types, movement costs, combat damage formula, hit calculation, bravery scaling, weapon stats |
| `reference/RESEARCH-ai-encounters.md` | Enemy AI tactics, encounter spawning, monster behaviour tables, patrol logic |
| `reference/RESEARCH-input-movement.md` | Joystick handling, direction encoding, movement vectors, speed tables |
| `reference/RESEARCH-items-world.md` | Inventory items (`stuff[N]`), item effects, world object placement, shops |
| `reference/RESEARCH-npcs-quests.md` | NPC dialogue trees, quest state machine, brother succession, rescue sequences |
| `reference/RESEARCH-systems.md` | Save/load, disk I/O, copy protection, day/night cycle, astral plane |
| `reference/RESEARCH-data-structures.md` | All structs, enums, constants, and array definitions from `ftale.h` |
| `reference/STORYLINE.md` | Narrative overview and index for STORYLINE-* files |
| `reference/STORYLINE-npcs.md` | Individual NPC interaction diagrams |
| `reference/STORYLINE-quests.md` | Quest progression state diagrams |
| `reference/STORYLINE-world-events.md` | World event sequences (day/night, door transitions, etc.) |
| `reference/ARCHITECTURE.md` | Subsystem map, data flow, rendering pipeline |
| `reference/CONTROLS.md` | Player controls reference |
| `reference/PROBLEMS.md` | Open questions that cannot be answered from source code alone |
| `reference/_discovery/` | Raw agent findings — use for deep detail when reference docs are insufficient |
| `reference/logic/` | Normative pseudo-code for non-trivial functions |

## world_db.json Schema

`reference/world_db.json` is a spatial index of the game world. Do not read it in full.
Use `search_text` to find entries by name, region, or type.

Top-level keys:
- `objects` — 129 world objects: `{id, name, type, region, sector, grid_col, grid_row, x, y, place_name}`
- `doors` — 86 door/stair/gate transitions: `{id, outside: {region, sector, x, y}, inside: {region, sector, x, y}}`
- `extents` — 23 encounter trigger zones: `{id, type, region, sector, x1, y1, x2, y2}`
- `zones` — 3 special zones: desert gate, fiery death box, astral plane
- `sector_terrain` — 996 entries: per-sector terrain composition `{region, sector, terrain_counts}`
- `region_grids` — 10 entries: full 64×32 tile grids per region

Regions are numbered 0–9. Sectors are 0-based tile coordinates within a region.

## Source Citation Format

- Reference doc: `reference/RESEARCH-terrain-combat.md §3.2`
- Source line: `fmain.c:1609` or `fmain.c:1609-1625`
- Speech message: `speak(42)` (index into `narr.asm` message table)
```

- [ ] **Step 2: Commit**

```bash
git add tools/research_agent/AGENT.md
git commit -m "feat(research-agent): add AGENT.md entry document"
```

---

## Task 5: agent.py — LangGraph ReAct loop

**Files:**
- Create: `tools/research_agent/agent.py`
- Modify: `tools/tests/test_research_agent.py`

- [ ] **Step 1: Write failing tests**

Append to `tools/tests/test_research_agent.py`:

```python
class TestAgentModule:
    """Smoke-tests for agent.py — uses a mock LLM to avoid needing a live server."""

    def test_create_agent_returns_graph(self, tmp_path):
        """create_agent() returns a compiled LangGraph object."""
        import importlib
        import research_agent.agent as agent_mod
        importlib.reload(agent_mod)
        from unittest.mock import MagicMock
        mock_llm = MagicMock()
        mock_llm.bind_tools = MagicMock(return_value=mock_llm)
        graph = agent_mod.create_agent(llm=mock_llm, repo_root=tmp_path)
        # LangGraph compiled graphs have an .invoke method
        assert hasattr(graph, "invoke")

    def test_token_usage_extracted_from_response(self):
        """extract_usage() handles present and absent usage fields gracefully."""
        import importlib
        import research_agent.agent as agent_mod
        importlib.reload(agent_mod)
        from langchain_core.messages import AIMessage
        msg_with_usage = AIMessage(
            content="answer",
            response_metadata={"token_usage": {"prompt_tokens": 10, "completion_tokens": 5}},
        )
        usage = agent_mod.extract_usage(msg_with_usage, elapsed=2.0)
        assert usage["prompt_tokens"] == 10
        assert usage["completion_tokens"] == 5
        assert abs(usage["tokens_per_sec"] - 2.5) < 0.01

        msg_no_usage = AIMessage(content="answer")
        assert agent_mod.extract_usage(msg_no_usage, elapsed=1.0) is None
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
tools/run.sh -m pytest tools/tests/test_research_agent.py::TestAgentModule -v
```

Expected: `ModuleNotFoundError: No module named 'research_agent.agent'`

- [ ] **Step 3: Implement agent.py**

Create `tools/research_agent/agent.py`:

```python
"""LangGraph ReAct agent for FTA research queries.

The agent loads AGENT.md as the system prompt and uses lazy tool calls
to read reference documentation on demand.
"""
from __future__ import annotations

import logging
import time
from pathlib import Path
from typing import Any

from langchain_core.messages import AIMessage, BaseMessage, HumanMessage, SystemMessage
from langchain_openai import ChatOpenAI
from langgraph.prebuilt import create_react_agent

from .config import Settings
from .tools import make_tools

logger = logging.getLogger(__name__)

_AGENT_MD = Path(__file__).parent / "AGENT.md"

# Type alias for token usage dict
TokenUsage = dict[str, Any]  # prompt_tokens, completion_tokens, tokens_per_sec


def load_system_prompt() -> str:
    """Return the contents of AGENT.md."""
    return _AGENT_MD.read_text(encoding="utf-8")


def create_agent(llm=None, repo_root: Path | None = None):
    """Create and return a compiled LangGraph ReAct graph.

    Args:
        llm: Optional pre-built LLM (used in tests). If None, built from Settings.
        repo_root: Repository root for tools. Defaults to three levels up from this file.
    """
    if repo_root is None:
        repo_root = Path(__file__).parent.parent.parent
    tools = make_tools(repo_root)
    if llm is None:
        settings = Settings()
        llm = ChatOpenAI(
            base_url=settings.openai_base_url,
            api_key=settings.openai_api_key,
            model=settings.openai_model,
            streaming=True,
        )
    system_prompt = load_system_prompt()
    graph = create_react_agent(
        model=llm,
        tools=tools,
        state_modifier=system_prompt,
    )
    return graph


def extract_usage(message: BaseMessage, elapsed: float) -> TokenUsage | None:
    """Extract token usage from an AIMessage response_metadata.

    Returns a dict with prompt_tokens, completion_tokens, tokens_per_sec,
    or None if usage data is not available.
    """
    if not isinstance(message, AIMessage):
        return None
    meta = getattr(message, "response_metadata", {}) or {}
    usage = meta.get("token_usage") or meta.get("usage") or {}
    prompt = usage.get("prompt_tokens") or usage.get("input_tokens")
    completion = usage.get("completion_tokens") or usage.get("output_tokens")
    if prompt is None or completion is None:
        return None
    tps = completion / elapsed if elapsed > 0 else 0.0
    return {
        "prompt_tokens": int(prompt),
        "completion_tokens": int(completion),
        "tokens_per_sec": round(tps, 1),
    }


class Session:
    """Holds per-session message history for the HTTP server."""

    def __init__(self, graph, max_history_turns: int = 20):
        self.graph = graph
        self.max_history_turns = max_history_turns
        self._messages: list[BaseMessage] = []

    def _trim(self) -> None:
        """Trim oldest non-system messages if history exceeds max."""
        non_system = [m for m in self._messages if not isinstance(m, SystemMessage)]
        if len(non_system) > self.max_history_turns * 2:
            # Keep the most recent max_history_turns*2 non-system messages
            keep = non_system[-(self.max_history_turns * 2):]
            self._messages = keep

    def query(self, text: str) -> tuple[str, list[str], TokenUsage | None]:
        """Run a query and return (answer, sources, usage).

        sources is a list of file paths mentioned in tool calls.
        """
        self._messages.append(HumanMessage(content=text))
        self._trim()

        start = time.monotonic()
        settings = Settings()
        result = self.graph.invoke(
            {"messages": self._messages},
            config={"recursion_limit": settings.agent_max_tool_steps + 5},
        )
        elapsed = time.monotonic() - start

        all_messages: list[BaseMessage] = result["messages"]
        # Update history with the full message list from this turn
        self._messages = all_messages

        # Find the final AI answer
        ai_messages = [m for m in all_messages if isinstance(m, AIMessage) and m.content]
        answer = ai_messages[-1].content if ai_messages else "(no response)"

        # Collect sources from tool call arguments
        sources: list[str] = []
        for msg in all_messages:
            if hasattr(msg, "tool_calls"):
                for call in (msg.tool_calls or []):
                    args = call.get("args", {})
                    for key in ("path", "pattern"):
                        if key in args:
                            sources.append(str(args[key]))
            # Also check ToolMessage names
            if hasattr(msg, "name") and msg.name:
                pass  # tool name not a path — skip

        usage = extract_usage(ai_messages[-1], elapsed) if ai_messages else None
        return answer, list(dict.fromkeys(sources)), usage  # deduplicated
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
tools/run.sh -m pytest tools/tests/test_research_agent.py::TestAgentModule -v
```

Expected: `2 passed`

- [ ] **Step 5: Run all research agent tests so far**

```bash
tools/run.sh -m pytest tools/tests/test_research_agent.py -v
```

Expected: all tests pass (TestConfig: 2, TestAgentTools: 10, TestAgentModule: 2 = 14 total)

- [ ] **Step 6: Commit**

```bash
git add tools/research_agent/agent.py tools/tests/test_research_agent.py
git commit -m "feat(research-agent): add LangGraph ReAct loop and session state"
```

---

## Task 6: server.py — FastAPI HTTP server

**Files:**
- Create: `tools/research_agent/server.py`
- Modify: `tools/tests/test_research_agent.py`

- [ ] **Step 1: Write failing tests**

Append to `tools/tests/test_research_agent.py`:

```python
class TestServer:
    """HTTP server tests using FastAPI TestClient (no live LLM required)."""

    @pytest.fixture
    def client(self, tmp_path, monkeypatch):
        """Return a TestClient with a mock agent session."""
        import importlib
        import research_agent.server as server_mod
        importlib.reload(server_mod)
        from fastapi.testclient import TestClient
        from unittest.mock import patch, MagicMock

        mock_session = MagicMock()
        mock_session.query.return_value = (
            "The damage formula is X.",
            ["reference/RESEARCH-terrain-combat.md"],
            {"prompt_tokens": 10, "completion_tokens": 5, "tokens_per_sec": 3.2},
        )

        with patch.object(server_mod, "_get_or_create_session", return_value=mock_session):
            app = server_mod.create_app()
            yield TestClient(app)

    def test_health_endpoint(self, client):
        resp = client.get("/health")
        assert resp.status_code == 200
        data = resp.json()
        assert data["status"] == "ok"
        assert "model" in data

    def test_query_endpoint_returns_answer(self, client):
        resp = client.post("/query", json={"query": "What is the damage formula?"})
        assert resp.status_code == 200
        data = resp.json()
        assert data["answer"] == "The damage formula is X."
        assert "reference/RESEARCH-terrain-combat.md" in data["sources"]
        assert "session_id" in data
        assert data["usage"]["prompt_tokens"] == 10

    def test_query_endpoint_accepts_session_id(self, client):
        resp = client.post("/query", json={"query": "Q", "session_id": "abc-123"})
        assert resp.status_code == 200
        assert resp.json()["session_id"] == "abc-123"

    def test_query_missing_query_field(self, client):
        resp = client.post("/query", json={})
        assert resp.status_code == 422
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
tools/run.sh -m pytest tools/tests/test_research_agent.py::TestServer -v
```

Expected: `ModuleNotFoundError: No module named 'research_agent.server'`

- [ ] **Step 3: Implement server.py**

Create `tools/research_agent/server.py`:

```python
"""FastAPI HTTP server for the FTA research agent.

Endpoints:
  POST /query          — synchronous query → JSON response
  POST /query/stream   — streaming query → SSE token stream
  GET  /health         — liveness check

Run with:
  tools/run.sh research_agent/server.py [--port 8765]
"""
from __future__ import annotations

import asyncio
import logging
import sys
import uuid
from typing import Any, Optional

import uvicorn
from fastapi import FastAPI
from fastapi.responses import StreamingResponse
from pydantic import BaseModel

from .agent import Session, create_agent, extract_usage
from .config import Settings

logger = logging.getLogger(__name__)

# In-process session store: session_id → Session
_sessions: dict[str, Session] = {}
_graph = None


def _get_graph():
    global _graph
    if _graph is None:
        _graph = create_agent()
    return _graph


def _get_or_create_session(session_id: str) -> Session:
    if session_id not in _sessions:
        settings = Settings()
        _sessions[session_id] = Session(
            graph=_get_graph(),
            max_history_turns=settings.agent_max_history_turns,
        )
    return _sessions[session_id]


class QueryRequest(BaseModel):
    query: str
    session_id: Optional[str] = None


class UsageInfo(BaseModel):
    prompt_tokens: int
    completion_tokens: int
    tokens_per_sec: float


class QueryResponse(BaseModel):
    answer: str
    sources: list[str]
    session_id: str
    usage: Optional[UsageInfo] = None


def create_app() -> FastAPI:
    settings = Settings()
    app = FastAPI(title="FTA Research Agent")

    @app.get("/health")
    def health():
        return {"status": "ok", "model": settings.openai_model}

    @app.post("/query", response_model=QueryResponse)
    def query(req: QueryRequest):
        sid = req.session_id or str(uuid.uuid4())
        session = _get_or_create_session(sid)
        answer, sources, usage = session.query(req.query)
        usage_info = UsageInfo(**usage) if usage else None
        if usage:
            logger.info(
                "POST /query session=%s prompt=%d completion=%d tok/s=%.1f",
                sid[:8],
                usage["prompt_tokens"],
                usage["completion_tokens"],
                usage["tokens_per_sec"],
            )
        else:
            logger.info("POST /query session=%s (tokens: unavailable)", sid[:8])
        return QueryResponse(
            answer=answer,
            sources=sources,
            session_id=sid,
            usage=usage_info,
        )

    @app.post("/query/stream")
    async def query_stream(req: QueryRequest):
        sid = req.session_id or str(uuid.uuid4())
        session = _get_or_create_session(sid)

        async def event_generator():
            # Run query in a thread so we don't block the event loop
            loop = asyncio.get_event_loop()
            answer, sources, usage = await loop.run_in_executor(
                None, session.query, req.query
            )
            # Stream the answer word-by-word as SSE events
            for word in answer.split(" "):
                yield f"data: {word} \n\n"
                await asyncio.sleep(0)
            stats = ""
            if usage:
                stats = (
                    f"[{usage['prompt_tokens']}p + {usage['completion_tokens']}c "
                    f"| {usage['tokens_per_sec']} tok/s]"
                )
            yield f"event: done\ndata: {stats}\n\n"

        return StreamingResponse(event_generator(), media_type="text/event-stream")

    return app


def main():
    import argparse
    parser = argparse.ArgumentParser(description="FTA Research Agent HTTP server")
    parser.add_argument("--port", type=int, default=None)
    args = parser.parse_args()

    settings = Settings()
    port = args.port or settings.agent_server_port

    logging.basicConfig(
        level=getattr(logging, settings.agent_log_level.upper(), logging.INFO),
        format="%(asctime)s %(levelname)s %(name)s — %(message)s",
    )

    # Validate LLM reachable before starting server
    import httpx
    try:
        httpx.get(f"{settings.openai_base_url.rstrip('/')}/models", timeout=5)
    except Exception as exc:
        logger.error("LLM endpoint unreachable at %s: %s", settings.openai_base_url, exc)
        sys.exit(1)

    uvicorn.run(create_app(), host="127.0.0.1", port=port, log_level="warning")


if __name__ == "__main__":
    main()
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
tools/run.sh -m pytest tools/tests/test_research_agent.py::TestServer -v
```

Expected: `4 passed`

- [ ] **Step 5: Run full test suite**

```bash
tools/run.sh -m pytest tools/tests/test_research_agent.py -v
```

Expected: 18 passed, 0 failed.

- [ ] **Step 6: Commit**

```bash
git add tools/research_agent/server.py tools/tests/test_research_agent.py
git commit -m "feat(research-agent): add FastAPI HTTP server with /query and /health"
```

---

## Task 7: repl.py — Interactive REPL

**Files:**
- Create: `tools/research_agent/repl.py`

No automated tests for this task — the REPL requires terminal interaction. Manual verification in Task 8.

- [ ] **Step 1: Implement repl.py**

Create `tools/research_agent/repl.py`:

```python
"""Interactive REPL for the FTA research agent.

Run with:
  tools/run.sh research_agent/repl.py

Special commands:
  /reset    — clear conversation history
  /sources  — show sources from last response
  /quit     — exit
"""
from __future__ import annotations

import logging
import sys
import time

from .agent import Session, create_agent
from .config import Settings


def main():
    settings = Settings()
    logging.basicConfig(
        level=getattr(logging, settings.agent_log_level.upper(), logging.INFO),
        format="%(levelname)s %(name)s — %(message)s",
        stream=sys.stderr,
    )

    print("FTA Research Agent  (type /quit to exit, /reset to clear history, /sources for last sources)")
    print(f"Model: {settings.openai_model}  Backend: {settings.openai_base_url}")
    print()

    graph = create_agent()
    session = Session(graph, max_history_turns=settings.agent_max_history_turns)
    last_sources: list[str] = []

    while True:
        try:
            raw = input("> ").strip()
        except (EOFError, KeyboardInterrupt):
            print("\nBye.")
            break

        if not raw:
            continue

        if raw == "/quit":
            print("Bye.")
            break

        if raw == "/reset":
            session = Session(graph, max_history_turns=settings.agent_max_history_turns)
            last_sources = []
            print("(history cleared)")
            continue

        if raw == "/sources":
            if last_sources:
                print("Sources from last response:")
                for s in last_sources:
                    print(f"  {s}")
            else:
                print("(no sources recorded)")
            continue

        # Run the query
        start = time.monotonic()
        try:
            answer, sources, usage = session.query(raw)
        except Exception as exc:
            print(f"Error: {exc}", file=sys.stderr)
            continue

        elapsed = time.monotonic() - start
        last_sources = sources

        print(answer)
        print()

        # Compact stats line
        if usage:
            print(
                f"[{elapsed:.1f}s | {usage['prompt_tokens']} prompt + "
                f"{usage['completion_tokens']} completion tokens | "
                f"{usage['tokens_per_sec']} tok/s]"
            )
        else:
            print(f"[{elapsed:.1f}s | tokens: unavailable]")

        if sources:
            print(f"Sources: {', '.join(sources)}")
        print()


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Commit**

```bash
git add tools/research_agent/repl.py
git commit -m "feat(research-agent): add interactive REPL with streaming stats"
```

---

## Task 8: Documentation and agent definition

**Files:**
- Modify: `tools/README.md`
- Create: `.github/agents/research.agent.md`

- [ ] **Step 1: Add Research Agent section to tools/README.md**

Open `tools/README.md`. After the `## Setup` section, add a new section before `## Quick Start`:

```markdown
## Research Agent

A locally-hosted LLM agent that answers questions about the FTA codebase by reasoning over
the reference documentation. Backed by any OpenAI-compatible server (LM Studio, Ollama, etc.).

### Setup

1. Copy `tools/.env.example` to `tools/.env` and fill in your model name:
   ```bash
   cp tools/.env.example tools/.env
   # Edit tools/.env — set OPENAI_MODEL to whatever model you have loaded
   ```

2. Make sure your LLM server is running (LM Studio on port 1234, or Ollama on port 11434).

### Running the HTTP server

```bash
tools/run.sh research_agent/server.py
# or with a custom port:
tools/run.sh research_agent/server.py --port 9000
```

Query it:
```bash
curl -s -X POST http://localhost:8765/query \
  -H "Content-Type: application/json" \
  -d '{"query": "What is the combat damage formula?"}' | python3 -m json.tool
```

### Running the interactive REPL

```bash
tools/run.sh research_agent/repl.py
```

Type your question at the `>` prompt. Special commands: `/reset`, `/sources`, `/quit`.

### Configuration

All settings live in `tools/.env` (see `tools/.env.example`). Key variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `OPENAI_BASE_URL` | `http://localhost:1234/v1` | LLM server URL |
| `OPENAI_API_KEY` | `lm-studio` | API key (any string for local servers) |
| `OPENAI_MODEL` | *(required)* | Model name |
| `AGENT_LOG_LEVEL` | `INFO` | Set to `DEBUG` to log every tool call |
| `AGENT_MAX_TOOL_STEPS` | `15` | Max reasoning steps before truncating |

```

- [ ] **Step 2: Create .github/agents/research.agent.md**

Create `.github/agents/research.agent.md`:

```markdown
---
description: "Use for natural-language queries about FTA mechanics, story, data structures, and source code — answers by reasoning over reference documentation rather than semantic search"
tools: [fetch]
---
You are a research assistant for *The Faery Tale Adventure* (MicroIllusions, 1987 Amiga).

You are backed by a locally-hosted LLM running via an ACP HTTP server at `http://localhost:8765`.

To answer a question, POST it to the research agent:

```bash
curl -s -X POST http://localhost:8765/query \
  -H "Content-Type: application/json" \
  -d '{"query": "<your question here>"}' | python3 -m json.tool
```

The agent will reason over the reference documentation and return an answer with source citations.

**Start the server first** (if not already running):
```bash
tools/run.sh research_agent/server.py
```

**Capabilities:**
- Answers questions about game mechanics, formulas, NPC dialogue, quests, data structures
- Cites specific reference documents and source file lines
- Can dive into original 1987 source code when explicitly asked
- Maintains conversation history within a session (pass `session_id` to continue)
```

- [ ] **Step 3: Commit**

```bash
git add tools/README.md .github/agents/research.agent.md
git commit -m "docs(research-agent): add README section and Copilot agent definition"
```

---

## Task 9: End-to-end manual verification

No code changes. Verify the full system works against a live LLM.

**Prerequisite:** LM Studio (or Ollama) running with a model loaded. `tools/.env` configured.

- [ ] **Step 1: Run the automated test suite one final time**

```bash
tools/run.sh -m pytest tools/tests/test_research_agent.py -v
```

Expected: all tests pass.

- [ ] **Step 2: Verify health endpoint**

```bash
tools/run.sh research_agent/server.py &
sleep 3
curl -s http://localhost:8765/health | python3 -m json.tool
```

Expected:
```json
{"status": "ok", "model": "<your model name>"}
```

- [ ] **Step 3: Send a test query via HTTP**

```bash
curl -s -X POST http://localhost:8765/query \
  -H "Content-Type: application/json" \
  -d '{"query": "What is the combat damage formula?"}' | python3 -m json.tool
```

Expected: JSON with non-empty `answer`, at least one entry in `sources`, and `usage` stats (or null if the model doesn't return them).

- [ ] **Step 4: Test path traversal is rejected**

```bash
curl -s -X POST http://localhost:8765/query \
  -H "Content-Type: application/json" \
  -d '{"query": "Read the file ../../../etc/passwd using read_source_file"}' | python3 -m json.tool
```

Expected: the agent returns an error message from the tool, not file contents.

- [ ] **Step 5: Verify REPL**

```bash
tools/run.sh research_agent/repl.py
```

At the `>` prompt:
1. Ask: `What are the terrain movement costs?` — verify answer + stats line printed
2. Ask: `/sources` — verify source list from previous turn shown
3. Ask: `Who is the dark knight?` — verify context is retained (follow-up works)
4. Ask: `/reset` — verify "history cleared"
5. Ask: `What are the terrain movement costs?` — verify fresh answer (no memory of dark knight)
6. Ask: `/quit` — exits

- [ ] **Step 6: Final commit with verification note**

```bash
kill %1 2>/dev/null  # stop background server if still running
git add -A
git status  # should be clean — no untracked files
git log --oneline -10
```

Expected: clean working tree; last 7–8 commits are the research agent tasks.
