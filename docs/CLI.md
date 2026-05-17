# NexusIDE CLI 完整参考

## 启动

```powershell
nexus                       # 默认进入 REPL（等同 nexus chat）
nx                          # 同上，更短
.\scripts\nexus.ps1         # 未安装到 PATH 时从仓库启动

nexus engine start          # 后台启动 Python 引擎
nexus engine status
nexus chat                  # 显式进入 REPL
nexus chat                  # 默认会自动拉起引擎（离线时）
nexus chat --no-start-engine  # 不自动启动，需先 nexus engine start
nexus collab url            # 协作 WebSocket 地址（多 CLI 实例）
```

引擎须在线：`uv run nexus-engine`（在 `packages/nexus-engine`）或 `nexus engine start`。

## 一次性 / 脚本

```powershell
nexus run "任务" -y
nexus chat -p "一句话"
nexus run "返回 JSON" --json-schema schema.json --json-out out.json
nexus search "TODO" --limit 50    # 分组 + 关键词高亮
nexus vector-index                # Chroma 语义索引
```

## REPL 命令（节选）

| 命令 | 说明 |
|------|------|
| `/compact` / `/compact fast` | LLM 语义压缩 / 规则压缩 |
| `/team <goal>` | 多智能体流水线（带 agent 输出预算） |
| `/vector-index` | 语义索引当前仓库 |
| `/sandbox [local\|docker]` | Shell 隔离（Docker 需本机守护进程） |
| `/sync` | 导出协作快照到 `.nexus/sync/` |
| `/export [path]` | 导出会话 JSON |
| `/hooks init` | PreToolUse 钩子（含 `command` 脚本） |
| `@path` + Tab | 模糊文件引用与补全 |

## 工具（模型可调用）

`read_file`, `write_file`, `edit_file`, `run_shell`, `glob_files`,  
`semantic_search`, `git_status`, `git_diff`, `git_branch`, `git_commit`, `git_log`, `git_push`,  
以及 MCP 注册工具。工具在 **CLI 进程内** 执行，不依赖浏览器或 WebView。

## 安全

- Shell 沙箱：`local`（默认）或 `docker`（`NEXUS_SANDBOX=docker` / `sandbox_mode` / `/sandbox docker`）
- 规则级拦截：危险命令片段、长度限制
- `/approvals`：suggest / auto-edit / full-auto
- `.nexus/hooks.toml`：deny / prompt / command 钩子

## 多模态

`[#image:path.png]` — 小图（<400KB）转 base64，引擎侧转视觉模型输入格式（需模型支持）。

## 协作

- `nexus collab url` — `ws://127.0.0.1:8765/v1/collab/ws?workspace=...`
- `/collab` — 本机 workspace.lock 占用

## 插件

- 脚本：`run.ps1` / `run.sh`
- WASM：`plugin.toml` 中 `entry = "main.wasm"`，需安装 [wasmtime](https://wasmtime.dev/)

## 仍依赖外部能力

- 企业云端账号、强隔离多租户 — 见 [`known-issues.md`](known-issues.md)
