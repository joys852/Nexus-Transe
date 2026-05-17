//! Tool approval modes — Codex-style Suggest / AutoEdit / FullAuto.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApprovalMode {
    /// Reads auto; writes & shell always prompt (unless session yolo).
    #[default]
    Suggest,
    /// Auto-approve file writes/edits; shell & MCP still prompt.
    AutoEdit,
    /// Auto-approve all built-in risky tools (like /yolo for the session turn).
    FullAuto,
}

impl ApprovalMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().replace('_', "-").as_str() {
            "suggest" | "default" => Some(Self::Suggest),
            "auto-edit" | "autoedit" | "edit" => Some(Self::AutoEdit),
            "full-auto" | "fullauto" | "auto" => Some(Self::FullAuto),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Suggest => "suggest",
            Self::AutoEdit => "auto-edit",
            Self::FullAuto => "full-auto",
        }
    }

    /// True → skip interactive approval for this tool when policy returns PendingApproval.
    pub fn auto_approve_pending_tool(&self, tool_name: &str) -> bool {
        match self {
            Self::FullAuto => true,
            Self::AutoEdit => {
                matches!(
                    tool_name,
                    "write_file" | "edit_file" | "git_commit" | "git_push"
                )
            }
            Self::Suggest => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_modes() {
        assert_eq!(ApprovalMode::parse("auto-edit"), Some(ApprovalMode::AutoEdit));
        assert_eq!(ApprovalMode::parse("full-auto"), Some(ApprovalMode::FullAuto));
    }

    #[test]
    fn auto_edit_writes_not_shell() {
        let m = ApprovalMode::AutoEdit;
        assert!(m.auto_approve_pending_tool("write_file"));
        assert!(!m.auto_approve_pending_tool("run_shell"));
    }
}
