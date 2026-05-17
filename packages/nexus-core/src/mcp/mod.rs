//! Model Context Protocol (MCP) client — stdio transport, tool discovery.

mod client;
mod stdio_transport;
mod types;

pub use client::McpClient;
pub use types::{McpServerConfig, McpTool, JsonRpcRequest, JsonRpcResponse};
