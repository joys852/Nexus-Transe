//! Persist active REPL session per project (resume after exit).

use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SessionPin {
    pub session_id: Uuid,
    pub title: Option<String>,
    pub updated_at: String,
}

impl SessionPin {
    pub fn path(project_root: &Path) -> PathBuf {
        project_root.join(".nexus").join("session.json")
    }

    pub fn load(project_root: &Path) -> Option<Self> {
        let path = Self::path(project_root);
        let raw = fs::read_to_string(path).ok()?;
        serde_json::from_str(&raw).ok()
    }

    pub fn save(project_root: &Path, session_id: Uuid, title: Option<&str>) -> anyhow::Result<()> {
        let dir = project_root.join(".nexus");
        fs::create_dir_all(&dir)?;
        let pin = SessionPin {
            session_id,
            title: title.map(String::from),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        fs::write(Self::path(project_root), serde_json::to_string_pretty(&pin)?)?;
        Ok(())
    }
}
