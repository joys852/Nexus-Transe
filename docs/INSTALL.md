# Nexus-Transe 安装与使用指南

> 项目主页：[https://github.com/joys852/Nexus-Transe](https://github.com/joys852/Nexus-Transe) · 许可：Apache-2.0

## 环境要求

| 工具 | 版本 |
|------|------|
| Rust | 1.80+ (`rustup`) |
| Python | 3.11+ |
| uv | 最新 |
| Node.js | 22+（可选，仅开发 `packages/shared/typescript` 时） |
| pnpm | 9+（同上） |
| Git | 任意 |

## 国内镜像（加速 uv / pip）

仓库已配置清华 PyPI + npmmirror Python 安装源。若仍慢，可先执行：

```powershell
. .\scripts\use-china-mirror.ps1
```

或手动设置：

```powershell
$env:UV_INDEX_URL = "https://pypi.tuna.tsinghua.edu.cn/simple"
$env:UV_PYTHON_INSTALL_MIRROR = "https://npmmirror.com/mirrors/python-build-standalone/"
```

**推荐从仓库根目录安装引擎**（会读取 `uv.toml`）：

```powershell
cd e:\IDE
uv sync --directory packages/nexus-engine --extra dev
```

其他可选源：阿里云 `https://mirrors.aliyun.com/pypi/simple/`、腾讯 `https://mirrors.cloud.tencent.com/pypi/simple/`

## 安装

```powershell
cd e:\IDE

# Rust CLI + 核心库
cargo build -p nexus-cli --release

# Python 引擎（使用镜像）
uv sync --directory packages/nexus-engine --extra dev
```

（可选）校验共享协议类型：

```powershell
pnpm install
pnpm exec tsc --noEmit -p packages/shared/typescript
```

## 配置

在 `%APPDATA%\nexus-ide\` 或 `~/.local/share/nexus-ide/` 自动创建数据目录。

可选配置文件（环境变量 `NEXUS_CONFIG` 指向 TOML 文件）：

```toml
default_model = "gpt-4o-mini"
engine_url = "http://127.0.0.1:8765"
```

### LLM API（引擎）

```powershell
$env:OPENAI_API_KEY = "sk-..."
$env:NEXUS_MODEL = "gpt-4o-mini"
# 可选中转
$env:NEXUS_API_BASE = "https://api.openai.com/v1"
```

Ollama 本地：

```powershell
$env:NEXUS_API_BASE = "http://127.0.0.1:11434/v1"
$env:NEXUS_MODEL = "llama3.2"
```

## 启动

**终端 1 — 引擎**

```powershell
cd packages\nexus-engine
uv run nexus-engine
```

**终端 2 — CLI 交互**

```powershell
cargo run -p nexus-cli -- chat
```

## CLI 命令速查

| 命令 | 说明 |
|------|------|
| `nexus chat` | 交互式 REPL（流式输出、Ctrl+C 取消） |
| `nexus run "task"` | 单次任务 |
| `nexus run "task" -y` | 自动批准写/Shell 工具 |
| `nexus index` | 扫描项目文件（尊重 .gitignore） |
| `nexus init` | 创建 `PROJECT.md` |
| `nexus engine status` | 检查引擎 |
| `nexus session list` | 列出会话 |

### REPL 内置命令

`/help` `/exit` `/cancel` `/index` `/git-status`

## 项目配置 PROJECT.md

在项目根目录创建 `PROJECT.md`（或 `CLAUDE.md`），内容会注入 AI 系统提示。

```powershell
nexus init
```

## 安全与审批

- **读取 / Git 状态**：默认允许
- **写入文件 / Shell / 编辑**：需确认（`y`）或使用 `nexus run -y`

## 跨平台构建

构建前请关闭正在运行的 `nexus`/`nexus.exe`，否则 Windows 可能无法覆盖 `target/` 下的二进制。

```powershell
cargo build -p nexus-cli --release
```

产物：`target/release/nexus.exe`（或对应平台的 `nexus` 二进制）。

### 生产发行包（上线标准）

```powershell
.\scripts\package-release.ps1
# 产物: dist\Nexus-Transe-1.0.0-windows-x64.zip
```

解压后运行 `bin\nexus.cmd`，详见 [PRODUCTION.md](./PRODUCTION.md)、[RELEASE.md](./RELEASE.md)。

### 安装到本机任意目录可用

```powershell
.\scripts\install.ps1 -AddToPath
# 新开终端，在任意项目目录：
nexus.cmd
```

详见 **[DISTRIBUTION.md](./DISTRIBUTION.md)**（跨机器拷贝、远程引擎、环境变量）。

### 从含桌面端的旧克隆升级

本产品为 **CLI + 引擎** 架构；浏览器 Web UI 与 `nexus-desktop` 壳已从仓库移除。若本地仍留有 `packages/nexus-desktop` 空目录，可手动删除。
