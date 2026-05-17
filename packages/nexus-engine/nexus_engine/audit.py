"""Append-only audit log for tool / workspace operations."""

from __future__ import annotations

import json
import os
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def _audit_path(workspace_root: str) -> Path:
    return Path(workspace_root) / ".nexus" / "audit.jsonl"


def log_event(workspace_root: str, event: str, payload: dict[str, Any]) -> None:
    path = _audit_path(workspace_root)
    path.parent.mkdir(parents=True, exist_ok=True)
    row = {
        "ts": datetime.now(timezone.utc).isoformat(),
        "event": event,
        "user": os.environ.get("USERNAME") or os.environ.get("USER") or "unknown",
        **payload,
    }
    with path.open("a", encoding="utf-8") as f:
        f.write(json.dumps(row, ensure_ascii=False) + "\n")


def tail_events(workspace_root: str, limit: int = 50) -> list[dict[str, Any]]:
    path = _audit_path(workspace_root)
    if not path.is_file():
        return []
    lines = path.read_text(encoding="utf-8").strip().splitlines()
    out: list[dict[str, Any]] = []
    for line in lines[-limit:]:
        try:
            out.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return out
