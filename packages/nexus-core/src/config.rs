use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NexusConfig {
    pub data_dir: PathBuf,
    pub engine_url: String,
    pub sync_socket: PathBuf,
    pub default_model: String,
    pub log_level: String,
    /// Terminal theme: light | dark | carbon
    pub theme: String,
    /// Shell sandbox: local | docker
    pub sandbox_mode: String,
}

impl NexusConfig {
    /// Validate config after merge (ROADMAP v2 §2.2).
    pub fn validate(&self) -> crate::Result<()> {
        if self.engine_url.trim().is_empty() {
            return Err(crate::NexusError::Config(
                "engine_url must not be empty".into(),
            ));
        }
        if !self.engine_url.starts_with("http://") && !self.engine_url.starts_with("https://") {
            return Err(crate::NexusError::Config(format!(
                "engine_url must be http(s), got: {}",
                self.engine_url
            )));
        }
        if self.default_model.trim().is_empty() {
            return Err(crate::NexusError::Config(
                "default_model must not be empty".into(),
            ));
        }
        if self.log_level != "trace"
            && self.log_level != "debug"
            && self.log_level != "info"
            && self.log_level != "warn"
            && self.log_level != "error"
        {
            return Err(crate::NexusError::Config(format!(
                "invalid log_level: {}",
                self.log_level
            )));
        }
        let t = self.theme.to_lowercase();
        if t != "light" && t != "dark" && t != "carbon" {
            return Err(crate::NexusError::Config(format!(
                "invalid theme: {} (use light, dark, or carbon)",
                self.theme
            )));
        }
        let sm = self.sandbox_mode.to_lowercase();
        if sm != "local" && sm != "docker" {
            return Err(crate::NexusError::Config(format!(
                "invalid sandbox_mode: {} (use local or docker)",
                self.sandbox_mode
            )));
        }
        Ok(())
    }

    /// Load with priority: defaults < file < env (ROADMAP v2 §2.2).
    pub fn load_merged() -> crate::Result<Self> {
        let mut cfg = Self::default();
        let config_path = std::env::var("NEXUS_CONFIG")
            .ok()
            .map(std::path::PathBuf::from)
            .or_else(|| {
                dirs::config_dir().map(|d| d.join("nexus").join("config.toml"))
            });
        if let Some(path) = config_path {
            if path.exists() {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    if let Ok(file) = toml::from_str::<NexusConfigFile>(&text) {
                        if let Some(m) = file.default_model {
                            cfg.default_model = m;
                        }
                        if let Some(u) = file.engine_url {
                            cfg.engine_url = u;
                        }
                        if let Some(d) = file.data_dir {
                            cfg.data_dir = d.into();
                        }
                        if let Some(l) = file.log_level {
                            cfg.log_level = l;
                        }
                        if let Some(t) = file.theme {
                            cfg.theme = t;
                        }
                        if let Some(s) = file.sandbox_mode {
                            cfg.sandbox_mode = s;
                        }
                    }
                }
            }
        }
        if let Ok(s) = std::env::var("NEXUS_SANDBOX") {
            cfg.sandbox_mode = s;
        }
        if let Ok(t) = std::env::var("NEXUS_THEME") {
            cfg.theme = t;
        }
        if let Ok(u) = std::env::var("NEXUS_ENGINE_URL") {
            cfg.engine_url = u;
        }
        if let Ok(m) = std::env::var("NEXUS_DEFAULT_MODEL") {
            cfg.default_model = m;
        }
        if let Ok(d) = std::env::var("NEXUS_DATA_DIR") {
            cfg.data_dir = d.into();
        }
        if let Ok(l) = std::env::var("NEXUS_LOG_LEVEL") {
            cfg.log_level = l;
        }
        let store = crate::providers::ProvidersStore::new(&cfg.data_dir);
        if let Ok(providers) = store.load() {
            if let Some(p) = providers.active_provider() {
                cfg.default_model = p.model.clone();
            }
        }
        cfg.validate()?;
        if let Some(s) = cfg.data_dir.to_str() {
            std::env::set_var("NEXUS_DATA_DIR", s);
        }
        Ok(cfg)
    }

    /// Persist config fields to `{data_dir}/config.toml`.
    pub fn save_to_data_dir(&self) -> crate::Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        let path = self.data_dir.join("config.toml");
        let body = format!(
            r#"engine_url = "{engine_url}"
default_model = "{default_model}"
log_level = "{log_level}"
theme = "{theme}"
sandbox_mode = "{sandbox_mode}"
"#,
            engine_url = self.engine_url,
            default_model = self.default_model,
            log_level = self.log_level,
            theme = self.theme,
            sandbox_mode = self.sandbox_mode,
        );
        std::fs::write(path, body)?;
        Ok(())
    }
}

#[derive(serde::Deserialize)]
struct NexusConfigFile {
    default_model: Option<String>,
    engine_url: Option<String>,
    data_dir: Option<String>,
    log_level: Option<String>,
    theme: Option<String>,
    sandbox_mode: Option<String>,
}

impl Default for NexusConfig {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nexus-ide");
        Self {
            data_dir: data_dir.clone(),
            engine_url: "http://127.0.0.1:8765".into(),
            sync_socket: data_dir.join("sync.sock"),
            default_model: "claude-sonnet-4-20250514".into(),
            log_level: "info".into(),
            theme: "dark".into(),
            sandbox_mode: "local".into(),
        }
    }
}

pub trait ConfigStore: Send + Sync {
    fn load(&self) -> crate::Result<NexusConfig>;
    fn save(&self, config: &NexusConfig) -> crate::Result<()>;
}
