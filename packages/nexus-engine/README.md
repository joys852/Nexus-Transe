# nexus-engine

NexusIDE 智能体引擎：LangGraph 编排、多模型路由（Anthropic 原生 / OpenAI 兼容中转）、SSE 聊天 API。

## 运行

```bash
uv sync --extra dev
uv run nexus-engine
```

默认监听 `http://127.0.0.1:8765`。

## 环境变量

| 变量 | 说明 |
|------|------|
| `OPENAI_API_KEY` / `ANTHROPIC_API_KEY` | API 密钥 |
| `NEXUS_DATA_DIR` | 数据目录（含 `providers.toml`） |
| `NEXUS_API_BASE` | 无 providers 时的默认 API 地址 |

推荐使用 `%APPDATA%\nexus-ide\providers.toml` 管理提供商，见仓库 `docs/PROVIDERS.md`。

## Python 版本

建议 **3.11–3.13**。若 3.14 下依赖安装失败，请使用：

```bash
uv python install 3.12
uv sync --python 3.12 --extra dev
```
