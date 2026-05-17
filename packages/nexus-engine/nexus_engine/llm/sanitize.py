"""Strip GLM/Kimi tool markup from model text and recover tool calls when APIs omit structured tools."""

from __future__ import annotations

import json
import re
import uuid
from typing import Any

# GLM / Kimi streamed control tokens (several relay variants)
_GLM_SECTION = re.compile(
    r"<\|(?:redacted_)?tool_calls_section_begin\|>.*?<\|(?:redacted_)?tool_calls_section_end\|>",
    re.DOTALL,
)
_GLM_CALL = re.compile(
    r"<\|(?:redacted_)?tool_call(?:_begin(?:_kimi)?)?\|>\s*([\w.]+):(\d+)\s*"
    r"<\|(?:redacted_)?tool_call_argument(?:_begin)?\|>\s*(\{.*?\})\s*"
    r"<\|(?:redacted_)?tool_call(?:_end(?:_kimi)?)?\|>",
    re.DOTALL,
)

# <tool_call>Bash multiline_mode="false" command="ls -la" description="..."</tool_call>
_XML_TOOL = re.compile(
    r"<tool_call>\s*(\w+)[^>]*?(?:command|cmd)=\"([^\"]*)\"[^>]*?</tool_call>",
    re.DOTALL | re.IGNORECASE,
)

# Leftover partial tags
_STRAY_TAG = re.compile(r"<\|/?tool[_\w]*\|>", re.IGNORECASE)
_STRAY_XML = re.compile(r"</?tool_call[^>]*>", re.IGNORECASE)


def _map_tool_name(name: str) -> str:
    n = name.split(".")[-1].strip().lower()
    aliases = {
        "bash": "run_shell",
        "shell": "run_shell",
        "run_bash": "run_shell",
        "read": "read_file",
        "write": "write_file",
        "edit": "edit_file",
        "glob": "glob_files",
    }
    return aliases.get(n, n)


def extract_tool_calls_from_text(text: str) -> tuple[str, list[dict[str, Any]]]:
    """Return visible text and parsed tool calls {call_id, tool_name, arguments}."""
    calls: list[dict[str, Any]] = []
    cleaned = text

    for m in _GLM_CALL.finditer(cleaned):
        raw_name, _idx, args_json = m.group(1), m.group(2), m.group(3)
        try:
            args = json.loads(args_json)
        except json.JSONDecodeError:
            args = {"raw": args_json}
        calls.append(
            {
                "call_id": str(uuid.uuid4()),
                "tool_name": _map_tool_name(raw_name),
                "arguments": args,
            }
        )

    for m in _XML_TOOL.finditer(cleaned):
        raw_name, command = m.group(1), m.group(2)
        calls.append(
            {
                "call_id": str(uuid.uuid4()),
                "tool_name": _map_tool_name(raw_name),
                "arguments": {"command": command},
            }
        )

    cleaned = _GLM_SECTION.sub("", cleaned)
    cleaned = _GLM_CALL.sub("", cleaned)
    cleaned = _XML_TOOL.sub("", cleaned)
    cleaned = _STRAY_TAG.sub("", cleaned)
    cleaned = _STRAY_XML.sub("", cleaned)
    cleaned = re.sub(r"\n{3,}", "\n\n", cleaned).strip()
    return cleaned, calls


def sanitize_stream_delta(delta: str) -> str:
    """Remove tool markup from a single streamed chunk (best-effort)."""
    if not delta:
        return ""
    if "<|tool" in delta or "<tool_call" in delta.lower():
        cleaned, _ = extract_tool_calls_from_text(delta)
        return cleaned
    return delta


class StreamSanitizer:
    """Hold back partial tool markup so raw tokens are not printed mid-stream."""

    def __init__(self) -> None:
        self._buf = ""

    def feed(self, chunk: str) -> str:
        if not chunk:
            return ""
        self._buf += chunk
        if _GLM_SECTION.search(self._buf):
            clean, _ = extract_tool_calls_from_text(self._buf)
            self._buf = ""
            return clean
        if "<|tool" in self._buf or "<tool_call" in self._buf.lower():
            for marker in (
                "<|tool_calls_section_begin|>",
                "<|redacted_tool_call_begin",
                "<tool_call",
            ):
                pos = self._buf.find(marker)
                if pos > 0:
                    emit, self._buf = self._buf[:pos], self._buf[pos:]
                    return emit
            return ""
        out, self._buf = self._buf, ""
        return out

    def flush(self) -> str:
        clean, _ = extract_tool_calls_from_text(self._buf)
        self._buf = ""
        return clean
