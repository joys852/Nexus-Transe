"""MCP server registry — load config, discover tools, invoke calls."""

from __future__ import annotations

import logging
import os
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from nexus_engine.mcp.bridge import merge_tool_definitions, mcp_tool_to_openai
from nexus_engine.mcp.names import decode_tool_name, encode_tool_name
from nexus_engine.mcp.stdio_transport import McpStdioTransport

logger = logging.getLogger(__name__)


def _data_dir() -> Path:
    if p := os.getenv("NEXUS_DATA_DIR"):
        return Path(p)
    if os.name == "nt":
        appdata = os.environ.get("APPDATA")
        if appdata:
            return Path(appdata) / "nexus-ide"
    home = Path(os.environ.get("HOME") or ".")
    return home / ".local" / "share" / "nexus-ide"


def discover_mcp_config_paths(workspace_root: str | None = None) -> list[Path]:
    paths: list[Path] = []
    if workspace_root:
        paths.append(Path(workspace_root) / ".nexus" / "mcp.toml")
    paths.append(_data_dir() / "mcp.toml")
    paths.append(Path.home() / ".nexus" / "mcp.toml")
    seen: set[Path] = set()
    out: list[Path] = []
    for p in paths:
        rp = p.resolve()
        if rp not in seen and p.exists():
            seen.add(rp)
            out.append(p)
    return out


@dataclass
class McpServerState:
    name: str
    command: str
    args: list[str]
    enabled: bool
    env: dict[str, str] = field(default_factory=dict)
    transport: McpStdioTransport | None = None
    tools: list[dict[str, Any]] = field(default_factory=list)


class McpRegistry:
    """Singleton MCP registry for the engine process."""

    def __init__(self) -> None:
        self.servers: dict[str, McpServerState] = {}
        self._openai_map: dict[str, tuple[str, str]] = {}
        self._loaded = False
        self._workspace: str | None = None

    async def ensure_loaded(self, workspace_root: str | None = None) -> None:
        if self._loaded and (workspace_root is None or workspace_root == self._workspace):
            return
        if workspace_root:
            self._workspace = workspace_root
        await self._load_configs(workspace_root)
        await self._connect_enabled()
        self._loaded = True

    async def reload(self, workspace_root: str | None = None) -> None:
        self._loaded = False
        for st in self.servers.values():
            if st.transport:
                await st.transport.stop()
        self.servers.clear()
        self._openai_map.clear()
        await self.ensure_loaded(workspace_root)

    async def _load_configs(self, workspace_root: str | None) -> None:
        import tomllib

        for path in discover_mcp_config_paths(workspace_root):
            try:
                data = tomllib.loads(path.read_text(encoding="utf-8"))
            except Exception as e:
                logger.warning("mcp config %s: %s", path, e)
                continue
            for raw in data.get("servers", []):
                name = raw.get("name") or "unnamed"
                if name in self.servers:
                    continue
                env_raw = raw.get("env") or {}
                env = {str(k): str(v) for k, v in env_raw.items()} if isinstance(env_raw, dict) else {}
                self.servers[name] = McpServerState(
                    name=name,
                    command=raw.get("command", ""),
                    args=list(raw.get("args") or []),
                    enabled=bool(raw.get("enabled", True)),
                    env=env,
                )

    async def _connect_enabled(self) -> None:
        cwd = self._workspace
        for st in self.servers.values():
            if not st.enabled or not st.command:
                continue
            if st.transport is not None:
                continue
            cmd = [st.command, *st.args]
            try:
                transport = McpStdioTransport(cmd, cwd=cwd, env=st.env or None)
                await transport.start()
                await transport.initialize()
                await transport.send_notification("notifications/initialized", {})
                tools = await transport.list_tools()
                st.transport = transport
                st.tools = tools
                for t in tools:
                    openai = encode_tool_name(st.name, t.get("name", "unknown"))
                    self._openai_map[openai] = (st.name, t.get("name", "unknown"))
                logger.info("MCP %s: %d tools", st.name, len(tools))
            except Exception as e:
                logger.warning("MCP server %s failed: %s", st.name, e)

    def resolve_openai_name(self, openai_name: str) -> tuple[str, str] | None:
        if openai_name in self._openai_map:
            return self._openai_map[openai_name]
        return decode_tool_name(openai_name)

    def list_tools_flat(self) -> list[dict[str, Any]]:
        out: list[dict[str, Any]] = []
        for st in self.servers.values():
            for t in st.tools:
                out.append(
                    {
                        "server": st.name,
                        "name": t.get("name", "unknown"),
                        "description": t.get("description", ""),
                        "input_schema": t.get("inputSchema")
                        or t.get("input_schema")
                        or {"type": "object"},
                    }
                )
        return out

    def status(self) -> list[dict[str, Any]]:
        return [
            {
                "name": st.name,
                "enabled": st.enabled,
                "connected": st.transport is not None,
                "tools": len(st.tools),
            }
            for st in self.servers.values()
        ]

    async def call_tool(
        self, server: str, tool: str, arguments: dict[str, Any]
    ) -> dict[str, Any]:
        st = self.servers.get(server)
        if not st or not st.transport:
            raise RuntimeError(f"MCP server not connected: {server}")
        return await st.transport.call_tool(tool, arguments)


_registry: McpRegistry | None = None


def get_mcp_registry() -> McpRegistry:
    global _registry
    if _registry is None:
        _registry = McpRegistry()
    return _registry


async def get_tool_definitions(
    builtin: list[dict[str, Any]],
    workspace_root: str | None = None,
) -> list[dict[str, Any]]:
    reg = get_mcp_registry()
    await reg.ensure_loaded(workspace_root)
    mcp_tools = reg.list_tools_flat()
    if not mcp_tools:
        return builtin
    return merge_tool_definitions(builtin, mcp_tools)


def enrich_tool_call(call: dict[str, Any]) -> dict[str, Any]:
    """Attach mcp_server / mcp_tool when the OpenAI name is an MCP tool."""
    reg = get_mcp_registry()
    resolved = reg.resolve_openai_name(call.get("tool_name", ""))
    if resolved:
        server, tool = resolved
        call = dict(call)
        call["mcp_server"] = server
        call["mcp_tool"] = tool
    return call
