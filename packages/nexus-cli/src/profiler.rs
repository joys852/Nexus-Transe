//! Runtime profiler / stats (ROADMAP v2 §2.0).

use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use nexus_core::engine::HttpEngineClient;
use nexus_core::storage::SessionRepository;
use nexus_core::sync::EngineClient;
use nexus_core::storage::sqlite::SqliteStore;
use uuid::Uuid;

const MAX_TURNS: usize = 16;

fn turn_history() -> &'static Mutex<VecDeque<TurnRecord>> {
    static H: OnceLock<Mutex<VecDeque<TurnRecord>>> = OnceLock::new();
    H.get_or_init(|| Mutex::new(VecDeque::new()))
}

#[derive(Debug, Clone)]
pub struct TurnRecord {
    pub label: String,
    pub duration: Duration,
    pub at: Instant,
}

pub fn record_turn(label: impl Into<String>, duration: Duration) {
    let mut g = turn_history().lock().unwrap_or_else(|e| e.into_inner());
    if g.len() >= MAX_TURNS {
        g.pop_front();
    }
    g.push_back(TurnRecord {
        label: label.into(),
        duration,
        at: Instant::now(),
    });
}

#[derive(Debug)]
pub struct ProfileSnapshot {
    pub engine_online: bool,
    pub engine_latency_ms: u64,
    pub session_count: usize,
    pub message_count: usize,
    pub db_bytes: u64,
    pub current_session: Uuid,
    pub recent_turns: Vec<TurnRecord>,
}

pub async fn collect(
    store: &SqliteStore,
    engine_url: &str,
    data_dir: &Path,
    current_session: Uuid,
) -> anyhow::Result<ProfileSnapshot> {
    let t0 = Instant::now();
    let engine = HttpEngineClient::new(engine_url);
    let engine_online = engine.health().await.unwrap_or(false);
    let engine_latency_ms = t0.elapsed().as_millis() as u64;

    let sessions = store.list_sessions(500).await?;
    let mut message_count = 0usize;
    for s in &sessions {
        if let Ok(msgs) = store.list_messages(s.id, 10_000).await {
            message_count += msgs.len();
        }
    }

    let db_bytes = db_file_size(data_dir);

    let recent_turns = turn_history()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .cloned()
        .collect();

    Ok(ProfileSnapshot {
        engine_online,
        engine_latency_ms,
        session_count: sessions.len(),
        message_count,
        db_bytes,
        current_session,
        recent_turns,
    })
}

fn db_file_size(data_dir: &Path) -> u64 {
    let path = data_dir.join("nexus.db");
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

pub fn format_snapshot(s: &ProfileSnapshot) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "Engine     {} · {} ms",
        if s.engine_online {
            "online"
        } else {
            "offline"
        },
        s.engine_latency_ms
    ));
    lines.push(format!(
        "Database   {} sessions · {} messages · {:.2} MB",
        s.session_count,
        s.message_count,
        s.db_bytes as f64 / 1_048_576.0
    ));
    lines.push(format!(
        "Session    {}",
        &s.current_session.to_string()[..8]
    ));
    if s.recent_turns.is_empty() {
        lines.push("Turns      (none recorded yet)".into());
    } else {
        lines.push("Turns      last activity:".into());
        for t in s.recent_turns.iter().rev().take(6) {
            lines.push(format!(
                "             {:<16} {:>4.1}s",
                t.label,
                t.duration.as_secs_f64()
            ));
        }
    }
    lines.join("\n")
}
