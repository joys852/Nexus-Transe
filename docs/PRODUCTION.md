# Nexus-Transe 生产发布指南

> 仓库：[https://github.com/joys852/Nexus-Transe](https://github.com/joys852/Nexus-Transe)  
> 许可：[Apache-2.0](../LICENSE)

## 版本矩阵

| 组件 | 版本 | 产物 |
|------|------|------|
| nexus-cli | 1.0.0 | `nexus.exe` / `nexus` / `nx` |
| nexus-engine | 1.0.0 | Python sidecar + `.venv` |
| 发行包 | 1.0.0 | `Nexus-Transe-{version}-{os}-{arch}.zip` |

版本号以根目录 [`VERSION`](../VERSION) 为准。

## 构建生产包

### Windows

```powershell
.\scripts\package-release.ps1
# 可选：写入用户 PATH
.\scripts\package-release.ps1 -Install
```

### Linux / macOS

```bash
chmod +x scripts/package-release.sh
./scripts/package-release.sh
```

### 包内容

```
Nexus-Transe-1.0.0-windows-x64/
  LICENSE
  NOTICE
  README.md
  VERSION
  assets/logo.png
  bin/nexus.exe, nx.exe, nexus.cmd
  engine/          # Python 包 + .venv（含 nexus-engine）
  docs/            # INSTALL, CLI, DISTRIBUTION 等
```

目标机要求：

- **Windows 10+**（x64）
- 无需 Rust；若包内已含 `.venv` 则**无需 uv**
- LLM：`OPENAI_API_KEY` 或 `providers.toml` + 密钥库

## 安装与配置

详见 [INSTALL.md](./INSTALL.md)、[DISTRIBUTION.md](./DISTRIBUTION.md)。

```toml
# %LOCALAPPDATA%\nexus-ide\config.toml
engine_url = "http://127.0.0.1:8765"
default_model = "gpt-4o-mini"
```

```powershell
$env:OPENAI_API_KEY = "sk-..."
$env:NEXUS_MASTER_KEY = "<32-byte-base64>"   # 生产必设
```

## 安全清单（上线前）

- [ ] 设置 `NEXUS_MASTER_KEY`，禁用默认开发密钥
- [ ] 工具策略：写文件 / Shell 默认需确认（`/approvals`）
- [ ] 审计：`.nexus/audit.jsonl` 与 SQLite `audit_log`
- [ ] 不向仓库提交 API Key、`.env`
- [ ] Windows：对 `nexus.exe` 做 Authenticode 签名（可选）
- [ ] CI：`cargo audit`、`pip-audit` / `uv pip audit`

## 监控

- 引擎健康：`GET http://127.0.0.1:8765/health`
- Prometheus：`GET /metrics`
- 可选：`NEXUS_SENTRY_DSN`

## CI / GitHub Release

推送 tag `v1.0.0` 触发 [`.github/workflows/release.yml`](../.github/workflows/release.yml)：

- 三平台 CLI 二进制
- 引擎 wheel
- 可选：附加 `dist/*.zip`（本地 `package-release` 产出）

## 支持

- 文档：`docs/`
- Issues：[Nexus-Transe Issues](https://github.com/joys852/Nexus-Transe/issues)
