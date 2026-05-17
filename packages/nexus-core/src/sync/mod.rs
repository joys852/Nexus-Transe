use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod memory;

use crate::Result;

pub use memory::MemorySyncBus;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncEvent {
    SessionUpdated {
        session_id: Uuid,
        revision: i64,
    },
    MessageAppended {
        session_id: Uuid,
        message_id: Uuid,
        sequence: i64,
    },
    TaskStatusChanged {
        session_id: Uuid,
        status: String,
    },
    ToolApprovalRequired {
        session_id: Uuid,
        call_id: String,
        tool_name: String,
    },
    StreamDelta {
        session_id: Uuid,
        delta: String,
    },
}

/// CLI session sync over local IPC (extensible for multi-client).
#[async_trait]
pub trait SyncBus: Send + Sync {
    async fn publish(&self, event: SyncEvent) -> Result<u64>;
    async fn subscribe(
        &self,
        client_id: &str,
        since_event_id: u64,
    ) -> Result<tokio::sync::mpsc::Receiver<SyncEvent>>;
}

#[async_trait]
pub trait EngineClient: Send + Sync {
    async fn health(&self) -> Result<bool>;
    async fn run_task(
        &self,
        session_id: Uuid,
        prompt: &str,
        model_id: Option<&str>,
    ) -> Result<()>;
    async fn pause_task(&self, session_id: Uuid) -> Result<()>;
    async fn resume_task(&self, session_id: Uuid) -> Result<()>;
}
