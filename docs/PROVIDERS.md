# 模型提供商与三方中转 / CC Switch

Nexus-Transe 支持两种 API 协议（与 CC Switch 下拉选项一致）：

| 协议 | `protocol` 值 | 适用场景 |
|------|----------------|----------|
| **Messages API（原生）** | `anthropic_messages` | 支持 `/v1/messages` 的官方或中转 |
| **Chat Completions（兼容）** | `openai_chat_completions` | 多数三方中转站、本地推理端点 |

## 配置文件

路径（与数据目录相同）：

- Windows: `%APPDATA%\nexus-ide\providers.toml`
- Linux/macOS: `~/.local/share/nexus-ide/providers.toml`

示例见仓库根目录 [packages/nexus-core/providers.example.toml](../packages/nexus-core/providers.example.toml)。

```toml
active = "openai-relay"

[[providers]]
id = "openai-relay"
name = "我的中转站"
protocol = "openai_chat_completions"
base_url = "https://api.example.com/v1"
model = "gpt-4o"
api_key_env = "OPENAI_API_KEY"
proxy_hint = true
```

## CLI 命令

```powershell
# 列出提供商
nexus provider list

# 切换当前提供商（写入 providers.toml active）
nexus provider use openai-relay

# 从 CC Switch / 外部配置导入
nexus provider import-cc-switch
nexus provider import-cc-switch --from %USERPROFILE%\.claude\settings.json
nexus provider import-cc-switch --from export.json

# 加密保存密钥（推荐生产环境）
$env:NEXUS_MASTER_KEY = "<32字节Base64>"
nexus secrets set openai-relay sk-xxx
# 然后在 providers.toml 中设置 api_key_vault = "openai-relay"
```

## 与 CC Switch 的关系

[CC Switch](https://github.com/farion1231/cc-switch) 用于在多款终端工具间切换模型配置。Nexus-Transe **不替代** CC Switch，但可：

1. **导入** CC Switch 写入的外部 `settings.json`（`ANTHROPIC_BASE_URL` / `ANTHROPIC_AUTH_TOKEN`）
2. **导入** 导出的 JSON 配置（`providers` 数组）
3. 使用相同的 **双协议** 模型，便于对照 CC Switch 里的选项

启动引擎前设置与当前提供商一致的环境变量，或由 `providers.toml` 的 `api_key_env` 自动读取。

## 环境变量（兼容）

| 变量 | 说明 |
|------|------|
| `ANTHROPIC_API_KEY` / `ANTHROPIC_AUTH_TOKEN` | 原生协议密钥 |
| `OPENAI_API_KEY` | Chat Completions 兼容中转 |
| `NEXUS_API_BASE` | 覆盖默认 base URL（无 providers.toml 时） |
| `NEXUS_DATA_DIR` | 数据目录（providers.toml 位置） |

## 代理说明

`proxy_hint = true` 仅用于配置提示「需开启代理」；系统代理请配置 `HTTP_PROXY` / `HTTPS_PROXY`，或在操作系统网络设置中配置。