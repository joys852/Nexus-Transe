"""Parse [NEXUS_VISION] markers into OpenAI-compatible multimodal content."""

from __future__ import annotations

import re
from typing import Any

_VISION_RE = re.compile(
    r"\[NEXUS_VISION mime=([^\s]+) path=([^\]]+)\]\n([A-Za-z0-9+/=\s]+)",
    re.DOTALL,
)


def expand_vision_content(content: str) -> str | list[dict[str, Any]]:
    m = _VISION_RE.search(content)
    if not m:
        return content
    mime, path, b64 = m.group(1), m.group(2), m.group(3).strip().replace("\n", "")
    text_before = content[: m.start()].strip()
    parts: list[dict[str, Any]] = []
    if text_before:
        parts.append({"type": "text", "text": text_before})
    parts.append(
        {
            "type": "image_url",
            "image_url": {"url": f"data:{mime};base64,{b64}"},
        }
    )
    parts.append({"type": "text", "text": f"(image: {path})"})
    return parts
