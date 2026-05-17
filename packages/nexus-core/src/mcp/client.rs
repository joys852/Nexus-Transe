use std::collections::HashMap;
use std::sync::Arc;

use super::stdio_transport::McpStdioTransport;
use super::types::{McpServerConfig, McpTool};
use crate::Result;

/// MCP client (stdio JSON-RPC). Transport reader/writer hardened in release pipeline.
pub struct McpClient {
    config: McpServerConfig,
    connected: bool,
    cached_tools: Vec<McpTool>,
    transport: Option<McpStdioTransport>,
}

impl McpClient {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            connected: false,
            cached_tools: Vec::new(),
            transport: None,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        if self.config.command.is_empty() {
            return Err(crate::NexusError::Config(format!(
                "MCP server {} has no command",
                self.config.name
            )));
        }
        let mut cmd: Vec<String> = vec![self.config.command.clone()];
        cmd.extend(self.config.args.clone());
        let transport = McpStdioTransport::spawn(&cmd, None).await?;
        transport.initialize().await?;
        let tools_raw = transport.list_tools().await?;
        let server = self.config.name.clone();
        self.cached_tools = tools_raw
            .into_iter()
            .map(|t| McpTool {
                server: server.clone(),
                name: t
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                description: t
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string(),
                input_schema: t
                    .get("inputSchema")
                    .cloned()
                    .unwrap_or(serde_json::json!({"type": "object"})),
            })
            .collect();
        self.transport = Some(transport);
        self.connected = true;
        tracing::info!(server = %self.config.name, tools = self.cached_tools.len(), "MCP connected");
        Ok(())
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>> {
        if !self.connected {
            return Ok(Vec::new());
        }
        Ok(self.cached_tools.clone())
    }

    pub async fn call_tool(&self, name: &str, arguments: serde_json::Value) -> Result<serde_json::Value> {
        let transport = self.transport.as_ref().ok_or_else(|| {
            crate::NexusError::Engine(format!("MCP {} not connected", self.config.name))
        })?;
        let res = transport
            .request(
                "tools/call",
                serde_json::json!({ "name": name, "arguments": arguments }),
            )
            .await?;
        Ok(res.get("result").cloned().unwrap_or(res))
    }

    pub fn register_tools(&mut self, tools: Vec<McpTool>) {
        self.cached_tools = tools;
        self.connected = true;
    }
}

#[allow(dead_code)]
pub struct McpRegistry {
    clients: HashMap<String, Arc<tokio::sync::Mutex<McpClient>>>,
}

#[allow(dead_code)]
impl McpRegistry {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub async fn register(&mut self, config: McpServerConfig) -> Result<()> {
        let mut client = McpClient::new(config.clone());
        if config.enabled {
            client.connect().await?;
        }
        self.clients
            .insert(config.name.clone(), Arc::new(tokio::sync::Mutex::new(client)));
        Ok(())
    }

    pub async fn load_from_file(&mut self, path: &std::path::Path) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }
        let text = std::fs::read_to_string(path)?;
        #[derive(serde::Deserialize)]
        struct McpFile {
            servers: Vec<McpServerConfig>,
        }
        let file: McpFile = toml::from_str(&text).map_err(|e| {
            crate::NexusError::Config(format!("mcp config: {e}"))
        })?;
        for c in file.servers {
            self.register(c).await?;
        }
        Ok(())
    }

    pub async fn discover_all(&self) -> Result<Vec<McpTool>> {
        let mut all = Vec::new();
        for client in self.clients.values() {
            let c = client.lock().await;
            all.extend(c.list_tools().await?);
        }
        Ok(all)
    }
}
