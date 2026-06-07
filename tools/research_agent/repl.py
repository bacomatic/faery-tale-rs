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
