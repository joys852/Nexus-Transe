use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub title: Option<String>,
    pub status: SessionStatus,
    pub revision: i64,
    pub model_id: Option<String>,
    pub agent_profile: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub session_id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub parent_id: Option<Uuid>,
    pub sequence: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRunStatus {
    Idle,
    Planning,
    Acting,
    Observing,
    Paused,
    Done,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCheckpoint {
    pub session_id: Uuid,
    pub thread_id: String,
    pub checkpoint_id: String,
    pub status: TaskRunStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionAction {
    Allow,
    Deny,
    Ask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPolicy {
    pub id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub tool_name: Option<String>,
    pub resource_pattern: Option<String>,
    pub action: PermissionAction,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub source: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub session_id: Uuid,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub call_id: String,
    /// User approved a previously pending tool call.
    #[serde(default)]
    pub approved: bool,
    #[serde(skip)]
    pub workspace: Option<std::sync::Arc<crate::project::WorkspaceToolContext>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolResultStatus {
    Ok,
    Denied,
    Error,
    PendingApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub call_id: String,
    pub status: ToolResultStatus,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub file_path: String,
    pub language: Option<String>,
    pub symbol_name: Option<String>,
    pub symbol_kind: Option<String>,
    pub start_line: u32,
    pub end_line: u32,
    pub content_hash: String,
    pub chroma_id: Option<String>,
}
