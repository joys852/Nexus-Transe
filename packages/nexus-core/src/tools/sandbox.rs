//! Shell / path sandbox guards before tool execution.

use super::ToolHandler;
use crate::models::ToolCallRequest;
use crate::Result;

const BLOCKED_SHELL_FRAGMENTS: &[&str] = &[
    "rm -rf /",
    "rm -rf /*",
    "format c:",
    "format c:\\",
    ":(){ :|:& };:",
    "mkfs.",
    "dd if=/dev/zero",
    "> /dev/sda",
];

const BLOCKED_PREFIXES: &[&str] = &[
    "curl ",
    "wget ",
    "invoke-webrequest",
    "iwr ",
];

/// Validate shell command before execution; returns error message if blocked.
pub fn validate_shell_command(command: &str) -> Result<()> {
    let lower = command.to_lowercase();
    for frag in BLOCKED_SHELL_FRAGMENTS {
        if lower.contains(frag) {
            return Err(crate::NexusError::ToolDenied {
                reason: format!("blocked command pattern: {frag}"),
            });
        }
    }
    for prefix in BLOCKED_PREFIXES {
        if lower.starts_with(prefix) && !lower.contains("localhost") {
            return Err(crate::NexusError::ToolDenied {
                reason: format!(
                    "network fetch blocked in sandbox (prefix: {prefix}); use MCP or explicit allow"
                ),
            });
        }
    }
    if lower.len() > 8_000 {
        return Err(crate::NexusError::ToolDenied {
            reason: "command too long".into(),
        });
    }
    Ok(())
}

/// Wrap tool execution (shell validation happens in builtin handlers).
pub async fn run_in_sandbox(
    handler: &dyn ToolHandler,
    request: ToolCallRequest,
) -> Result<crate::models::ToolCallResult> {
    handler.execute(request).await
}
