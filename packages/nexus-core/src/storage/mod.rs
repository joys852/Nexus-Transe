use async_trait::async_trait;
use uuid::Uuid;

use crate::models::{Message, MessageRole, Session, SessionStatus};
use crate::Result;

pub mod sqlite;

/// Persistence layer: sessions, messages, checkpoints, audit.
#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create_session(&self, workspace_id: Option<Uuid>, title: Option<&str>) -> Result<Session>;
    async fn get_session(&self, id: Uuid) -> Result<Option<Session>>;
    async fn update_status(&self, id: Uuid, status: SessionStatus, revision: i64) -> Result<Session>;
    async fn append_message(
        &self,
        session_id: Uuid,
        role: MessageRole,
        content: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<Message>;
    async fn list_messages(&self, session_id: Uuid, limit: u32) -> Result<Vec<Message>>;
    /// Replace all messages for a session (e.g. after `/compact`).
    async fn replace_session_messages(
        &self,
        session_id: Uuid,
        messages: &[(MessageRole, String)],
    ) -> Result<()>;
    async fn list_sessions(&self, limit: u32) -> Result<Vec<Session>>;
}

#[async_trait]
pub trait CheckpointStore: Send + Sync {
    async fn save_checkpoint(
        &self,
        session_id: Uuid,
        thread_id: &str,
        checkpoint_id: &str,
        state: &[u8],
    ) -> Result<()>;
    async fn load_latest(&self, session_id: Uuid, thread_id: &str) -> Result<Option<Vec<u8>>>;
}

#[async_trait]
pub trait AuditLog: Send + Sync {
    async fn record_tool_call(
        &self,
        session_id: Option<Uuid>,
        tool_name: &str,
        arguments: &serde_json::Value,
        status: &str,
        duration_ms: Option<u64>,
    ) -> Result<()>;
}
