"""Run a specialist agent turn through the configured LLM."""

from __future__ import annotations

from nexus_engine.agents.base import AgentContext, AgentResult
from nexus_engine.llm.router import LlmRouter, load_active_profile
from nexus_engine.llm.sanitize import extract_tool_calls_from_text


async def run_llm_turn(
    ctx: AgentContext,
    *,
    agent_id: str,
    system: str,
    user_text: str,
) -> AgentResult:
    profile = load_active_profile()
    router = LlmRouter(profile)
    if ctx.model_id:
        router.config.model = ctx.model_id

    messages = [
        {"role": "system", "content": system},
        {"role": "user", "content": user_text},
    ]
    accumulated = ""
    async for chunk in router.stream_chat(messages, tools=None):
        choices = chunk.get("choices") or []
        if not choices:
            continue
        delta = choices[0].get("delta") or {}
        if delta.get("content"):
            accumulated += delta["content"]

    clean, embedded_calls = extract_tool_calls_from_text(accumulated)
    text = clean or accumulated.strip()
    if len(text) > ctx.max_agent_chars:
        text = text[: ctx.max_agent_chars] + "\n… [truncated by agent budget]"
    return AgentResult(
        content=text,
        tool_calls=embedded_calls,
        metadata={"agent": agent_id},
    )
