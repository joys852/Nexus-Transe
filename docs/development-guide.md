# 开发指南

## 环境要求

| 组件 | 版本 |
|------|------|
| Rust | 1.80+ |
| Node | 22+（可选，仅 `packages/shared/typescript`） |
| pnpm | 9+（同上） |
| Python | 3.11+ |
| uv | 最新 |

## 本地启动

详见 [INSTALL.md](INSTALL.md)。简要步骤：

```powershell
# 引擎
cd packages\nexus-engine && uv sync --extra dev && uv run nexus-engine

# CLI 交互
cargo run -p nexus-cli -- chat
```

设置 `OPENAI_API_KEY` 后 AI 功能可用。

## 数据目录

默认：`%APPDATA%/nexus-ide`（Windows）或 `~/.local/share/nexus-ide`（Linux）

- `nexus.db` — SQLite
- `chroma/` — 向量索引（Beta）
- `sync.sock` — IPC（Beta）

## 测试

```bash
cargo test -p nexus-core
cd packages/nexus-engine && uv run pytest
```

## 迁移

SQL 文件：`migrations/`. 由 `SqliteStore::connect` 自动执行。

## 添加工具

1. 在 `nexus-core` 实现 `ToolHandler`
2. 注册到 `ToolRegistry`
3. 在 `permission_policies` 表配置 `allow|deny|ask`
4. Engine 通过 HTTP 回调 Rust 执行（Beta）

## 添加 LLM Provider

在 `nexus_engine/llm/` 实现 `LLMProvider`，并在 `create_provider()` 注册。

## 代码规范

- Rust：`cargo fmt`, `clippy`
- Python：`ruff check`, `ruff format`
- TypeScript：`pnpm exec tsc --noEmit -p packages/shared/typescript`
