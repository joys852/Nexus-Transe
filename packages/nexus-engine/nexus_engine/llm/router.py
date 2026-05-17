"""Route to OpenAI-compat or Anthropic native based on provider profile."""

from __future__ import annotations

import os
from pathlib import Path
from typing import Any, AsyncIterator

from nexus_engine.llm.anthropic_native import AnthropicMessagesClient
from nexus_engine.llm.api_keys import resolve_api_key
from nexus_engine.llm.openai_compat import OpenAICompatClient
from nexus_engine.llm.provider import LLMConfig

try:
    import tomllib
except ImportError:
    import tomli as tomllib  # type: ignore


def _providers_path() -> Path:
    if p := os.getenv("NEXUS_DATA_DIR"):
        return Path(p) / "providers.toml"
    if os.name == "nt":
        appdata = os.environ.get("APPDATA")
        if appdata:
            return Path(appdata) / "nexus-ide" / "providers.toml"
    home = Path(os.environ.get("HOME") or ".")
    return home / ".local" / "share" / "nexus-ide" / "providers.toml"


def load_active_profile() -> dict[str, Any] | None:
    path = _providers_path()
    if not path.exists():
        return None
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    active = data.get("active")
    for p in data.get("providers", []):
        if p.get("id") == active:
            return p
    return data.get("providers", [None])[0] if data.get("providers") else None


def profile_to_config(profile: dict[str, Any] | None) -> LLMConfig:
    if not profile:
        from nexus_engine.llm.openai_compat import config_from_env

        return config_from_env()

    protocol = profile.get("protocol", "openai_chat_completions")
    api_key = resolve_api_key(profile)

    return LLMConfig(
        provider=protocol,
        model=profile.get("model", "gpt-4o-mini"),
        api_key=api_key,
        base_url=profile.get("base_url"),
        temperature=0.2,
        max_tokens=8192,
    )


class LlmRouter:
    def __init__(self, profile: dict[str, Any] | None = None) -> None:
        self.profile = profile or load_active_profile()
        self.config = profile_to_config(self.profile)

    def _is_anthropic(self) -> bool:
        if not self.profile:
            return False
        p = self.profile.get("protocol", "")
        return p in ("anthropic_messages", "anthropic")

    async def stream_chat(
        self,
        messages: list[dict[str, Any]],
        tools: list[dict[str, Any]] | None = None,
    ) -> AsyncIterator[dict[str, Any]]:
        if not self.config.api_key:
            from nexus_engine.llm.api_keys import require_api_key

            self.config.api_key = require_api_key(self.profile)
        if self._is_anthropic():
            client = AnthropicMessagesClient(self.config)
        else:
            client = OpenAICompatClient(self.config)
        async for chunk in client.stream_chat(messages, tools=tools):
            yield chunk

    async def complete_chat(
        self,
        messages: list[dict[str, Any]],
        tools: list[dict[str, Any]] | None = None,
    ) -> str:
        """Non-streaming completion — accumulate deltas from stream API."""
        parts: list[str] = []
        async for chunk in self.stream_chat(messages, tools=tools):
            for choice in chunk.get("choices") or []:
                delta = choice.get("delta") or {}
                if content := delta.get("content"):
                    parts.append(content)
                msg = choice.get("message") or {}
                if content := msg.get("content"):
                    parts.append(content)
        return "".join(parts)
