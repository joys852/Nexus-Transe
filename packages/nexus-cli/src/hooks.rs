//! PreToolUse hooks — `.nexus/hooks.toml` policy gates.

use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookOutcome {
    /// No hook matched — use normal approval mode.
    Pass,
    Allow,
    Deny(String),
    RequireApproval,
}

#[derive(Debug, Deserialize)]
struct HooksFile {
    #[serde(default)]
    pre_tool_use: Vec<PreToolUseHook>,
}

#[derive(Debug, Deserialize)]
struct PreToolUseHook {
    /// Tool name or `*` for all.
    tools: Vec<String>,
    /// `allow` | `deny` | `prompt`
    action: String,
    /// Optional substring match against serialized arguments (e.g. shell command).
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default)]
    message: Option<String>,
    /// Optional shell command (exit 0=allow, 1=deny, 2=prompt).
    #[serde(default)]
    command: Option<String>,
}

fn hooks_path(workspace: &Path) -> PathBuf {
    workspace.join(".nexus").join("hooks.toml")
}

fn load_hooks(workspace: &Path) -> HooksFile {
    let path = hooks_path(workspace);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return HooksFile {
            pre_tool_use: Vec::new(),
        };
    };
    toml::from_str(&text).unwrap_or(HooksFile {
        pre_tool_use: Vec::new(),
    })
}

/// Evaluate PreToolUse hooks before interactive approval.
pub fn pre_tool_use(
    workspace: &Path,
    tool_name: &str,
    arguments: &serde_json::Value,
) -> HookOutcome {
    let file = load_hooks(workspace);
    let args_text = arguments.to_string().to_lowercase();
    for hook in &file.pre_tool_use {
        let matches_tool = hook.tools.iter().any(|t| t == "*" || t == tool_name);
        if !matches_tool {
            continue;
        }
        if let Some(pat) = &hook.pattern {
            if !args_text.contains(&pat.to_lowercase()) {
                continue;
            }
        }
        if let Some(cmd) = &hook.command {
            if let Some(outcome) = run_hook_command(workspace, cmd, tool_name, arguments) {
                return outcome;
            }
        }
        return match hook.action.to_lowercase().as_str() {
            "allow" | "auto" => HookOutcome::Allow,
            "deny" | "block" => HookOutcome::Deny(
                hook.message
                    .clone()
                    .unwrap_or_else(|| format!("blocked by .nexus/hooks.toml ({tool_name})")),
            ),
            _ => HookOutcome::RequireApproval,
        };
    }
    HookOutcome::Pass
}

fn run_hook_command(
    workspace: &Path,
    command: &str,
    tool_name: &str,
    arguments: &serde_json::Value,
) -> Option<HookOutcome> {
    let args_json = arguments.to_string();
    #[cfg(windows)]
    let output = Command::new("cmd")
        .args(["/C", command])
        .current_dir(workspace)
        .env("NEXUS_TOOL", tool_name)
        .env("NEXUS_ARGS", &args_json)
        .output()
        .ok()?;
    #[cfg(not(windows))]
    let output = Command::new("sh")
        .args(["-c", command])
        .current_dir(workspace)
        .env("NEXUS_TOOL", tool_name)
        .env("NEXUS_ARGS", &args_json)
        .output()
        .ok()?;
    let status = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr);
    match status {
        0 => Some(HookOutcome::Allow),
        1 => Some(HookOutcome::Deny(if stderr.is_empty() {
            format!("hook command denied ({command})")
        } else {
            stderr.trim().to_string()
        })),
        2 => Some(HookOutcome::RequireApproval),
        _ => Some(HookOutcome::Deny(format!(
            "hook exited {status}: {}",
            stderr.trim()
        ))),
    }
}

pub fn example_hooks_toml() -> &'static str {
    r#"# Nexus-Transe PreToolUse hooks
[[pre_tool_use]]
tools = ["run_shell"]
action = "deny"
pattern = "rm -rf"
message = "Destructive rm is blocked in this workspace"

[[pre_tool_use]]
tools = ["write_file", "edit_file"]
action = "prompt"
"#
}
