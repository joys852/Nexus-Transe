"""LLM-backed conversation summarization for /compact."""

from __future__ import annotations

from typing import Any

from nexus_engine.llm.router import LlmRouter, load_active_profile


async def summarize_messages(messages: list[dict[str, Any]], *, max_input_chars: int = 24_000) -> str:
    """Produce a dense summary of user/assistant turns for context compression."""
    lines: list[str] = []
    total = 0
    for m in messages:
        role = m.get("role", "?")
        if role not in ("user", "assistant"):
            continue
        content = str(m.get("content", "")).strip()
        if not content:
            continue
        chunk = content[:2000] + ("…" if len(content) > 2000 else "")
        line = f"{role}: {chunk}"
        if total + len(line) > max_input_chars:
            lines.append("… [truncated for summarization]")
            break
        lines.append(line)
        total += len(line)

    if not lines:
        return "[No prior conversation to summarize]"

    transcript = "\n".join(lines)
    prompt = (
        "Summarize the following coding-session conversation for context compression. "
        "Preserve: goals, decisions, file paths touched, errors, and open tasks. "
        "Use concise bullet points in the user's language. Max 800 words.\n\n"
        f"{transcript}"
    )
    router = LlmRouter(load_active_profile())
    out = await router.complete_chat(
        [{"role": "user", "content": prompt}],
        tools=None,
    )
    return out.strip() or "[Summary unavailable]"
