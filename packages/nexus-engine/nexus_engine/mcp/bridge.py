"""Bridge MCP tool definitions into OpenAI tool schema."""

from __future__ import annotations

from typing import Any

from nexus_engine.mcp.names import encode_tool_name


def mcp_tool_to_openai(server: str, tool: dict[str, Any]) -> dict[str, Any]:
    name = tool.get("name", "unknown")
    return {
        "type": "function",
        "function": {
            "name": encode_tool_name(server, name),
            "description": f"[MCP:{server}] {tool.get('description', '')}",
            "parameters": tool.get("input_schema") or tool.get("inputSchema") or {"type": "object"},
        },
    }


def merge_tool_definitions(
    builtin: list[dict[str, Any]],
    mcp_tools: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    out = list(builtin)
    for t in mcp_tools:
        server = t.get("server", "default")
        out.append(mcp_tool_to_openai(server, t))
    return out
