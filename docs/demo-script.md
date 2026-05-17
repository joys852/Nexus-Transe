# NexusIDE MVP 演示视频脚本（约 8 分钟）

## 1. 开场（30s）

- 展示 NexusIDE 架构图：CLI + 共享 Rust 核心 + Python 引擎
- 说明：类 Claude Code，本地优先、可审批工具调用

## 2. 环境启动（45s）

1. 启动引擎：`uv run nexus-engine`，浏览器访问 `http://127.0.0.1:8765/health`
2. `nexus engine status` 显示 online

## 3. 项目初始化（60s）

1. 进入示例仓库
2. `nexus init` 生成 `PROJECT.md`，编辑项目说明
3. `nexus index` 显示索引文件数量

## 4. CLI 交互式会话（3min）

1. `nexus chat`
2. 提问：「列出 src 目录结构并说明职责」
3. 展示流式彩色输出
4. 触发 `read_file` 工具（自动执行）
5. 触发 `write_file` 或 `run_shell` — 展示审批提示，输入 `y` 批准
6. `Ctrl+C` 取消当前生成，输入 `/exit` 退出
7. 可选：演示 `/plan`、`/sessions`、`/theme` 等斜杠命令

## 5. 单次任务（45s）

```powershell
nexus run "运行 git status 并总结变更" -y
```

## 6. 安全与多模型（45s）

- 展示 `OPENAI_API_KEY` / `NEXUS_API_BASE` 切换 DeepSeek 或 Ollama
- `nexus provider list` / `nexus provider use …`
- 强调写操作需审批

## 7. 收尾（30s）

- 路线图：Tree-sitter、Chroma、MCP、会话同步
- 仓库链接与 `docs/INSTALL.md`
