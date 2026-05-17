"""WebSocket relay for workspace collaboration (presence / broadcast)."""

from __future__ import annotations

import json
import logging
from collections import defaultdict
from typing import Any

from fastapi import WebSocket, WebSocketDisconnect

logger = logging.getLogger(__name__)


class CollabHub:
    def __init__(self) -> None:
        self._rooms: dict[str, set[WebSocket]] = defaultdict(set)

    def peer_count(self, workspace: str) -> int:
        return len(self._rooms.get(workspace, set()))

    async def connect(self, websocket: WebSocket, workspace: str) -> None:
        await websocket.accept()
        room = self._rooms[workspace]
        room.add(websocket)
        await websocket.send_json(
            {
                "type": "joined",
                "workspace": workspace,
                "peers": len(room),
            }
        )
        await self._broadcast(
            workspace,
            {
                "type": "peer_joined",
                "workspace": workspace,
                "peers": len(room),
            },
            exclude=websocket,
        )
        try:
            while True:
                raw = await websocket.receive_text()
                try:
                    msg: dict[str, Any] = json.loads(raw)
                except json.JSONDecodeError:
                    msg = {"type": "message", "text": raw}
                msg.setdefault("workspace", workspace)
                await self._broadcast(workspace, msg, exclude=None)
        except WebSocketDisconnect:
            pass
        finally:
            room.discard(websocket)
            await self._broadcast(
                workspace,
                {
                    "type": "peer_left",
                    "workspace": workspace,
                    "peers": len(room),
                },
                exclude=None,
            )

    async def _broadcast(
        self,
        workspace: str,
        payload: dict[str, Any],
        *,
        exclude: WebSocket | None,
    ) -> None:
        dead: list[WebSocket] = []
        for ws in list(self._rooms.get(workspace, set())):
            if exclude is not None and ws is exclude:
                continue
            try:
                await ws.send_json(payload)
            except Exception:
                dead.append(ws)
        for ws in dead:
            self._rooms[workspace].discard(ws)


hub = CollabHub()
