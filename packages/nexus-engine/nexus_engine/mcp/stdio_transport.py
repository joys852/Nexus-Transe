"""Full-duplex MCP stdio transport — Content-Length framing (MCP spec) + NDJSON fallback."""

from __future__ import annotations

import asyncio
import json
import os
import uuid
from typing import Any, Mapping, Optional


def _encode_message(obj: dict[str, Any]) -> bytes:
    body = json.dumps(obj, separators=(",", ":")).encode("utf-8")
    header = f"Content-Length: {len(body)}\r\n\r\n".encode("ascii")
    return header + body


def _try_parse_buffer(buf: bytes) -> tuple[list[dict[str, Any]], bytes]:
    """Parse one or more JSON-RPC messages from buffer."""
    messages: list[dict[str, Any]] = []
    while buf:
        if buf.startswith(b"Content-Length:") or buf.startswith(b"content-length:"):
            end_hdr = buf.find(b"\r\n\r\n")
            if end_hdr < 0:
                break
            header = buf[:end_hdr].decode("ascii", errors="replace")
            length = 0
            for part in header.split("\r\n"):
                if ":" in part:
                    k, v = part.split(":", 1)
                    if k.strip().lower() == "content-length":
                        length = int(v.strip())
            if length <= 0:
                buf = buf[end_hdr + 4 :]
                continue
            start = end_hdr + 4
            if len(buf) < start + length:
                break
            body = buf[start : start + length]
            buf = buf[start + length :]
            try:
                messages.append(json.loads(body.decode("utf-8")))
            except json.JSONDecodeError:
                continue
            continue

        if b"\n" in buf:
            line, _, buf = buf.partition(b"\n")
            text = line.decode("utf-8", errors="replace").strip()
            if text:
                try:
                    messages.append(json.loads(text))
                except json.JSONDecodeError:
                    pass
            continue
        break
    return messages, buf


class McpStdioTransport:
    """MCP over subprocess stdin/stdout with Content-Length framing."""

    def __init__(
        self,
        cmd: list[str],
        *,
        cwd: str | None = None,
        env: Mapping[str, str] | None = None,
    ) -> None:
        self.cmd = cmd
        self.cwd = cwd
        self.env = dict(env) if env else None
        self.proc: Optional[asyncio.subprocess.Process] = None
        self.pending: dict[str, asyncio.Future[dict[str, Any]]] = {}
        self._reader_task: Optional[asyncio.Task[None]] = None
        self._read_buf = b""

    async def start(self) -> None:
        if self.proc is not None:
            return
        child_env = None
        if self.env:
            child_env = {**os.environ, **self.env}
        self.proc = await asyncio.create_subprocess_exec(
            *self.cmd,
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            cwd=self.cwd,
            env=child_env,
        )
        self._reader_task = asyncio.create_task(self._read_loop())

    async def stop(self) -> None:
        if self._reader_task:
            self._reader_task.cancel()
            try:
                await self._reader_task
            except asyncio.CancelledError:
                pass
        if self.proc and self.proc.returncode is None:
            self.proc.terminate()
            try:
                await asyncio.wait_for(self.proc.wait(), timeout=3)
            except TimeoutError:
                self.proc.kill()
        self.proc = None
        self._read_buf = b""

    async def _read_loop(self) -> None:
        assert self.proc and self.proc.stdout
        while True:
            chunk = await self.proc.stdout.read(65536)
            if not chunk:
                break
            self._read_buf += chunk
            parsed, self._read_buf = _try_parse_buffer(self._read_buf)
            for msg in parsed:
                self._dispatch(msg)

    def _dispatch(self, msg: dict[str, Any]) -> None:
        msg_id = msg.get("id")
        if msg_id is not None and str(msg_id) in self.pending:
            fut = self.pending.pop(str(msg_id))
            if not fut.done():
                fut.set_result(msg)

    async def _write(self, payload: dict[str, Any]) -> None:
        if not self.proc or not self.proc.stdin:
            raise RuntimeError("MCP transport not started")
        self.proc.stdin.write(_encode_message(payload))
        await self.proc.stdin.drain()

    async def send_request(
        self,
        method: str,
        params: dict[str, Any] | None = None,
        *,
        timeout: float = 120.0,
    ) -> dict[str, Any]:
        msg_id = str(uuid.uuid4())
        loop = asyncio.get_event_loop()
        fut: asyncio.Future[dict[str, Any]] = loop.create_future()
        self.pending[msg_id] = fut
        await self._write(
            {
                "jsonrpc": "2.0",
                "id": msg_id,
                "method": method,
                "params": params or {},
            }
        )
        try:
            return await asyncio.wait_for(fut, timeout=timeout)
        except TimeoutError:
            self.pending.pop(msg_id, None)
            raise TimeoutError(f"MCP request timed out: {method}") from None

    async def send_notification(
        self, method: str, params: dict[str, Any] | None = None
    ) -> None:
        await self._write({"jsonrpc": "2.0", "method": method, "params": params or {}})

    async def initialize(self) -> dict[str, Any]:
        return await self.send_request(
            "initialize",
            {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "nexus-engine", "version": "1.0.0"},
            },
        )

    async def list_tools(self) -> list[dict[str, Any]]:
        res = await self.send_request("tools/list", {})
        return list(res.get("result", {}).get("tools", []))

    async def call_tool(self, name: str, arguments: dict[str, Any]) -> dict[str, Any]:
        res = await self.send_request(
            "tools/call",
            {"name": name, "arguments": arguments},
        )
        return res.get("result", res)
