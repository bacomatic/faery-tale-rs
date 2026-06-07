"""Tests for tools/research_agent/."""
import os
import sys
import pytest
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


class TestServer:
    """HTTP server tests using FastAPI TestClient (no live LLM required)."""

    @pytest.fixture
    def client(self, monkeypatch):
        """Return a TestClient with a mock agent session."""
        monkeypatch.setenv("OPENAI_MODEL", "test-model")
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
