use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;

use super::{AuditLog, CheckpointStore, SessionRepository};
use crate::models::{Message, MessageRole, Session, SessionStatus};
use crate::Result;

pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub async fn connect(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path.display()))?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .pragma("cache_size", "-64000")
            .pragma("busy_timeout", "5000");
        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .min_connections(1)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect_with(options)
            .await?;
        sqlx::migrate!().run(&pool).await?;
        Ok(Self { pool })
    }
}

fn parse_session(row: &sqlx::sqlite::SqliteRow) -> Session {
    let status = match row.get::<String, _>("status").as_str() {
        "paused" => SessionStatus::Paused,
        "completed" => SessionStatus::Completed,
        "failed" => SessionStatus::Failed,
        _ => SessionStatus::Active,
    };
    Session {
        id: Uuid::parse_str(row.get::<String, _>("id").as_str()).unwrap_or_else(|_| Uuid::nil()),
        workspace_id: row
            .get::<Option<String>, _>("workspace_id")
            .and_then(|s| Uuid::parse_str(&s).ok()),
        title: row.get("title"),
        status,
        revision: row.get("revision"),
        model_id: row.get("model_id"),
        agent_profile: row.get("agent_profile"),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

#[async_trait]
impl SessionRepository for SqliteStore {
    async fn create_session(&self, workspace_id: Option<Uuid>, title: Option<&str>) -> Result<Session> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO sessions (id, workspace_id, title, status, revision, agent_profile)
               VALUES (?1, ?2, ?3, 'active', 0, 'default')"#,
        )
        .bind(id.to_string())
        .bind(workspace_id.map(|u| u.to_string()))
        .bind(title)
        .execute(&self.pool)
        .await?;
        self.get_session(id).await?.ok_or_else(|| {
            crate::NexusError::Other(anyhow::anyhow!("session not found after insert"))
        })
    }

    async fn get_session(&self, id: Uuid) -> Result<Option<Session>> {
        let row = sqlx::query(
            r#"SELECT id, workspace_id, title, status, revision, model_id, agent_profile,
                      created_at, updated_at FROM sessions WHERE id = ?1"#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.as_ref().map(parse_session))
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: SessionStatus,
        revision: i64,
    ) -> Result<Session> {
        let status_str = match status {
            SessionStatus::Active => "active",
            SessionStatus::Paused => "paused",
            SessionStatus::Completed => "completed",
            SessionStatus::Failed => "failed",
        };
        let updated = sqlx::query(
            r#"UPDATE sessions SET status = ?1, revision = ?2, updated_at = datetime('now')
               WHERE id = ?3 AND revision = ?4"#,
        )
        .bind(status_str)
        .bind(revision + 1)
        .bind(id.to_string())
        .bind(revision)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() == 0 {
            return Err(crate::NexusError::SyncConflict {
                expected: revision,
                actual: revision,
            });
        }
        self.get_session(id).await?.ok_or_else(|| {
            crate::NexusError::Other(anyhow::anyhow!("session not found"))
        })
    }

    async fn append_message(
        &self,
        session_id: Uuid,
        role: MessageRole,
        content: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<Message> {
        let id = Uuid::new_v4();
        let role_str = match role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
            MessageRole::Tool => "tool",
        };
        let seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(sequence), -1) + 1 FROM messages WHERE session_id = ?1",
        )
        .bind(session_id.to_string())
        .fetch_one(&self.pool)
        .await?;
        sqlx::query(
            r#"INSERT INTO messages (id, session_id, role, content, metadata_json, sequence)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
        )
        .bind(id.to_string())
        .bind(session_id.to_string())
        .bind(role_str)
        .bind(content)
        .bind(metadata.as_ref().map(|v| v.to_string()))
        .bind(seq)
        .execute(&self.pool)
        .await?;
        Ok(Message {
            id,
            session_id,
            role,
            content: content.to_string(),
            metadata,
            parent_id: None,
            sequence: seq,
            created_at: chrono::Utc::now(),
        })
    }

    async fn replace_session_messages(
        &self,
        session_id: Uuid,
        messages: &[(MessageRole, String)],
    ) -> Result<()> {
        sqlx::query("DELETE FROM messages WHERE session_id = ?1")
            .bind(session_id.to_string())
            .execute(&self.pool)
            .await?;
        for (role, content) in messages {
            self.append_message(session_id, role.clone(), content, None)
                .await?;
        }
        Ok(())
    }

    async fn list_messages(&self, session_id: Uuid, limit: u32) -> Result<Vec<Message>> {
        let rows = sqlx::query(
            r#"SELECT id, session_id, role, content, metadata_json, parent_id, sequence, created_at
               FROM messages WHERE session_id = ?1 ORDER BY sequence DESC LIMIT ?2"#,
        )
        .bind(session_id.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|r| {
                let role = match r.get::<String, _>("role").as_str() {
                    "assistant" => MessageRole::Assistant,
                    "system" => MessageRole::System,
                    "tool" => MessageRole::Tool,
                    _ => MessageRole::User,
                };
                Message {
                    id: Uuid::parse_str(r.get::<String, _>("id").as_str())
                        .unwrap_or_else(|_| Uuid::nil()),
                    session_id: Uuid::parse_str(r.get::<String, _>("session_id").as_str())
                        .unwrap_or_else(|_| Uuid::nil()),
                    role,
                    content: r.get("content"),
                    metadata: r
                        .get::<Option<String>, _>("metadata_json")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    parent_id: r
                        .get::<Option<String>, _>("parent_id")
                        .and_then(|s| Uuid::parse_str(&s).ok()),
                    sequence: r.get("sequence"),
                    created_at: chrono::Utc::now(),
                }
            })
            .collect())
    }

    async fn list_sessions(&self, limit: u32) -> Result<Vec<Session>> {
        let rows = sqlx::query(
            r#"SELECT id, workspace_id, title, status, revision, model_id, agent_profile,
                      created_at, updated_at
               FROM sessions ORDER BY updated_at DESC LIMIT ?1"#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(parse_session).collect())
    }
}

#[async_trait]
impl CheckpointStore for SqliteStore {
    async fn save_checkpoint(
        &self,
        session_id: Uuid,
        thread_id: &str,
        checkpoint_id: &str,
        state: &[u8],
    ) -> Result<()> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO checkpoints (id, session_id, thread_id, checkpoint_id, state_blob)
               VALUES (?1, ?2, ?3, ?4, ?5)"#,
        )
        .bind(id.to_string())
        .bind(session_id.to_string())
        .bind(thread_id)
        .bind(checkpoint_id)
        .bind(state)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_latest(&self, session_id: Uuid, thread_id: &str) -> Result<Option<Vec<u8>>> {
        let row = sqlx::query(
            r#"SELECT state_blob FROM checkpoints
               WHERE session_id = ?1 AND thread_id = ?2
               ORDER BY created_at DESC LIMIT 1"#,
        )
        .bind(session_id.to_string())
        .bind(thread_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.get::<Vec<u8>, _>("state_blob")))
    }
}

#[async_trait]
impl AuditLog for SqliteStore {
    async fn record_tool_call(
        &self,
        session_id: Option<Uuid>,
        tool_name: &str,
        arguments: &serde_json::Value,
        status: &str,
        duration_ms: Option<u64>,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO audit_log (session_id, tool_name, arguments_json, result_status, duration_ms)
               VALUES (?1, ?2, ?3, ?4, ?5)"#,
        )
        .bind(session_id.map(|u| u.to_string()))
        .bind(tool_name)
        .bind(arguments.to_string())
        .bind(status)
        .bind(duration_ms.map(|d| d as i64))
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
