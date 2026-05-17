# NexusIDE CLI — 对标 Claude Code / Codex CLI 路线图

来源：`s1.txt`（2026 对比摘要）。本文把差距映射为 **阶段目标 + 仓库路径**，便于迭代验收。

## 阶段 A — 已达成的底座（当前仓库）

| 维度 | Claude Code / Codex | NexusIDE |
|------|---------------------|----------|
| 交互 REPL + 流式 | ✅ | `nexus-cli` + SSE |
| 一次性命令 | `claude -p` / `codex exec` | `nexus run`；**`nexus chat -p`**（本期） |
| 管道 stdin | `echo \| …` | **`prompt == -`**（本期） |
| Plan 模式 | `/plan` | `/plan` |
| MCP | ✅ | `mcp.toml` + `/mcp` |
| 技能 | ✅ | `/skills` + sync |
| 会话列表 / TUI | ✅ | `/sessions` |
| 主题 | — | `/theme` |

## 阶段 B — 已完成（安全与交互对齐 Codex）

| 能力 | 说明 | 实现 |
|------|------|------|
| **审批模式** | Suggest（默认）/ AutoEdit / FullAuto | `approval.rs` + `/approvals` + `ChatRunner::execute_tools` |
| **`@file` 引用** | 终端内显式夹带文件片段 | `input::expand_message`（REPL + `run` + `chat -p`） |
| **一次性命令** | `claude -p` / `codex exec` | `nexus chat -p "…"` / `nexus chat -p -`（stdin） |
| **会话分叉** | Fork 对话继续探索 | `/fork` → 复制 user/assistant 消息 + hydrate |
| **`/context`** | 粗粒度上下文用量提示 | REPL 命令（instructions/skills/messages 估算） |
| **`/compact`** | 语义压缩 | `/compact` LLM 摘要 · `/compact fast` 规则模式 |

## 阶段 C — 已完成（引擎与中台）

| 能力 | 实现 |
|------|------|
| **上下文压缩** | `POST /v1/sessions/{id}/compact` + `ContextCompressor` 摘要 + `/compact` 同步 SQLite |
| **`@` 模糊解析** | `at_resolve.rs` — 模糊匹配 + 多候选提示 |
| **PreToolUse 钩子** | `.nexus/hooks.toml` + `/hooks init` |
| **Git 深化** | `git_branch` / `git_commit` / `git_log`（Rust + engine 工具定义） |
| **结构化输出** | `nexus run --json-schema schema.json [--json-out out.json]` |
| **子代理 Team** | `/team <goal>` → LangGraph orchestrate 流水线 |

## 阶段 D — 已完成（基础闭环）

| 能力 | 实现 |
|------|------|
| **插件安装** | `PluginManager::install_scaffold` + `/plugins install <id>` + TUI `i` 键 |
| **协作占位** | 已有 `/collab` + workspace lock；云端执行留作后续产品项 |

---

验收：阶段 B 完成后，应用户可用 **`/approvals auto-edit`** 减少写文件确认；用 **`nexus chat -p "…"`** 做 CI 一键跑；用 **`@src/foo.rs`** 在一句提问里夹带代码片段。
