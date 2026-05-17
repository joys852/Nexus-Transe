//! LLM provider profiles — relay stations, protocol switching, CC Switch import.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::secrets::SecretVault;
use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiProtocol {
    /// Anthropic Messages API (`/v1/messages`) — official & native relays
    AnthropicMessages,
    /// OpenAI Chat Completions (`/v1/chat/completions`) — most 三方中转站
    OpenAiChatCompletions,
}

impl ApiProtocol {
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::AnthropicMessages => "Anthropic Messages (原生)",
            Self::OpenAiChatCompletions => "OpenAI Chat Completions (中转/代理)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub id: String,
    pub name: String,
    pub protocol: ApiProtocol,
    pub base_url: String,
    pub model: String,
    /// Environment variable name holding the API key
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// Inline API key (prefer env vars in production)
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub api_key_vault: Option<String>,
    #[serde(default)]
    pub small_fast_model: Option<String>,
    #[serde(default)]
    pub proxy_hint: bool,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub extra_headers: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub active: Option<String>,
    #[serde(default)]
    pub providers: Vec<ProviderProfile>,
}

impl ProvidersConfig {
    pub fn active_provider(&self) -> Option<&ProviderProfile> {
        let id = self.active.as_ref()?;
        self.providers.iter().find(|p| &p.id == id)
    }

    pub fn get(&self, id: &str) -> Option<&ProviderProfile> {
        self.providers.iter().find(|p| p.id == id)
    }
}

pub struct ProvidersStore {
    path: PathBuf,
}

impl ProvidersStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join("providers.toml"),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<ProvidersConfig> {
        if !self.path.exists() {
            return Ok(default_providers());
        }
        let text = std::fs::read_to_string(&self.path)?;
        let cfg: ProvidersConfig = toml::from_str(&text).map_err(|e| {
            crate::NexusError::Config(format!("providers.toml: {e}"))
        })?;
        Ok(cfg)
    }

    pub fn save(&self, config: &ProvidersConfig) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(config).map_err(|e| {
            crate::NexusError::Config(format!("providers serialize: {e}"))
        })?;
        std::fs::write(&self.path, text)?;
        Ok(())
    }

    pub fn set_active(&self, id: &str) -> Result<ProvidersConfig> {
        let mut cfg = self.load()?;
        if cfg.get(id).is_none() {
            return Err(crate::NexusError::Config(format!("unknown provider: {id}")));
        }
        cfg.active = Some(id.to_string());
        self.save(&cfg)?;
        Ok(cfg)
    }

    /// Resolve API key from inline field, env, vault, or common fallbacks.
    pub fn resolve_api_key(
        &self,
        profile: &ProviderProfile,
        data_dir: &Path,
    ) -> Result<Option<String>> {
        if let Some(key) = &profile.api_key {
            if !key.is_empty() {
                return Ok(Some(key.clone()));
            }
        }
        if let Some(env) = &profile.api_key_env {
            if let Some(v) = resolve_env_or_inline_key(env) {
                return Ok(Some(v));
            }
        }
        if let Some(vault_id) = &profile.api_key_vault {
            let vault = SecretVault::open(data_dir)?;
            let store = vault.load()?;
            return vault.get_api_key(&store, vault_id);
        }
        // Fallback common env names
        for env in ["ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN", "OPENAI_API_KEY", "NEXUS_API_KEY"] {
            if let Ok(v) = std::env::var(env) {
                if !v.is_empty() {
                    return Ok(Some(v));
                }
            }
        }
        Ok(None)
    }
}

/// Import from CC Switch–compatible sources (Claude settings, export JSON).
pub fn import_cc_switch(source: &Path) -> Result<ProvidersConfig> {
    if source.file_name().and_then(|s| s.to_str()) == Some("settings.json") {
        return import_claude_settings(source);
    }
    if source.extension().and_then(|s| s.to_str()) == Some("json") {
        return import_cc_switch_json(source);
    }
    // Try as Claude settings path
    if source.exists() && source.is_file() {
        if let Ok(cfg) = import_claude_settings(source) {
            if !cfg.providers.is_empty() {
                return Ok(cfg);
            }
        }
        return import_cc_switch_json(source);
    }
    Err(crate::NexusError::Config(format!(
        "unsupported import source: {}",
        source.display()
    )))
}

fn import_claude_settings(path: &Path) -> Result<ProvidersConfig> {
    let text = std::fs::read_to_string(path)?;
    let v: serde_json::Value = serde_json::from_str(&text)?;
    let mut providers = Vec::new();

    let base_url = v
        .get("env")
        .and_then(|e| e.get("ANTHROPIC_BASE_URL"))
        .or_else(|| v.get("ANTHROPIC_BASE_URL"))
        .and_then(|x| x.as_str())
        .unwrap_or("https://api.anthropic.com");

    let model = v
        .get("env")
        .and_then(|e| e.get("ANTHROPIC_MODEL"))
        .or_else(|| v.get("ANTHROPIC_MODEL"))
        .and_then(|x| x.as_str())
        .unwrap_or("claude-sonnet-4-20250514");

    let is_relay = !base_url.contains("api.anthropic.com");
    providers.push(ProviderProfile {
        id: "cc-switch-claude".into(),
        name: if is_relay {
            "CC Switch / Claude (中转)".into()
        } else {
            "Anthropic Messages (原生)".into()
        },
        protocol: ApiProtocol::AnthropicMessages,
        base_url: base_url.trim_end_matches('/').to_string(),
        model: model.into(),
        api_key_env: Some("ANTHROPIC_AUTH_TOKEN".into()),
        api_key: None,
        api_key_vault: None,
        small_fast_model: None,
        proxy_hint: is_relay,
        timeout_ms: None,
        extra_headers: Default::default(),
    });

    Ok(ProvidersConfig {
        active: Some("cc-switch-claude".into()),
        providers,
    })
}

fn import_cc_switch_json(path: &Path) -> Result<ProvidersConfig> {
    let text = std::fs::read_to_string(path)?;
    let v: serde_json::Value = serde_json::from_str(&text)?;

    let arr = v
        .get("providers")
        .or_else(|| v.as_array().map(|_| &v))
        .and_then(|p| p.as_array())
        .ok_or_else(|| crate::NexusError::Config("expected providers array".into()))?;

    let mut providers = Vec::new();
    for item in arr {
        let id = item
            .get("id")
            .or_else(|| item.get("name"))
            .and_then(|x| x.as_str())
            .unwrap_or("imported")
            .to_string();
        let name = item
            .get("name")
            .or_else(|| item.get("title"))
            .and_then(|x| x.as_str())
            .unwrap_or(&id)
            .to_string();

        let settings = item.get("settings").or(Some(item));
        let base_url = settings
            .and_then(|s| {
                s.get("ANTHROPIC_BASE_URL")
                    .or_else(|| s.get("base_url"))
                    .or_else(|| s.get("baseUrl"))
                    .or_else(|| s.get("OPENAI_BASE_URL"))
            })
            .and_then(|x| x.as_str())
            .unwrap_or("https://api.anthropic.com");

        let protocol_str = item
            .get("protocol")
            .and_then(|x| x.as_str())
            .unwrap_or("");
        let protocol = if protocol_str.contains("openai")
            || base_url.contains("/v1")
            && !base_url.contains("anthropic")
        {
            ApiProtocol::OpenAiChatCompletions
        } else {
            ApiProtocol::AnthropicMessages
        };

        let model = settings
            .and_then(|s| {
                s.get("ANTHROPIC_MODEL")
                    .or_else(|| s.get("model"))
                    .or_else(|| s.get("OPENAI_MODEL"))
            })
            .and_then(|x| x.as_str())
            .unwrap_or("claude-sonnet-4-20250514");

        providers.push(ProviderProfile {
            id: id.clone(),
            name,
            protocol,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.into(),
            api_key_env: Some(
                item.get("api_key_env")
                    .and_then(|x| x.as_str())
                    .unwrap_or("ANTHROPIC_AUTH_TOKEN")
                    .into(),
            ),
            api_key: item.get("api_key").and_then(|x| x.as_str()).map(String::from),
            api_key_vault: None,
            small_fast_model: None,
            proxy_hint: !base_url.contains("api.anthropic.com") && !base_url.contains("api.openai.com"),
            timeout_ms: None,
            extra_headers: Default::default(),
        });
    }

    let active = v
        .get("active")
        .and_then(|x| x.as_str())
        .map(String::from)
        .or_else(|| providers.first().map(|p| p.id.clone()));

    Ok(ProvidersConfig { active, providers })
}

/// Default Windows CC Switch Claude config path.
pub fn cc_switch_claude_settings_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let p = PathBuf::from(userprofile).join(".claude").join("settings.json");
            if p.exists() {
                return Some(p);
            }
        }
    }
    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let p = PathBuf::from(home).join(".claude").join("settings.json");
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

/// Read env var, or treat `name_or_key` as an inline key if it looks like one.
pub fn resolve_env_or_inline_key(name_or_key: &str) -> Option<String> {
    if let Ok(v) = std::env::var(name_or_key) {
        if !v.is_empty() {
            return Some(v);
        }
    }
    if looks_like_api_key(name_or_key) {
        return Some(name_or_key.to_string());
    }
    None
}

fn looks_like_api_key(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() {
        return false;
    }
    if t.starts_with("sk-") || t.starts_with("pk-") || t.starts_with("Bearer ") {
        return true;
    }
    // Env names are short identifiers; keys are long opaque strings.
    t.len() >= 24
        && !t.contains(' ')
        && t.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        && !t.chars().all(|c| c.is_ascii_alphabetic() || c == '_')
}

pub fn default_providers() -> ProvidersConfig {
    ProvidersConfig {
        active: Some("openai-official".into()),
        providers: vec![
            ProviderProfile {
                id: "anthropic-official".into(),
                name: "Anthropic Messages (原生)".into(),
                protocol: ApiProtocol::AnthropicMessages,
                base_url: "https://api.anthropic.com".into(),
                model: "claude-sonnet-4-20250514".into(),
                api_key_env: Some("ANTHROPIC_API_KEY".into()),
                api_key: None,
                api_key_vault: None,
                small_fast_model: Some("claude-3-5-haiku-20241022".into()),
                proxy_hint: false,
                timeout_ms: Some(120_000),
                extra_headers: Default::default(),
            },
            ProviderProfile {
                id: "openai-official".into(),
                name: "OpenAI Chat Completions (官方)".into(),
                protocol: ApiProtocol::OpenAiChatCompletions,
                base_url: "https://api.openai.com/v1".into(),
                model: "gpt-4o-mini".into(),
                api_key_env: Some("OPENAI_API_KEY".into()),
                api_key: None,
                api_key_vault: None,
                small_fast_model: None,
                proxy_hint: false,
                timeout_ms: Some(120_000),
                extra_headers: Default::default(),
            },
            ProviderProfile {
                id: "openai-relay-template".into(),
                name: "OpenAI Chat Completions (三方中转 — 需配置)".into(),
                protocol: ApiProtocol::OpenAiChatCompletions,
                base_url: "https://your-relay.example.com/v1".into(),
                model: "gpt-4o".into(),
                api_key_env: Some("OPENAI_API_KEY".into()),
                api_key: None,
                api_key_vault: None,
                small_fast_model: None,
                proxy_hint: true,
                timeout_ms: Some(120_000),
                extra_headers: Default::default(),
            },
        ],
    }
}
