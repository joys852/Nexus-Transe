use thiserror::Error;

pub type Result<T> = std::result::Result<T, NexusError>;

#[derive(Debug, Error)]
pub enum NexusError {
    #[error("storage error: {0}")]
    Storage(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("ignore/gitignore error: {0}")]
    Ignore(#[from] ignore::Error),

    #[error("base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("utf8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("tool denied: {reason}")]
    ToolDenied { reason: String },

    #[error("approval required for tool `{tool}`")]
    ApprovalRequired { tool: String },

    #[error("invalid configuration: {0}")]
    Config(String),

    #[error("sync conflict: expected revision {expected}, got {actual}")]
    SyncConflict { expected: i64, actual: i64 },

    #[error("engine unavailable: {0}")]
    Engine(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
