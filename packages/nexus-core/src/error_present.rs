//! Rich error presentation (ROADMAP v2 §2.1).

use crate::error::NexusError;

#[derive(Debug, Clone)]
pub struct ErrorPresentation {
    pub code: &'static str,
    pub title: String,
    pub context: Vec<String>,
    pub suggestion: Option<&'static str>,
    pub docs_path: Option<&'static str>,
}

impl ErrorPresentation {
    pub fn docs_url(&self) -> Option<String> {
        self.docs_path
            .map(|p| format!("https://nexuside.dev/docs/errors/{p}"))
    }
}

pub fn present(err: &NexusError) -> ErrorPresentation {
    match err {
        NexusError::Storage(e) => ErrorPresentation {
            code: "E042",
            title: format!("Database error: {e}"),
            context: vec![],
            suggestion: Some(
                "Close other Nexus processes or run: nexus db doctor (if available)",
            ),
            docs_path: Some("E042"),
        },
        NexusError::Migration(e) => ErrorPresentation {
            code: "E043",
            title: format!("Migration failed: {e}"),
            context: vec![],
            suggestion: Some("Back up nexus.db and restart, or reinstall the schema"),
            docs_path: Some("E043"),
        },
        NexusError::Config(msg) => ErrorPresentation {
            code: "E101",
            title: msg.clone(),
            context: vec![],
            suggestion: Some("Run: nexus provider doctor · check providers.toml"),
            docs_path: Some("E101"),
        },
        NexusError::Engine(msg) => ErrorPresentation {
            code: "E201",
            title: msg.clone(),
            context: vec![],
            suggestion: Some("Start engine: cd packages/nexus-engine && uv run nexus-engine"),
            docs_path: Some("E201"),
        },
        NexusError::ToolDenied { reason } => ErrorPresentation {
            code: "E301",
            title: reason.clone(),
            context: vec![],
            suggestion: Some("Approve the tool when prompted or use /approve for this session"),
            docs_path: Some("E301"),
        },
        NexusError::ApprovalRequired { tool } => ErrorPresentation {
            code: "E302",
            title: format!("Approval required for `{tool}`"),
            context: vec![],
            suggestion: Some("Use arrow keys to approve in the interactive menu"),
            docs_path: Some("E302"),
        },
        NexusError::SyncConflict { expected, actual } => ErrorPresentation {
            code: "E044",
            title: format!("Sync conflict: expected revision {expected}, got {actual}"),
            context: vec![],
            suggestion: Some("Retry the command or start a new session with /new"),
            docs_path: Some("E044"),
        },
        NexusError::Io(e) => ErrorPresentation {
            code: "E001",
            title: format!("I/O error: {e}"),
            context: vec![],
            suggestion: None,
            docs_path: Some("E001"),
        },
        NexusError::Json(e) => ErrorPresentation {
            code: "E002",
            title: format!("JSON error: {e}"),
            context: vec![],
            suggestion: None,
            docs_path: Some("E002"),
        },
        NexusError::Other(e) => ErrorPresentation {
            code: "E000",
            title: e.to_string(),
            context: vec![],
            suggestion: None,
            docs_path: Some("E000"),
        },
        other => ErrorPresentation {
            code: "E000",
            title: other.to_string(),
            context: vec![],
            suggestion: None,
            docs_path: Some("E000"),
        },
    }
}

pub fn present_anyhow(err: &anyhow::Error) -> ErrorPresentation {
    if let Some(ne) = err.downcast_ref::<NexusError>() {
        return present(ne);
    }
    let mut context = Vec::new();
    let mut source = err.source();
    while let Some(s) = source {
        context.push(s.to_string());
        source = s.source();
    }
    ErrorPresentation {
        code: "E000",
        title: err.to_string(),
        context,
        suggestion: Some("Run with RUST_LOG=nexus=debug for details"),
        docs_path: Some("E000"),
    }
}
