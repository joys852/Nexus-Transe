"""OpenAI function name encoding for MCP tools."""

from __future__ import annotations


def encode_tool_name(server: str, tool: str) -> str:
    return f"mcp_{server}_{tool}".replace("-", "_")


def decode_tool_name(openai_name: str) -> tuple[str, str] | None:
    if not openai_name.startswith("mcp_"):
        return None
    rest = openai_name[4:]
    if "_" not in rest:
        return None
    server, tool = rest.split("_", 1)
    return server, tool.replace("_", "-")
