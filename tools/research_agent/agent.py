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
        prompt=system_prompt,
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

        usage = extract_usage(ai_messages[-1], elapsed) if ai_messages else None
        return answer, list(dict.fromkeys(sources)), usage  # deduplicated
