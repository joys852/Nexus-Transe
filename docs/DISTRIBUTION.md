# CLI 分发与跨机器安装

NexusIDE 由两部分组成，需一并部署才能在任意目录使用：

| 组件 | 作用 | 典型路径 |
|------|------|----------|
| **nexus** / **nx** | Rust CLI（REPL、工具、会话） | `bin/nexus.exe` |
| **nexus-engine** | Python 侧车（LLM、MCP、向量） | `engine/` |

CLI 的工作目录可以是任意项目文件夹；配置与数据库在用户数据目录，与安装目录无关。

## 推荐：一键安装到固定前缀

在**有 Rust + uv** 的开发机上构建，安装到本机固定目录后，其他终端、任意 `cd` 均可运行。

### Windows

```powershell
cd e:\IDE
.\scripts\install.ps1 -AddToPath
# 新开终端后，任意目录：
nexus.cmd
# 或
nx.cmd
```

默认安装位置：`%LOCALAPPDATA%\NexusIDE\`

```
%LOCALAPPDATA%\NexusIDE\
  bin\
    nexus.exe
    nx.exe
    nexus.cmd      ← 自动设置 NEXUS_ENGINE_DIR
  engine\          ← Python 包 + .venv
  config.example.toml
```

### Linux / macOS

```bash
chmod +x scripts/install.sh
./scripts/install.sh --prefix "$HOME/.local/nexus-ide" --add-to-path
# 任意目录：
~/.local/nexus-ide/bin/nexus-wrap
```

## 引擎如何被找到

按顺序（先命中先用）：

1. 环境变量 **`NEXUS_ENGINE_DIR`**（安装脚本通过 `nexus.cmd` / wrapper 设置）
2. **可执行文件旁**：`<nexus.exe 所在目录>/engine` 或 `nexus-engine`
3. **开发仓库**：向上查找 `packages/nexus-engine`
4. 当前目录向上查找 `packages/nexus-engine`

启动引擎时：

- 若 `engine/.venv` 已存在 → 直接运行其中的 `nexus-engine`（**目标机可不装 uv**）
- 否则需要 **`uv run nexus-engine`**（需安装 uv）

## 拷贝到另一台机器

### 方式 A：便携目录（同系统、同架构）

1. 在构建机执行 `install.ps1` / `install.sh` 生成完整前缀
2. 将整个 `NexusIDE` 文件夹 zip 复制到目标机（例如 `D:\Apps\NexusIDE`）
3. 目标机无需 Rust；若已包含 `.venv` 则无需 uv
4. 将 `bin` 加入 PATH，或始终用绝对路径调用 `nexus.cmd`

注意：**Windows 的 `.venv` 不能拷到 Linux**，需按目标平台各装一次。

### 方式 B：仅分发 CLI + 远程引擎

适合团队统一 LLM 网关：

```toml
# %APPDATA%\nexus-ide\config.toml  或 NEXUS_CONFIG
engine_url = "http://your-team-server:8765"
```

```powershell
$env:NEXUS_CONFIG = "C:\path\to\config.toml"
nexus engine status
nexus
```

目标机只需 `nexus.exe`，不必带 Python 引擎。

### 方式 C：cargo install（仅 CLI）

```powershell
cargo install --path packages/nexus-cli --locked
```

仍需在本机或远程提供引擎，并设置 `NEXUS_ENGINE_DIR` 或 `engine_url`。

## 环境变量速查

| 变量 | 说明 |
|------|------|
| `NEXUS_ENGINE_DIR` | Python 引擎根目录（含 `pyproject.toml`） |
| `NEXUS_ENGINE_URL` | 引擎 HTTP 地址，默认 `http://127.0.0.1:8765` |
| `NEXUS_CONFIG` | 用户 `config.toml` 路径 |
| `NEXUS_DATA_DIR` | SQLite、密钥、插件数据目录 |
| `OPENAI_API_KEY` / `NEXUS_API_BASE` | LLM（由引擎读取） |

用户数据默认位置：

- Windows：`%APPDATA%\nexus-ide\` 或 `%LOCALAPPDATA%\nexus-ide\`
- Linux/macOS：`~/.local/share/nexus-ide/`

## 跨平台发布构建

在对应平台编译 release 二进制：

```powershell
# Windows
cargo build -p nexus-cli --release
# 产物: target\release\nexus.exe
```

```bash
# Linux / macOS
cargo build -p nexus-cli --release
# 产物: target/release/nexus
```

发布 checklist：

- [ ] `install.ps1` / `install.sh` 生成的前缀含 `engine/.venv`
- [ ] 附带 `config.example.toml` 与 API Key 说明
- [ ] 文档说明最低要求：Windows 10+ / glibc Linux / macOS 12+
- [ ] 可选：代码签名 `nexus.exe`（减少 SmartScreen 拦截）

## 常见问题

**Q: 任意目录运行报 engine not found**  
A: 使用 `nexus.cmd`（会设 `NEXUS_ENGINE_DIR`），或手动：

```powershell
$env:NEXUS_ENGINE_DIR = "C:\Users\you\AppData\Local\NexusIDE\engine"
nexus
```

**Q: 不想自动拉引擎**  
A: 先 `nexus engine start`，或远程 `engine_url`；开发时也可 `uv run nexus-engine` 常驻。

**Q: 多台电脑共享配置**  
A: 复制整个 `nexus-ide` 数据目录，或只用 `NEXUS_CONFIG` 指向共享 TOML（密钥注意安全）。
