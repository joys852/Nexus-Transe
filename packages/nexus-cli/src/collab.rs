//! Workspace collaboration / presence (ROADMAP v2 §2.0).

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceLock {
    pub pid: u32,
    pub session_id: String,
    pub host: String,
    pub user: String,
    pub started_at: DateTime<Utc>,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct CollabPeer {
    pub lock: WorkspaceLock,
    pub stale: bool,
    pub is_self: bool,
}

fn lock_dir(workspace: &Path) -> PathBuf {
    workspace.join(".nexus")
}

fn lock_path(workspace: &Path) -> PathBuf {
    lock_dir(workspace).join("workspace.lock")
}

pub fn acquire_lock(workspace: &Path, session_id: Uuid, label: &str) -> anyhow::Result<()> {
    let dir = lock_dir(workspace);
    std::fs::create_dir_all(&dir)?;
    let lock = WorkspaceLock {
        pid: std::process::id(),
        session_id: session_id.to_string(),
        host: hostname(),
        user: username(),
        started_at: Utc::now(),
        label: label.to_string(),
    };
    let text = serde_json::to_string_pretty(&lock)?;
    std::fs::write(lock_path(workspace), text)?;
    Ok(())
}

pub fn release_lock(workspace: &Path) {
    let path = lock_path(workspace);
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(lock) = serde_json::from_str::<WorkspaceLock>(&text) {
            if lock.pid == std::process::id() {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

pub fn read_peers(workspace: &Path) -> Vec<CollabPeer> {
    let path = lock_path(workspace);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(lock) = serde_json::from_str::<WorkspaceLock>(&text) else {
        return Vec::new();
    };
    let stale = is_stale_lock(&path, &lock);
    let is_self = lock.pid == std::process::id();
    vec![CollabPeer {
        lock,
        stale,
        is_self,
    }]
}

fn is_stale_lock(path: &Path, lock: &WorkspaceLock) -> bool {
    if lock.pid == std::process::id() {
        return false;
    }
    if let Ok(meta) = std::fs::metadata(path) {
        if let Ok(modified) = meta.modified() {
            if let Ok(age) = SystemTime::now().duration_since(modified) {
                if age > Duration::from_secs(3600) {
                    return true;
                }
            }
        }
    }
    !pid_likely_alive(lock.pid)
}

#[cfg(windows)]
fn pid_likely_alive(pid: u32) -> bool {
    use std::process::Command;
    let out = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output();
    match out {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout);
            s.contains(&pid.to_string())
        }
        Err(_) => true,
    }
}

#[cfg(not(windows))]
fn pid_likely_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "local".into())
}

fn username() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "user".into())
}

pub fn format_peer_line(peer: &CollabPeer) -> String {
    let mark = if peer.is_self {
        "you"
    } else if peer.stale {
        "stale"
    } else {
        "active"
    };
    format!(
        "{} @ {} · session {} · {}",
        peer.lock.user,
        peer.lock.host,
        &peer.lock.session_id[..8.min(peer.lock.session_id.len())],
        mark
    )
}
