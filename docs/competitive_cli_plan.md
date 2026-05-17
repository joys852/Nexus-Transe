# Nexus-Transe CLI — 终端能力路线图（内部参考）

来源：竞品能力对照摘要。本文把差距映射为 **阶段目标 + 仓库路径**，便于迭代验收。

## 阶段 A — 已达成的底座

| 维度 | 参考能力 | Nexus-Transe |
|------|----------|--------------|
| 交互 REPL + 流式 | 终端对话 | `nexus-cli` + SSE |
| 一次性命令 | 单条指令执行 | `nexus run`；`nexus chat -p` |
| 管道 stdin | 标准输入 | `prompt == -` |
| Plan 模式 | 规划模式 | `/plan` |
| MCP | 外部工具协议 | `mcp.toml` + `/mcp` |
| 技能 | 可扩展技能包 | `/skills` + sync |
| 会话列表 / TUI | 会话管理 | `/sessions` |
| 主题 | 终端主题 | `/theme` |

## 阶段 B — 安全与交互

| 能力 | 说明 | 实现 |
|------|------|------|
| **审批模式** | Suggest / AutoEdit / FullAuto | `approval.rs` + `/approvals` |
| **`@file` 引用** | 终端内夹带文件片段 | `input::expand_message` |
| **一次性命令** | 非交互执行 | `nexus chat -p` / stdin `-` |
| **会话分叉** | 分支探索 | `/fork` |
| **`/context`** | 上下文用量提示 | REPL 命令 |
| **`/compact`** | 语义压缩 | `/compact` / `/compact fast` |

## 阶段 C — 引擎与中台

| 能力 | 实现 |
|------|------|
| **上下文压缩** | `POST /v1/sessions/{id}/compact` |
| **`@` 模糊解析** | `at_resolve.rs` |
| **PreToolUse 钩子** | `.nexus/hooks.toml` + `/hooks init` |
| **JSON Schema 输出** | `--json-schema` / `--json-out` |
| **Git 工具** | `git_*` builtin tools |
| **插件** | `plugins/` + WASM/脚本 |

## 阶段 D — 协作与运维

| 能力 | 实现 |
|------|------|
| **协作 WebSocket** | `/v1/collab/ws` |
| **可观测** | `/metrics`、审计日志 |
| **企业模板** | `config/enterprise.example.toml` |
