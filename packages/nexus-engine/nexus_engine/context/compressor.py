"""Token-budget context compression for long sessions."""

from __future__ import annotations

from pydantic import BaseModel, Field


class MessageLike(BaseModel):
    role: str
    content: str
    priority: int = 5


class ContextCompressor(BaseModel):
    max_chars: int = 96_000
    reserve_tools: int = 12_000
    tail_count: int = 12

    def compress_messages(
        self, messages: list[dict], *, summary_override: str | None = None
    ) -> list[dict]:
        """Return messages fitting in budget; summarize dropped middle."""
        limit = self.max_chars - self.reserve_tools
        total = sum(len(str(m.get("content", ""))) for m in messages)
        if total <= limit:
            return messages

        if not messages:
            return messages

        out: list[dict] = []
        head = 0
        if messages[0].get("role") == "system":
            out.append(messages[0])
            head = 1

        tail = messages[-self.tail_count :]
        omitted = messages[head : len(messages) - len(tail)]
        if omitted:
            body = summary_override or self._summarize_omitted(omitted)
            out.append({"role": "system", "content": body})
        for m in tail:
            if m not in out:
                out.append(m)
        return out

    def _summarize_omitted(self, omitted: list[dict]) -> str:
        """Build a compact narrative of dropped turns (Claude-style middle compression)."""
        lines = [f"[Context compacted: {len(omitted)} earlier messages summarized]"]
        for m in omitted[-8:]:
            role = m.get("role", "?")
            raw = str(m.get("content", "")).replace("\n", " ").strip()
            if not raw:
                continue
            preview = raw[:180] + ("…" if len(raw) > 180 else "")
            lines.append(f"- {role}: {preview}")
        if len(omitted) > 8:
            lines.insert(1, f"  … and {len(omitted) - 8} more turns")
        return "\n".join(lines)
