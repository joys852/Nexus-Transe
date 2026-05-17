"""Metrics and optional Sentry initialization."""

from __future__ import annotations

import os
import time
from typing import Any

_start = time.monotonic()
_counters: dict[str, int] = {
    "nexus_chat_requests_total": 0,
    "nexus_tool_bridge_total": 0,
    "nexus_errors_total": 0,
}


def inc(name: str, n: int = 1) -> None:
    _counters[name] = _counters.get(name, 0) + n


def maybe_init_sentry() -> None:
    dsn = os.getenv("NEXUS_SENTRY_DSN")
    if not dsn:
        return
    try:
        import sentry_sdk

        sentry_sdk.init(dsn=dsn, traces_sample_rate=float(os.getenv("NEXUS_SENTRY_TRACES", "0.1")))
    except ImportError:
        pass


def prometheus_text() -> str:
    uptime = time.monotonic() - _start
    lines = [
        "# HELP nexus_uptime_seconds Engine process uptime",
        "# TYPE nexus_uptime_seconds gauge",
        f"nexus_uptime_seconds {uptime:.3f}",
    ]
    for name, value in sorted(_counters.items()):
        lines.append(f"# TYPE {name} counter")
        lines.append(f"{name} {value}")
    return "\n".join(lines) + "\n"
