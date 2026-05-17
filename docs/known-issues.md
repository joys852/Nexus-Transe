# 已知问题与能力边界

## 已交付（CLI + Engine）

| 领域 | 状态 |
|------|------|
| MCP stdio | Content-Length 帧 + NDJSON 回退；`env` 字段 |
| 语义检索 | Chroma `nexus vector-index` + `semantic_search` 工具 |
| 多模态 | `[#image:]` → base64 → OpenAI / Anthropic vision blocks |
| JSON Schema | 引擎 `jsonschema` + CLI `--json-schema` |
| Shell 沙箱 | 规则拦截 + **Docker 可选**（`NEXUS_SANDBOX=docker`） |
| 协作 | `/sync` 快照 + **WebSocket** `/v1/collab/ws` |
| 插件 | 脚本 + wasmtime；`nexus plugins` |
| 可观测 | `GET /metrics`、`.nexus/audit.jsonl`、可选 `NEXUS_SENTRY_DSN` |
| 企业模板 | `config/enterprise.example.toml`、`.devcontainer/` |
| VS Code | `extensions/nexus-vscode` — 启动引擎 / 打开 CLI 终端 |

## CLI 使用前提

1. 引擎运行：`uv run nexus-engine`（或 `nexus engine start`）
2. 在项目根目录执行 `nexus` / `nx`，工具在本地工作区内执行

## 尚未完成 / 需另立项

| 项 | 说明 |
|----|------|
| WASM 进程内 VM | 当前为 **wasmtime 子进程** |
| 强容器策略 | Docker 为简易 alpine，非 Dev Container 全功能 |
| 企业 SSO | 模板已有，OIDC 未接入运行时 |
| MCP 全部第三方服务 | 依赖各 server 安装与网络 |
| 图形化 IDE | 已移除 Web/Desktop 壳，仅保留终端 CLI |

## 测试

```powershell
cargo test -p nexus-core
cargo test -p nexus-cli
cd packages/nexus-engine && uv run pytest -q
```
