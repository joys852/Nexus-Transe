//! MCP stdio JSON-RPC transport (ROADMAP v2 §3.3).

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Mutex, oneshot};
use uuid::Uuid;

use crate::Result;

pub struct McpStdioTransport {
    stdin: Arc<Mutex<ChildStdin>>,
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<Value>>>>,
    _child: Child,
}

impl McpStdioTransport {
    pub async fn spawn(cmd: &[String], cwd: Option<&std::path::Path>) -> Result<Self> {
        let program = cmd.first().ok_or_else(|| {
            crate::NexusError::Config("MCP command empty".into())
        })?;
        let args = &cmd[1..];
        let mut command = Command::new(program);
        command.args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
        if let Some(c) = cwd {
            command.current_dir(c);
        }
        let mut child = command.spawn().map_err(crate::NexusError::Io)?;
        let stdin = child.stdin.take().ok_or_else(|| {
            crate::NexusError::Engine("MCP stdin unavailable".into())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            crate::NexusError::Engine("MCP stdout unavailable".into())
        })?;
        let pending: Arc<Mutex<HashMap<String, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_reader = pending.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let Ok(msg) = serde_json::from_str::<Value>(line) else {
                    continue;
                };
                if let Some(id) = msg.get("id").and_then(|v| v.as_str()) {
                    let mut map = pending_reader.lock().await;
                    if let Some(tx) = map.remove(id) {
                        let _ = tx.send(msg);
                    }
                }
            }
        });
        Ok(Self {
            stdin: Arc::new(Mutex::new(stdin)),
            pending,
            _child: child,
        })
    }

    pub async fn request(&self, method: &str, params: Value) -> Result<Value> {
        let id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id.clone(), tx);
        let payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let mut stdin = self.stdin.lock().await;
        stdin
            .write_all(format!("{payload}\n").as_bytes())
            .await
            .map_err(crate::NexusError::Io)?;
        stdin.flush().await.map_err(crate::NexusError::Io)?;
        let msg = rx
            .await
            .map_err(|_| crate::NexusError::Engine("MCP response channel closed".into()))?;
        Ok(msg)
    }

    pub async fn initialize(&self) -> Result<Value> {
        self.request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "nexus-core", "version": "0.1.0" },
            }),
        )
        .await
    }

    pub async fn list_tools(&self) -> Result<Vec<Value>> {
        let res = self.request("tools/list", json!({})).await?;
        Ok(res
            .get("result")
            .and_then(|r| r.get("tools"))
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default())
    }
}
