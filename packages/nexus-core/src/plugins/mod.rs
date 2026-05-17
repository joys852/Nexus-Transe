//! Plugin system — manifest loading, script runner (WASM planned).

pub mod runner;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub permissions: Vec<PluginPermission>,
    pub entry: String,
    pub min_nexus_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermission {
    ReadFiles,
    WriteFiles,
    RunShell,
    Network,
    McpBridge,
}

#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub root: PathBuf,
}

pub struct PluginManager {
    plugins_dir: PathBuf,
    loaded: Vec<LoadedPlugin>,
}

impl PluginManager {
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self {
            plugins_dir,
            loaded: Vec::new(),
        }
    }

    pub fn scan(&mut self) -> Result<()> {
        self.loaded.clear();
        if !self.plugins_dir.is_dir() {
            std::fs::create_dir_all(&self.plugins_dir)?;
            return Ok(());
        }
        for entry in std::fs::read_dir(&self.plugins_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let manifest_path = entry.path().join("plugin.toml");
            if manifest_path.is_file() {
                let text = std::fs::read_to_string(&manifest_path)?;
                let manifest: PluginManifest = toml::from_str(&text).map_err(|e| {
                    crate::NexusError::Config(format!("plugin manifest: {e}"))
                })?;
                self.loaded.push(LoadedPlugin {
                    manifest,
                    root: entry.path(),
                });
            }
        }
        Ok(())
    }

    pub fn list(&self) -> &[LoadedPlugin] {
        &self.loaded
    }

    pub fn has_permission(&self, plugin_id: &str, perm: PluginPermission) -> bool {
        self.loaded
            .iter()
            .find(|p| p.manifest.id == plugin_id)
            .map(|p| p.manifest.permissions.contains(&perm))
            .unwrap_or(false)
    }

    /// Scaffold a marketplace plugin under `plugins_dir/{id}/plugin.toml`.
    pub fn install_scaffold(
        &mut self,
        id: &str,
        name: &str,
        version: &str,
        description: &str,
        permissions: &[PluginPermission],
    ) -> Result<PathBuf> {
        let root = self.plugins_dir.join(id);
        if root.join("plugin.toml").is_file() {
            return Err(crate::NexusError::Config(format!(
                "plugin already installed: {id}"
            )));
        }
        std::fs::create_dir_all(&root)?;
        let perms: Vec<String> = permissions
            .iter()
            .map(|p| match p {
                PluginPermission::ReadFiles => "read_files",
                PluginPermission::WriteFiles => "write_files",
                PluginPermission::RunShell => "run_shell",
                PluginPermission::Network => "network",
                PluginPermission::McpBridge => "mcp_bridge",
            })
            .map(String::from)
            .collect();
        let perms_toml = if perms.is_empty() {
            "read_files".to_string()
        } else {
            format!(
                "[{}]",
                perms
                    .iter()
                    .map(|p| format!("\"{p}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let manifest = format!(
            r#"id = "{id}"
name = "{name}"
version = "{version}"
description = "{description}"
permissions = {perms_toml}
entry = "main.wasm"
min_nexus_version = "0.1.0"
"#
        );
        std::fs::write(root.join("plugin.toml"), manifest)?;
        std::fs::write(
            root.join("README.md"),
            format!("# {name}\n\n{description}\n\nInstalled from NexusIDE marketplace.\n"),
        )?;
        #[cfg(windows)]
        std::fs::write(
            root.join("run.ps1"),
            "Write-Host \"NexusIDE plugin $env:NEXUS_PLUGIN_ID ready\"\n",
        )?;
        #[cfg(not(windows))]
        std::fs::write(
            root.join("run.sh"),
            "#!/bin/sh\necho \"NexusIDE plugin ready\"\n",
        )?;
        self.scan()?;
        Ok(root)
    }
}

pub fn default_plugins_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("plugins")
}
