"""OpenAI-compatible chat completions (streaming) — works with OpenAI, DeepSeek, Ollama, proxies."""

from __future__ import annotations

import json
import os
from typing import Any, AsyncIterator

import httpx

from nexus_engine.llm.provider import ChatMessage, LLMConfig


def config_from_env() -> LLMConfig:
    provider = os.getenv("NEXUS_LLM_PROVIDER", "openai_compatible")
    model = os.getenv("NEXUS_MODEL", os.getenv("OPENAI_MODEL", "gpt-4o-mini"))
    api_key = os.getenv("OPENAI_API_KEY") or os.getenv("NEXUS_API_KEY")
    base_url = os.getenv(
        "NEXUS_API_BASE",
        os.getenv("OPENAI_BASE_URL", "https://api.openai.com/v1"),
    )
    return LLMConfig(provider=provider, model=model, api_key=api_key, base_url=base_url)


class OpenAICompatClient:
    def __init__(self, config: LLMConfig | None = None) -> None:
        self.config = config or config_from_env()

    def _headers(self) -> dict[str, str]:
        h = {"Content-Type": "application/json"}
        if self.config.api_key:
            h["Authorization"] = f"Bearer {self.config.api_key}"
        return h

    async def stream_chat(
        self,
        messages: list[ChatMessage] | list[dict[str, Any]],
        tools: list[dict[str, Any]] | None = None,
    ) -> AsyncIterator[dict[str, Any]]:
        url = f"{self.config.base_url.rstrip('/')}/chat/completions"
        msg_payload = [
            m.model_dump() if isinstance(m, ChatMessage) else m for m in messages
        ]
        payload: dict[str, Any] = {
            "model": self.config.model,
            "messages": msg_payload,
            "stream": True,
            "temperature": self.config.temperature,
        }
        if tools:
            payload["tools"] = tools
            payload["tool_choice"] = "auto"

        async with httpx.AsyncClient(timeout=120.0) as client:
            async with client.stream(
                "POST", url, headers=self._headers(), json=payload
            ) as response:
                response.raise_for_status()
                async for line in response.aiter_lines():
                    if not line.startswith("data:"):
                        continue
                    data = line[5:].strip()
                    if data == "[DONE]":
                        break
                    chunk = json.loads(data)
                    yield chunk
