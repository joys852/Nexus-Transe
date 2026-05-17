# Nexus-Transe

<p align="center">
  <img src="assets/logo.png" alt="Nexus-Transe — Cybertron Nexus Command Interface" width="640" />
</p>

<p align="center">
  <strong>终端指挥 CLI</strong> — 本地优先的开发工作流助手<br/>
  <a href="https://github.com/joys852/Nexus-Transe">github.com/joys852/Nexus-Transe</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License" />
  <img src="https://img.shields.io/badge/version-1.0.0-cyan.svg" alt="Version" />
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey.svg" alt="Platform" />
</p>

**Nexus-Transe**（赛博坦 Nexus 指挥界面）是面向开发者的 **纯终端智能体 CLI**：Rust 核心 + Python 编排引擎，支持工具调用、MCP、多智能体流水线与会话持久化。

## 架构

```
nexus / nx (Rust CLI) ── nexus-core ── HTTP ── nexus-engine (Python)
```

| 组件 | 说明 |
|------|------|
| `packages/nexus-cli` | 交互 REPL、`nexus` / `nx` 二进制 |
| `packages/nexus-core` | SQLite、工具沙箱、提供商配置 |
| `packages/nexus-engine` | 模型推理、SSE、MCP、向量检索 |

## 快速开始

### 生产安装包（推荐）

在**构建机**（需 Rust + [uv](https://docs.astral.sh/uv/)）上：

```powershell
git clone https://github.com/joys852/Nexus-Transe.git
cd Nexus-Transe
.\scripts\package-release.ps1
```

产物：`dist/Nexus-Transe-1.0.0-windows-x64.zip` — 解压后运行 `bin\nexus.cmd`，任意目录可用。

### 开发模式

```powershell
# 终端 A — 引擎
cd packages\nexus-engine
uv sync --extra dev
uv run nexus-engine

# 终端 B — CLI
cargo run -p nexus-cli
```

或：`.\scripts\install.ps1 -AddToPath` 安装到 `%LOCALAPPDATA%\NexusIDE\`。

## 常用命令

```powershell
nexus              # 进入 REPL（默认）
nx                 # 短命令
nexus run "任务" -y
nexus engine status
nexus provider list
```

## 文档

| 文档 | 内容 |
|------|------|
| [INSTALL.md](docs/INSTALL.md) | 安装与环境 |
| [DISTRIBUTION.md](docs/DISTRIBUTION.md) | 跨机器分发 |
| [PRODUCTION.md](docs/PRODUCTION.md) | 生产发布清单 |
| [RELEASE.md](docs/RELEASE.md) | 版本发布流程 |
| [CLI.md](docs/CLI.md) | 命令与斜杠参考 |

## 开源许可

本项目采用 **[Apache License 2.0](LICENSE)**，与 [GitHub 仓库](https://github.com/joys852/Nexus-Transe) 一致。

- 源码：Apache-2.0
- Logo：`assets/logo.png` 为项目品牌标识，详见 [NOTICE](NOTICE)

## 贡献

欢迎 Issue / PR：[Nexus-Transe](https://github.com/joys852/Nexus-Transe/issues)

## 作者

[joys852](https://github.com/joys852)
