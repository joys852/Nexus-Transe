# NexusIDE API 参考

## Engine HTTP（Sidecar，默认 `127.0.0.1:8765`）

### `GET /health`

```json
{ "ok": true, "version": "0.1.0" }
```

### `POST /v1/tasks/run`

请求：

```json
{
  "session_id": "uuid",
  "prompt": "implement login",
  "model_id": "claude-sonnet-4-20250514",
  "agent_profile": "default"
}
```

响应：

```json
{ "task_id": "uuid", "status": "planning" }
```

### `POST /v1/tasks/{session_id}/pause` | `/resume`

暂停/恢复 LangGraph 检查点。

### `GET /v1/tasks/{session_id}/stream`

SSE 流：`data: {"session_id","delta"}`

---

## Rust Core Traits

| Trait | 职责 |
|-------|------|
| `SessionRepository` | 会话与消息 CRUD |
| `CheckpointStore` | ReAct 图状态持久化 |
| `AuditLog` | 工具调用审计 |
| `ToolHandler` + `ToolRegistry` | 工具注册与策略 |
| `SyncBus` | CLI 内会话与事件 |
| `EngineClient` | 调用 Python sidecar |

详见 `packages/nexus-core/src/`.

---

## TypeScript

共享类型：`packages/shared/typescript/src/protocol.ts`
