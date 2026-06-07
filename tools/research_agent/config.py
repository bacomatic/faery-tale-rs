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
