"""Hierarchical memory: short-term + long-term summaries (ROADMAP v2 §3.2)."""

from __future__ import annotations

from collections import deque
from dataclasses import dataclass, field
from typing import Any


@dataclass
class HierarchicalMemory:
    short_term_max: int = 10
    short_term: deque[dict[str, Any]] = field(default_factory=lambda: deque(maxlen=10))
    long_term: list[str] = field(default_factory=list)
    facts: list[str] = field(default_factory=list)

    def add_message(self, message: dict[str, Any]) -> None:
        self.short_term.append(message)
        content = str(message.get("content", ""))
        if len(content) > 2000 and message.get("role") == "assistant":
            self._maybe_summarize(content)

    def _maybe_summarize(self, content: str) -> None:
        preview = content[:400].replace("\n", " ")
        summary = f"[Summary] {preview}…"
        if summary not in self.long_term:
            self.long_term.append(summary)
            if len(self.long_term) > 20:
                self.long_term = self.long_term[-20:]

    def add_fact(self, fact: str) -> None:
        if fact and fact not in self.facts:
            self.facts.append(fact)
            if len(self.facts) > 50:
                self.facts = self.facts[-50:]

    def retrieve(self, query: str = "", k: int = 5) -> list[str]:
        """Priority: short-term snippets, matching facts, long-term summaries."""
        out: list[str] = []
        q = query.lower()
        for m in list(self.short_term)[-k:]:
            c = str(m.get("content", ""))[:200]
            if c:
                out.append(f"[recent {m.get('role')}] {c}")
        for f in self.facts:
            if not q or q in f.lower():
                out.append(f"[fact] {f}")
                if len(out) >= k:
                    return out
        for s in reversed(self.long_term):
            out.append(s)
            if len(out) >= k:
                break
        return out[:k]

    def inject_system_context(self) -> str:
        parts = []
        if self.long_term:
            parts.append("## Session memory (compressed)\n" + "\n".join(self.long_term[-5:]))
        if self.facts:
            parts.append("## Key facts\n" + "\n".join(f"- {f}" for f in self.facts[-10:]))
        return "\n\n".join(parts)
