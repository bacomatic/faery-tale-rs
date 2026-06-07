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
