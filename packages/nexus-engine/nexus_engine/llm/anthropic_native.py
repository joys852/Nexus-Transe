"""Anthropic Messages API (native) — for official API and Anthropic-compatible relays."""

from __future__ import annotations

import json
from typing import Any, AsyncIterator, Union

import httpx

from nexus_engine.llm.provider import LLMConfig


class AnthropicMessagesClient:
    def __init__(self, config: LLMConfig) -> None:
        self.config = config

    def _base(self) -> str:
        base = (self.config.base_url or "https://api.anthropic.com").rstrip("/")
        if base.endswith("/v1"):
            return base
        return f"{base}/v1"

    def _headers(self) -> dict[str, str]:
        h = {
            "Content-Type": "application/json",
            "anthropic-version": "2023-06-01",
        }
        if self.config.api_key:
            h["x-api-key"] = self.config.api_key
            h["Authorization"] = f"Bearer {self.config.api_key}"
        return h

    def _normalize_content(self, content: Union[str, list, None]) -> Union[str, list[dict[str, Any]]]:
        if content is None:
            return ""
        if isinstance(content, str):
            return content
        if not isinstance(content, list):
            return str(content)
        blocks: list[dict[str, Any]] = []
        for part in content:
            if not isinstance(part, dict):
                continue
            kind = part.get("type")
            if kind == "text":
                blocks.append({"type": "text", "text": part.get("text", "")})
            elif kind == "image_url":
                url_obj = part.get("image_url") or {}
                url = str(url_obj.get("url", ""))
                if url.startswith("data:"):
                    header, _, b64 = url.partition(",")
                    mime = "image/png"
                    if "image/" in header:
                        mime = header.split(";")[0].replace("data:", "")
                    blocks.append(
                        {
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": mime,
                                "data": b64,
                            },
                        }
                    )
                elif url:
                    blocks.append(
                        {
                            "type": "image",
                            "source": {"type": "url", "url": url},
                        }
                    )
        return blocks if blocks else ""

    def _to_anthropic_messages(self, messages: list[dict[str, Any]]) -> tuple[str | None, list[dict]]:
        system_parts = []
        out = []
        for m in messages:
            role = m.get("role", "user")
            content = m.get("content")
            if role == "system":
                if isinstance(content, str):
                    system_parts.append(content)
                else:
                    system_parts.append(str(content))
            elif role == "tool":
                text = content if isinstance(content, str) else str(content)
                out.append({"role": "user", "content": f"[tool result]\n{text}"})
            else:
                ar = "assistant" if role == "assistant" else "user"
                out.append({"role": ar, "content": self._normalize_content(content)})
        system = "\n\n".join(system_parts) if system_parts else None
        return system, out

    async def stream_chat(
        self,
        messages: list[dict[str, Any]],
        tools: list[dict[str, Any]] | None = None,
    ) -> AsyncIterator[dict[str, Any]]:
        system, msg_list = self._to_anthropic_messages(messages)
        url = f"{self._base()}/messages"
        payload: dict[str, Any] = {
            "model": self.config.model,
            "max_tokens": self.config.max_tokens or 8192,
            "messages": msg_list,
            "stream": True,
        }
        if system:
            payload["system"] = system
        if tools:
            payload["tools"] = [
                {
                    "name": t["function"]["name"],
                    "description": t["function"].get("description", ""),
                    "input_schema": t["function"].get("parameters", {"type": "object"}),
                }
                for t in tools
                if t.get("type") == "function"
            ]

        if not self.config.api_key:
            raise ValueError(
                "API key missing — run: nexus provider doctor\n"
                "Set ANTHROPIC_API_KEY / GPT_AGENT_API_KEY or: nexus secrets set --provider gpt-agent-glm --key YOUR_KEY"
            )

        async with httpx.AsyncClient(timeout=120.0) as client:
            async with client.stream(
                "POST", url, headers=self._headers(), json=payload
            ) as response:
                if response.status_code == 401:
                    raise ValueError(
                        f"401 Unauthorized for {url} — API key rejected. "
                        f"Run: nexus provider doctor — use the gpt-agent.cc key (not Anthropic official)."
                    )
                response.raise_for_status()
                tool_blocks: dict[int, dict[str, Any]] = {}

                async for line in response.aiter_lines():
                    if not line.startswith("data:"):
                        continue
                    data = line[5:].strip()
                    if not data:
                        continue
                    event = json.loads(data)
                    et = event.get("type")

                    if et == "content_block_start":
                        block = event.get("content_block") or {}
                        if block.get("type") == "tool_use":
                            idx = event.get("index", len(tool_blocks))
                            tool_blocks[idx] = {
                                "id": block.get("id", ""),
                                "name": block.get("name", ""),
                                "arguments": "",
                            }
                    elif et == "content_block_delta":
                        delta = event.get("delta", {})
                        if delta.get("type") == "text_delta":
                            text = delta.get("text", "")
                            yield {
                                "choices": [{"delta": {"content": text}}],
                            }
                        elif delta.get("type") == "input_json_delta":
                            idx = event.get("index", 0)
                            entry = tool_blocks.setdefault(
                                idx, {"id": "", "name": "", "arguments": ""}
                            )
                            entry["arguments"] += delta.get("partial_json", "")
                    elif et == "content_block_stop":
                        idx = event.get("index", 0)
                        entry = tool_blocks.get(idx)
                        if entry and entry.get("name"):
                            yield {
                                "choices": [
                                    {
                                        "delta": {
                                            "tool_calls": [
                                                {
                                                    "index": idx,
                                                    "id": entry["id"],
                                                    "function": {
                                                        "name": entry["name"],
                                                        "arguments": entry["arguments"] or "{}",
                                                    },
                                                }
                                            ]
                                        }
                                    }
                                ],
                            }
                    elif et == "message_stop":
                        break
