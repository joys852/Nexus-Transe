"""FastAPI sidecar — CLI calls via HTTP."""

from fastapi import FastAPI, Query, WebSocket
from fastapi.responses import PlainTextResponse, StreamingResponse
from pydantic import BaseModel, Field

from nexus_engine import __version__
from nexus_engine.agents.base import AgentContext
from nexus_engine.agents.orchestrator import MultiAgentOrchestrator
from nexus_engine.api.chat import (
    compact_session,
    continue_after_tools,
    hydrate_session,
    stream_chat_turn,
)
from nexus_engine.graph.react_loop import RunTaskInput, TaskStatus

app = FastAPI(title="Nexus Engine", version=__version__)


@app.on_event("startup")
def _startup() -> None:
    from nexus_engine.observability import maybe_init_sentry

    maybe_init_sentry()


class HealthResponse(BaseModel):
    ok: bool = True
    version: str


class RunTaskRequest(BaseModel):
    session_id: str
    prompt: str
    model_id: str | None = None
    agent_profile: str = "default"


class RunTaskResponse(BaseModel):
    task_id: str
    status: TaskStatus


class ChatRequest(BaseModel):
    message: str
    model_id: str | None = None
    provider_id: str | None = None
    project_md: str | None = None
    workspace_root: str = "."
    mode: str = "default"
    skills_context: str | None = None


class SetActiveProviderRequest(BaseModel):
    provider_id: str


class ToolResultItem(BaseModel):
    call_id: str
    tool_name: str
    output: dict | None = None
    error: str | None = None


class ToolResultsRequest(BaseModel):
    results: list[ToolResultItem]


class HydrateMessage(BaseModel):
    role: str
    content: str


class HydrateRequest(BaseModel):
    messages: list[HydrateMessage] = Field(default_factory=list)


class CompactRequest(BaseModel):
    max_chars: int | None = None
    semantic: bool = True


class McpCallRequest(BaseModel):
    server: str
    tool: str
    arguments: dict = Field(default_factory=dict)
    workspace_root: str | None = None


@app.get("/health", response_model=HealthResponse)
async def health() -> HealthResponse:
    return HealthResponse(version=__version__)


@app.post("/v1/tasks/run", response_model=RunTaskResponse)
async def run_task(body: RunTaskRequest) -> RunTaskResponse:
    _ = RunTaskInput(
        session_id=body.session_id,
        prompt=body.prompt,
        model_id=body.model_id,
        agent_profile=body.agent_profile,
    )
    return RunTaskResponse(task_id=body.session_id, status=TaskStatus.PLANNING)


@app.post("/v1/tasks/{session_id}/pause")
async def pause_task(session_id: str) -> dict[str, str]:
    return {"session_id": session_id, "status": TaskStatus.PAUSED.value}


@app.post("/v1/tasks/{session_id}/resume")
async def resume_task(session_id: str) -> dict[str, str]:
    return {"session_id": session_id, "status": TaskStatus.ACTING.value}


@app.post("/v1/providers/active")
async def set_active_provider(body: SetActiveProviderRequest):
    from nexus_engine.llm.router import _providers_path

    import re
    import tomllib

    path = _providers_path()
    if not path.exists():
        return {"ok": False, "error": "providers.toml not found"}
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    ids = [p.get("id") for p in data.get("providers", [])]
    if body.provider_id not in ids:
        return {"ok": False, "error": f"unknown provider: {body.provider_id}"}
    text = path.read_text(encoding="utf-8")
    if re.search(r'^active\s*=', text, re.MULTILINE):
        text = re.sub(
            r'^active\s*=\s*.*$',
            f'active = "{body.provider_id}"',
            text,
            count=1,
            flags=re.MULTILINE,
        )
    else:
        text = f'active = "{body.provider_id}"\n\n' + text
    path.write_text(text, encoding="utf-8")
    return {"ok": True, "active": body.provider_id}


@app.get("/v1/providers")
async def list_providers():
    from nexus_engine.llm.router import _providers_path

    import tomllib

    path = _providers_path()
    if not path.exists():
        return {"active": None, "providers": []}
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    return {
        "active": data.get("active"),
        "providers": [
            {
                "id": p.get("id"),
                "name": p.get("name"),
                "protocol": p.get("protocol"),
                "base_url": p.get("base_url"),
                "model": p.get("model"),
                "proxy_hint": p.get("proxy_hint", False),
            }
            for p in data.get("providers", [])
        ],
    }


@app.get("/v1/mcp/status")
async def mcp_status(workspace_root: str | None = None):
    from nexus_engine.mcp.registry import discover_mcp_config_paths, get_mcp_registry

    reg = get_mcp_registry()
    await reg.ensure_loaded(workspace_root)
    return {
        "config_paths": [str(p) for p in discover_mcp_config_paths(workspace_root)],
        "servers": reg.status(),
        "tools": reg.list_tools_flat(),
    }


@app.post("/v1/mcp/reload")
async def mcp_reload(workspace_root: str | None = None):
    from nexus_engine.mcp.registry import get_mcp_registry

    reg = get_mcp_registry()
    await reg.reload(workspace_root)
    return {"ok": True, "servers": reg.status()}


@app.post("/v1/mcp/call")
async def mcp_call(body: McpCallRequest):
    from nexus_engine.mcp.registry import get_mcp_registry

    reg = get_mcp_registry()
    await reg.ensure_loaded(body.workspace_root)
    try:
        result = await reg.call_tool(body.server, body.tool, body.arguments)
        return {"ok": True, "output": result}
    except Exception as e:
        return {"ok": False, "error": str(e)}


@app.post("/v1/sessions/{session_id}/hydrate")
async def hydrate_chat_session(session_id: str, body: HydrateRequest):
    hydrate_session(
        session_id,
        [{"role": m.role, "content": m.content} for m in body.messages],
    )
    return {"session_id": session_id, "messages": len(body.messages)}


@app.post("/v1/sessions/{session_id}/compact")
async def compact_chat_session(session_id: str, body: CompactRequest | None = None):
    req = body or CompactRequest()
    return await compact_session(
        session_id, max_chars=req.max_chars, semantic=req.semantic
    )


@app.get("/metrics")
async def metrics():
    from nexus_engine.observability import prometheus_text

    return PlainTextResponse(prometheus_text(), media_type="text/plain; version=0.0.4")


@app.post("/v1/sessions/{session_id}/chat")
async def chat_stream(session_id: str, body: ChatRequest):
    from nexus_engine.observability import inc

    inc("nexus_chat_requests_total")

    async def gen():
        async for line in stream_chat_turn(
            session_id,
            body.message,
            model_id=body.model_id,
            project_md=body.project_md,
            workspace_root=body.workspace_root,
            mode=body.mode,
            skills_context=body.skills_context,
        ):
            yield line

    return StreamingResponse(gen(), media_type="text/event-stream")


@app.post("/v1/sessions/{session_id}/tool-results")
async def submit_tool_results(session_id: str, body: ToolResultsRequest):
    async def gen():
        async for line in continue_after_tools(
            session_id, [r.model_dump() for r in body.results]
        ):
            yield line

    return StreamingResponse(gen(), media_type="text/event-stream")


@app.post("/v1/sessions/{session_id}/orchestrate")
async def orchestrate(session_id: str, body: ChatRequest):
    orch = MultiAgentOrchestrator()
    ctx = AgentContext(
        session_id=session_id,
        workspace_root=body.workspace_root,
        model_id=body.model_id,
    )
    results = await orch.run_pipeline(ctx, body.message)
    return {
        "session_id": session_id,
        "phases": [
            {"agent": r.metadata.get("agent"), "content": r.content} for r in results
        ],
    }


class VectorIndexRequest(BaseModel):
    workspace_root: str = "."
    max_files: int = 500


class JsonValidateRequest(BaseModel):
    data: dict | list | str
    schema: dict


@app.post("/v1/vector/index")
async def vector_index(body: VectorIndexRequest):
    from nexus_engine.vector.chroma_store import ChromaStore

    store = ChromaStore()
    stats = store.index_workspace(body.workspace_root, max_files=body.max_files)
    return {"ok": True, **stats}


@app.get("/v1/vector/search")
async def vector_search(q: str, workspace_root: str = ".", k: int = 12):
    from nexus_engine.vector.chroma_store import ChromaStore

    store = ChromaStore()
    hits = store.search(workspace_root, q, k=k)
    return {"query": q, "results": hits}


@app.post("/v1/validate/json-schema")
async def validate_json_schema(body: JsonValidateRequest):
    try:
        import jsonschema
    except ImportError:
        return {"ok": False, "error": "jsonschema not installed"}
    try:
        jsonschema.validate(instance=body.data, schema=body.schema)
        return {"ok": True}
    except jsonschema.ValidationError as e:
        return {"ok": False, "error": str(e.message)}


@app.get("/v1/sync/status")
async def sync_status(workspace_root: str = "."):
    from pathlib import Path as P

    sync_dir = P(workspace_root) / ".nexus" / "sync"
    files = list(sync_dir.glob("*.json")) if sync_dir.is_dir() else []
    return {"workspace": workspace_root, "snapshots": len(files), "path": str(sync_dir)}


@app.websocket("/v1/collab/ws")
async def collab_ws(websocket: WebSocket, workspace: str = Query(default=".")):
    from nexus_engine.collab_hub import hub

    await hub.connect(websocket, workspace)


@app.get("/v1/collab/status")
async def collab_status(workspace: str = "."):
    from nexus_engine.collab_hub import hub

    return {"workspace": workspace, "peers": hub.peer_count(workspace)}


@app.post("/v1/sync/export")
async def sync_export(workspace_root: str = ".", session_id: str | None = None):
    import json
    from datetime import datetime, timezone
    from pathlib import Path as P

    sync_dir = P(workspace_root) / ".nexus" / "sync"
    sync_dir.mkdir(parents=True, exist_ok=True)
    name = f"{session_id or 'workspace'}_{datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S')}.json"
    path = sync_dir / name
    payload = {
        "exported_at": datetime.now(timezone.utc).isoformat(),
        "workspace_root": workspace_root,
        "session_id": session_id,
    }
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    return {"ok": True, "path": str(path)}


@app.get("/v1/tasks/{session_id}/stream")
async def stream_task(session_id: str):
    async def gen():
        yield f'event: token\ndata: {{"session_id": "{session_id}", "delta": "[use POST /v1/sessions/{{id}}/chat]"}}\n\n'
        yield 'event: done\ndata: {"status": "completed"}\n\n'

    return StreamingResponse(gen(), media_type="text/event-stream")


def main() -> None:
    import uvicorn

    uvicorn.run("nexus_engine.api.server:app", host="127.0.0.1", port=8765, reload=False)
