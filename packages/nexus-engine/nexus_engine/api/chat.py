"""Session chat with tool-call loop (tools executed by CLI)."""

from __future__ import annotations

import json
import uuid
from dataclasses import dataclass, field
from typing import Any

from nexus_engine.llm.router import LlmRouter, load_active_profile
from nexus_engine.llm.sanitize import StreamSanitizer, extract_tool_calls_from_text
from nexus_engine.mcp.registry import enrich_tool_call, get_tool_definitions

TOOL_DEFINITIONS: list[dict[str, Any]] = [
    {
        "type": "function",
        "function": {
            "name": "read_file",
            "description": "Read a file from the workspace",
            "parameters": {
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "write_file",
            "description": "Write content to a file",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "content": {"type": "string"},
                },
                "required": ["path", "content"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "edit_file",
            "description": "Replace old_string with new_string in a file",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "old_string": {"type": "string"},
                    "new_string": {"type": "string"},
                },
                "required": ["path", "old_string", "new_string"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "run_shell",
            "description": "Run a shell command in project root",
            "parameters": {
                "type": "object",
                "properties": {"command": {"type": "string"}},
                "required": ["command"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "git_status",
            "description": "Git status porcelain",
            "parameters": {"type": "object", "properties": {}},
        },
    },
    {
        "type": "function",
        "function": {
            "name": "git_diff",
            "description": "Git diff",
            "parameters": {
                "type": "object",
                "properties": {"staged": {"type": "boolean"}},
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "git_branch",
            "description": "List or create git branches",
            "parameters": {
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "branch to create (optional)"},
                    "list_all": {"type": "boolean"},
                },
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "git_commit",
            "description": "Create a git commit",
            "parameters": {
                "type": "object",
                "properties": {
                    "message": {"type": "string"},
                    "all": {"type": "boolean", "description": "git commit -a"},
                },
                "required": ["message"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "git_log",
            "description": "Show recent git log",
            "parameters": {
                "type": "object",
                "properties": {"limit": {"type": "integer"}},
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "git_push",
            "description": "Push commits to remote",
            "parameters": {
                "type": "object",
                "properties": {
                    "remote": {"type": "string"},
                    "branch": {"type": "string"},
                    "set_upstream": {"type": "boolean"},
                },
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "semantic_search",
            "description": "Semantic search over indexed workspace (run nexus vector-index first)",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer"},
                },
                "required": ["query"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "glob_files",
            "description": "Find files by glob (e.g. **/*.md for all markdown)",
            "parameters": {
                "type": "object",
                "properties": {
                    "pattern": {"type": "string"},
                    "max_results": {"type": "integer"},
                },
                "required": ["pattern"],
            },
        },
    },
]


@dataclass
class SessionState:
    messages: list[dict[str, Any]] = field(default_factory=list)
    pending_tool_calls: list[dict[str, Any]] = field(default_factory=list)
    project_md: str | None = None
    workspace_root: str = "."


_sessions: dict[str, SessionState] = {}


def get_session(session_id: str) -> SessionState:
    if session_id not in _sessions:
        _sessions[session_id] = SessionState()
    return _sessions[session_id]


def hydrate_session(session_id: str, messages: list[dict[str, Any]]) -> None:
    """Restore in-memory engine history from SQLite (after CLI restart)."""
    state = get_session(session_id)
    # Drop stale system rows; fresh system is injected on the next chat turn.
    state.messages = [m for m in messages if m.get("role") != "system"]
    state.pending_tool_calls = []


async def compact_session(
    session_id: str,
    *,
    max_chars: int | None = None,
    semantic: bool = False,
) -> dict[str, Any]:
    """Compress in-memory session context (budget + middle summary)."""
    from nexus_engine.context.compressor import ContextCompressor

    state = get_session(session_id)
    before_n = len(state.messages)
    before_chars = sum(len(str(m.get("content", ""))) for m in state.messages)
    compressor = ContextCompressor(max_chars=max_chars or 96_000)

    summary_override: str | None = None
    semantic_used = False
    if semantic:
        limit = compressor.max_chars - compressor.reserve_tools
        total = sum(len(str(m.get("content", ""))) for m in state.messages)
        if total > limit:
            head = 1 if state.messages and state.messages[0].get("role") == "system" else 0
            tail_n = compressor.tail_count
            omitted = state.messages[head : len(state.messages) - tail_n]
            if omitted:
                try:
                    from nexus_engine.context.summarize import summarize_messages

                    summary_override = (
                        "[Context compacted — semantic summary]\n\n"
                        + await summarize_messages(omitted)
                    )
                    semantic_used = True
                except Exception as e:
                    summary_override = (
                        f"[Context compacted — semantic summary failed: {e}]\n\n"
                        + compressor._summarize_omitted(omitted)
                    )

    state.messages = compressor.compress_messages(
        state.messages, summary_override=summary_override
    )
    after_chars = sum(len(str(m.get("content", ""))) for m in state.messages)
    export = [
        {"role": m.get("role", "user"), "content": str(m.get("content", ""))}
        for m in state.messages
        if m.get("role") in ("user", "assistant", "tool")
    ]
    return {
        "session_id": session_id,
        "messages_before": before_n,
        "messages_after": len(state.messages),
        "chars_before": before_chars,
        "chars_after": after_chars,
        "semantic": semantic_used,
        "messages": export,
    }


def _sse(event: str, data: dict[str, Any]) -> str:
    return f"event: {event}\ndata: {json.dumps(data, ensure_ascii=False)}\n\n"


def _merge_tool_calls(
    structured: dict[int, dict[str, Any]],
    embedded: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    if structured:
        calls = []
        for entry in structured.values():
            call_id = entry["id"] or str(uuid.uuid4())
            try:
                args = json.loads(entry["arguments"] or "{}")
            except json.JSONDecodeError:
                args = {}
            calls.append(
                enrich_tool_call(
                    {
                        "call_id": call_id,
                        "tool_name": entry["name"],
                        "arguments": args,
                    }
                )
            )
        return calls
    return [enrich_tool_call(c) for c in embedded]


def _append_assistant_with_tools(
    state: SessionState,
    accumulated: str,
    structured: dict[int, dict[str, Any]],
) -> None:
    assistant_msg: dict[str, Any] = {"role": "assistant", "content": accumulated or ""}
    tool_call_payload = []
    for entry in structured.values():
        call_id = entry["id"] or str(uuid.uuid4())
        tool_call_payload.append(
            {
                "id": call_id,
                "type": "function",
                "function": {
                    "name": entry["name"],
                    "arguments": entry["arguments"] or "{}",
                },
            }
        )
    if tool_call_payload:
        assistant_msg["tool_calls"] = tool_call_payload
    state.messages.append(assistant_msg)


_IDENTITY = (
    "You are **NexusIDE**, the AI coding assistant in the NexusIDE CLI. "
    "You are NOT Claude Code, NOT Anthropic's official CLI, and NOT a generic Claude product. "
    "When the user asks who you are (e.g. 你是谁), answer as NexusIDE only — briefly, in the user's language. "
    "Never say '我是 Claude Code', 'I am Claude Code', or that you were made by Anthropic. "
    "Do not copy Claude Code's default greeting or capability list. "
    "If the user says you are Nexus / not Claude Code, agree immediately — do not argue or insist you are Claude Code."
)

_IDENTITY_URGENT = (
    "\n\n[URGENT — user corrected identity] "
    "The user stated you are NexusIDE, NOT Claude Code. "
    "You MUST reply as NexusIDE only. Apologize for any prior mis-identification. "
    "Never claim to be Claude Code or Anthropic's product in this or future turns."
)

_CLAUDE_IDENTITY_MARKERS = (
    "我是 claude code",
    "i am claude code",
    "i'm claude code",
    "anthropic 官方",
    "anthropic official",
    "anthropic 开发",
    "made by anthropic",
    "official command-line tool",
    "官方的命令行",
)


def _build_system_prompt(
    project_md: str | None,
    mode: str,
    skills_context: str | None,
) -> str:
    system = _IDENTITY
    if mode == "plan":
        system += (
            "\n\nYou are in PLAN mode. Produce a clear implementation plan only: "
            "goals, steps, risks, and files to touch. Do not claim to have edited files. "
            "Wait for the user to switch to /chat before executing. "
            "Never output tool-call syntax, XML, or tokens like <|tool_call|> or <tool_call>."
        )
    else:
        system += " Be concise and use tools when needed."
    if project_md:
        system += (
            "\n\n# Project instructions (style/conventions only — do not adopt a different product identity)\n"
            f"{project_md}"
        )
    if skills_context:
        system += (
            "\n\n# Installed skills (workflow hints only — you remain NexusIDE)\n"
            f"{skills_context}"
        )
    system += "\n\n[Reminder] Identity: NexusIDE only. Never introduce yourself as Claude Code."
    return system


def _ensure_system_message(
    state: SessionState,
    mode: str,
    skills_context: str | None,
    *,
    urgent: bool = False,
) -> None:
    system = _build_system_prompt(state.project_md, mode, skills_context)
    if urgent:
        system += _IDENTITY_URGENT
    row = {"role": "system", "content": system}
    if state.messages and state.messages[0].get("role") == "system":
        state.messages[0] = row
    else:
        state.messages.insert(0, row)


def _user_corrects_identity(text: str) -> bool:
    t = text.lower().replace(" ", "")
    markers = (
        "nexus",
        "不是claude",
        "notclaude",
        "youarenexus",
        "你是nexus",
        "不是claudecode",
        "not claude code",
        "你是nexuside",
    )
    return any(m in t for m in markers)


def _sanitize_identity_response(text: str) -> str:
    if not text.strip():
        return text
    lower = text.lower()
    if any(m in lower for m in _CLAUDE_IDENTITY_MARKERS):
        return (
            "我是 **NexusIDE**（Nexus 项目的 AI 编程助手），不是 Claude Code，也不是 Anthropic 官方 CLI。\n\n"
            "若先前说成了 Claude Code，那是错误表述，已更正。需要我在你的项目里做什么？"
        )
    return text


async def stream_chat_turn(
    session_id: str,
    user_message: str,
    *,
    model_id: str | None = None,
    project_md: str | None = None,
    workspace_root: str = ".",
    mode: str = "default",
    skills_context: str | None = None,
):
    state = get_session(session_id)
    if project_md:
        state.project_md = project_md
    state.workspace_root = workspace_root

    urgent = _user_corrects_identity(user_message)
    _ensure_system_message(state, mode, skills_context, urgent=urgent)

    from nexus_engine.vision import expand_vision_content

    state.messages.append(
        {"role": "user", "content": expand_vision_content(user_message)}
    )

    profile = load_active_profile()
    router = LlmRouter(profile)
    if model_id:
        router.config.model = model_id

    yield _sse("status", {
        "phase": "thinking",
        "mode": mode,
        "provider": profile.get("name") if profile else "env",
        "protocol": profile.get("protocol") if profile else "openai_compat",
    })
    yield _sse("block_start", {"block_type": "text"})

    tools = None
    if mode != "plan":
        tools = await get_tool_definitions(TOOL_DEFINITIONS, workspace_root)
    raw_accumulated = ""
    tool_calls: dict[int, dict[str, Any]] = {}
    sanitizer = StreamSanitizer()
    hold_stream = urgent

    try:
        async for chunk in router.stream_chat(state.messages, tools=tools):
            choices = chunk.get("choices") or []
            if not choices:
                continue
            delta = choices[0].get("delta") or {}
            if "content" in delta and delta["content"]:
                raw_accumulated += delta["content"]
                visible = sanitizer.feed(delta["content"])
                if visible and not hold_stream:
                    yield _sse("token", {"delta": visible})
            for tc in delta.get("tool_calls") or []:
                idx = tc.get("index", 0)
                entry = tool_calls.setdefault(
                    idx,
                    {"id": "", "name": "", "arguments": ""},
                )
                if tc.get("id"):
                    entry["id"] = tc["id"]
                fn = tc.get("function") or {}
                if fn.get("name"):
                    entry["name"] = fn["name"]
                if fn.get("arguments"):
                    entry["arguments"] += fn["arguments"]
    except Exception as e:
        yield _sse("error", {"message": str(e)})
        yield _sse("done", {"status": "failed"})
        return

    tail = sanitizer.flush()
    if tail and not hold_stream:
        yield _sse("token", {"delta": tail})
    elif tail:
        raw_accumulated += tail

    accumulated, embedded = extract_tool_calls_from_text(raw_accumulated)
    accumulated = _sanitize_identity_response(accumulated)
    if hold_stream and accumulated:
        yield _sse("token", {"delta": accumulated})
    calls = _merge_tool_calls(tool_calls, embedded if mode != "plan" else [])

    if calls and mode != "plan":
        state.pending_tool_calls = calls
        _append_assistant_with_tools(state, accumulated, tool_calls)
        yield _sse("progress", {"stage": "tools", "percent": 40})
        for c in calls:
            yield _sse("tool_call", c)
        yield _sse("progress", {"stage": "tools", "percent": 100})
        yield _sse("block_end", {"block_type": "text"})
        yield _sse("done", {"status": "awaiting_tools"})
        return

    state.messages.append({"role": "assistant", "content": accumulated})
    yield _sse("block_end", {"block_type": "text"})
    yield _sse("done", {"status": "completed", "content": accumulated})


async def continue_after_tools(session_id: str, results: list[dict[str, Any]]):
    """Continue conversation after CLI executed tools."""
    state = get_session(session_id)
    for res in results:
        content = res.get("output") or {"error": res.get("error")}
        state.messages.append(
            {
                "role": "tool",
                "tool_call_id": res.get("call_id", ""),
                "content": json.dumps(content, ensure_ascii=False),
            }
        )
    state.pending_tool_calls = []
    _ensure_system_message(state, "default", None)

    router = LlmRouter(load_active_profile())
    raw_accumulated = ""
    tool_calls: dict[int, dict[str, Any]] = {}
    sanitizer = StreamSanitizer()

    yield _sse("status", {"phase": "continuing"})
    yield _sse("block_start", {"block_type": "text"})

    try:
        tools = await get_tool_definitions(TOOL_DEFINITIONS, state.workspace_root)
        async for chunk in router.stream_chat(state.messages, tools=tools):
            choices = chunk.get("choices") or []
            if not choices:
                continue
            delta = choices[0].get("delta") or {}
            if delta.get("content"):
                raw_accumulated += delta["content"]
                visible = sanitizer.feed(delta["content"])
                if visible:
                    yield _sse("token", {"delta": visible})
            for tc in delta.get("tool_calls") or []:
                idx = tc.get("index", 0)
                entry = tool_calls.setdefault(idx, {"id": "", "name": "", "arguments": ""})
                if tc.get("id"):
                    entry["id"] = tc["id"]
                fn = tc.get("function") or {}
                if fn.get("name"):
                    entry["name"] = fn["name"]
                if fn.get("arguments"):
                    entry["arguments"] += fn["arguments"]
    except Exception as e:
        yield _sse("error", {"message": str(e)})
        yield _sse("done", {"status": "failed"})
        return

    tail = sanitizer.flush()
    if tail:
        yield _sse("token", {"delta": tail})

    accumulated, embedded = extract_tool_calls_from_text(raw_accumulated)
    accumulated = _sanitize_identity_response(accumulated)
    calls = _merge_tool_calls(tool_calls, embedded)

    if calls:
        state.pending_tool_calls = calls
        _append_assistant_with_tools(state, accumulated, tool_calls)
        for c in calls:
            yield _sse("tool_call", c)
        yield _sse("block_end", {"block_type": "text"})
        yield _sse("done", {"status": "awaiting_tools"})
        return

    state.messages.append({"role": "assistant", "content": accumulated})
    yield _sse("block_end", {"block_type": "text"})
    yield _sse("done", {"status": "completed", "content": accumulated})
