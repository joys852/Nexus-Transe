use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use crate::project::WorkspaceToolContext;
use super::ToolHandler;
use crate::models::{
    PermissionAction, PermissionPolicy, ToolCallRequest, ToolCallResult, ToolDefinition,
    ToolResultStatus,
};
use crate::Result;

pub struct ReadFileTool;
pub struct WriteFileTool;
pub struct EditFileTool;
pub struct RunShellTool;
pub struct GitStatusTool;
pub struct GitDiffTool;
pub struct GitBranchTool;
pub struct GitCommitTool;
pub struct GitLogTool;
pub struct GitPushTool;
pub struct SemanticSearchTool;
pub struct GlobFilesTool;

fn read_path_arg(args: &serde_json::Value) -> Result<String> {
    args.get("path")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| crate::NexusError::Other(anyhow::anyhow!("missing path")))
}

#[async_trait]
impl ToolHandler for ReadFileTool {
    fn definition(&self) -> ToolDefinition {
        tool_def(
            "read_file",
            "Read a text file from the workspace (source, config, CLAUDE.md, PROJECT.md, *.md)",
            serde_json::json!({
            "type": "object",
            "properties": { "path": { "type": "string" } },
            "required": ["path"]
        }))
    }

    async fn execute(&self, request: ToolCallRequest) -> Result<ToolCallResult> {
        let ctx = ctx_from_request(&request)?;
        let rel = read_path_arg(&request.arguments)?;
        let path = ctx.resolve(&rel);
        ctx.ensure_in_workspace(&path)?;
        let content = read_capped(path, 128 * 1024).await?;
        ok_result(&request.call_id, serde_json::json!({ "content": content, "path": rel }))
    }
}

#[async_trait]
impl ToolHandler for WriteFileTool {
    fn definition(&self) -> ToolDefinition {
        tool_def("write_file", "Write content to a file (creates parent dirs)", serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "content": { "type": "string" }
            },
            "required": ["path", "content"]
        }))
    }

    async fn execute(&self, request: ToolCallRequest) -> Result<ToolCallResult> {
        let ctx = ctx_from_request(&request)?;
        if !ctx.auto_approve && !request.approved {
            return pending(&request.call_id);
        }
        let rel = read_path_arg(&request.arguments)?;
        let content = request
            .arguments
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::NexusError::Other(anyhow::anyhow!("missing content")))?;
        let path = ctx.resolve(&rel);
        ctx.ensure_in_workspace(&path)?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, content).await?;
        ok_result(&request.call_id, serde_json::json!({ "path": rel, "bytes": content.len() }))
    }
}

#[async_trait]
impl ToolHandler for EditFileTool {
    fn definition(&self) -> ToolDefinition {
        tool_def(
            "edit_file",
            "Replace old_string with new_string in a file (must be unique)",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "old_string": { "type": "string" },
                    "new_string": { "type": "string" }
                },
                "required": ["path", "old_string", "new_string"]
            }),
        )
    }

    async fn execute(&self, request: ToolCallRequest) -> Result<ToolCallResult> {
        let ctx = ctx_from_request(&request)?;
        if !ctx.auto_approve && !request.approved {
            return pending(&request.call_id);
        }
        let rel = read_path_arg(&request.arguments)?;
        let old_s = request.arguments["old_string"].as_str().unwrap_or("");
        let new_s = request.arguments["new_string"].as_str().unwrap_or("");
        let path = ctx.resolve(&rel);
        ctx.ensure_in_workspace(&path)?;
        let content = tokio::fs::read_to_string(&path).await?;
        let count = content.matches(old_s).count();
        if count == 0 {
            return err_result(&request.call_id, "old_string not found");
        }
        if count > 1 {
            return err_result(&request.call_id, "old_string not unique");
        }
        let updated = content.replacen(old_s, new_s, 1);
        tokio::fs::write(&path, &updated).await?;
        ok_result(
            &request.call_id,
            serde_json::json!({ "path": rel, "replacements": 1 }),
        )
    }
}

#[async_trait]
impl ToolHandler for RunShellTool {
    fn definition(&self) -> ToolDefinition {
        tool_def("run_shell", "Run a shell command in the project root", serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string" },
                "cwd": { "type": "string" }
            },
            "required": ["command"]
        }))
    }

    async fn execute(&self, request: ToolCallRequest) -> Result<ToolCallResult> {
        let ctx = ctx_from_request(&request)?;
        if !ctx.auto_approve && !request.approved {
            return pending(&request.call_id);
        }
        let cmd = request.arguments["command"]
            .as_str()
            .ok_or_else(|| crate::NexusError::Other(anyhow::anyhow!("missing command")))?;
        super::sandbox::validate_shell_command(cmd)?;
        let cwd = request
            .arguments
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(|s| ctx.resolve(s))
            .unwrap_or_else(|| ctx.project.root.clone());
        let mode = super::sandbox_exec::effective_sandbox_mode(&ctx.sandbox_mode);
        let output = super::sandbox_exec::run_shell(cmd, &cwd, mode).await?;
        ok_result(
            &request.call_id,
            serde_json::json!({
                "exit_code": output.status.code(),
                "stdout": String::from_utf8_lossy(&output.stdout),
                "stderr": String::from_utf8_lossy(&output.stderr),
            }),
        )
    }
}

#[async_trait]
impl ToolHandler for GitStatusTool {
    fn definition(&self) -> ToolDefinition {
        tool_def("git_status", "Show git status porcelain", serde_json::json!({
            "type": "object",
            "properties": {}
        }))
    }

    async fn execute(&self, request: ToolCallRequest) -> Result<ToolCallResult> {
        let ctx = ctx_from_request(&request)?;
        let out = run_git(&ctx.project.root, &["status", "--porcelain"]).await?;
        ok_result(&request.call_id, serde_json::json!({ "output": out }))
    }
}

#[async_trait]
impl ToolHandler for GitDiffTool {
    fn definition(&self) -> ToolDefinition {
        tool_def("git_diff", "Show git diff", serde_json::json!({
            "type": "object",
            "properties": {
                "staged": { "type": "boolean", "default": false }
            }
        }))
    }

    async fn execute(&self, request: ToolCallRequest) -> Result<ToolCallResult> {
        let ctx = ctx_from_request(&request)?;
        let staged = request.arguments.get("staged").and_then(|v| v.as_bool()).unwrap_or(false);
        let args: Vec<&str> = if staged {
            vec!["diff", "--staged"]
        } else {
            vec!["diff"]
        };
        let out = run_git(&ctx.project.root, &args).await?;
        ok_result(&request.call_id, serde_json::json!({ "output": out }))
    }
}

#[async_trait]
impl ToolHandler for GitBranchTool {
    fn definition(&self) -> ToolDefinition {
        tool_def(
            "git_branch",
            "Show current branch or list/create branches",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "create this branch" },
                    "list_all": { "type": "boolean" }
                }
            }),
        )
    }

    async fn execute(&self, request: ToolCallRequest) -> Result<ToolCallResult> {
        let ctx = ctx_from_request(&request)?;
        if let Some(name) = request.arguments.get("name").and_then(|v| v.as_str()) {
            let out = run_git(&ctx.project.root, &["branch", name]).await?;
            return ok_result(&request.call_id, serde_json::json!({ "output": out, "created": name }));
        }
        let list_all = request
            .arguments
            .get("list_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let args: Vec<&str> = if list_all {
            vec!["branch", "-a"]
        } else {
            vec!["branch", "--show-current"]
        };
        let out = run_git(&ctx.project.root, &args).await?;
        ok_result(&request.call_id, serde_json::json!({ "output": out }))
    }
}

#[async_trait]
impl ToolHandler for GitCommitTool {
    fn definition(&self) -> ToolDefinition {
        tool_def(
            "git_commit",
            "Create a git commit with a message",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" },
                    "all": { "type": "boolean" }
                },
                "required": ["message"]
            }),
        )
    }

    async fn execute(&self, request: ToolCallRequest) -> Result<ToolCallResult> {
        let ctx = ctx_from_request(&request)?;
        let msg = request
            .arguments
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("nexus commit");
        let all = request.arguments.get("all").and_then(|v| v.as_bool()).unwrap_or(false);
        let mut args = vec!["commit", "-m", msg];
        if all {
            args.insert(1, "-a");
        }
        if !request.approved {
            return pending(&request.call_id);
        }
        let out = run_git(&ctx.project.root, &args).await?;
        ok_result(
            &request.call_id,
            serde_json::json!({ "output": out, "message": msg }),
        )
    }
}

#[async_trait]
impl ToolHandler for GitLogTool {
    fn definition(&self) -> ToolDefinition {
        tool_def(
            "git_log",
            "Show recent git commits",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer" }
                }
            }),
        )
    }

    async fn execute(&self, request: ToolCallRequest) -> Result<ToolCallResult> {
        let ctx = ctx_from_request(&request)?;
        let limit = request
            .arguments
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(15) as usize;
        let n = limit.to_string();
        let out = run_git(
            &ctx.project.root,
            &["log", "--oneline", "-n", &n],
        )
        .await?;
        ok_result(&request.call_id, serde_json::json!({ "output": out }))
    }
}

#[async_trait]
impl ToolHandler for GitPushTool {
    fn definition(&self) -> ToolDefinition {
        tool_def(
            "git_push",
            "Push commits to remote (requires approval)",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "remote": { "type": "string" },
                    "branch": { "type": "string" },
                    "set_upstream": { "type": "boolean" }
                }
            }),
        )
    }

    async fn execute(&self, request: ToolCallRequest) -> Result<ToolCallResult> {
        let ctx = ctx_from_request(&request)?;
        if !request.approved {
            return pending(&request.call_id);
        }
        let remote = request
            .arguments
            .get("remote")
            .and_then(|v| v.as_str())
            .unwrap_or("origin");
        let branch = request.arguments.get("branch").and_then(|v| v.as_str());
        let set_upstream = request
            .arguments
            .get("set_upstream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let mut args = vec!["push", remote];
        if set_upstream {
            if let Some(b) = branch {
                args.push("-u");
                args.push(b);
            } else {
                args.push("-u");
            }
        } else if let Some(b) = branch {
            args.push(b);
        }
        let out = run_git(&ctx.project.root, &args).await?;
        ok_result(
            &request.call_id,
            serde_json::json!({ "output": out, "remote": remote }),
        )
    }
}

#[async_trait]
impl ToolHandler for SemanticSearchTool {
    fn definition(&self) -> ToolDefinition {
        tool_def(
            "semantic_search",
            "Semantic search over indexed workspace (ChromaDB via engine)",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer" }
                },
                "required": ["query"]
            }),
        )
    }

    async fn execute(&self, request: ToolCallRequest) -> Result<ToolCallResult> {
        let ctx = ctx_from_request(&request)?;
        let query = request
            .arguments
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if query.is_empty() {
            return err_result(&request.call_id, "missing query");
        }
        let limit = request
            .arguments
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;
        let url = format!(
            "{}/v1/vector/search",
            ctx.engine_url.trim_end_matches('/')
        );
        let root = ctx.project.root.to_string_lossy();
        let res = reqwest::Client::new()
            .get(&url)
            .query(&[("q", query), ("workspace_root", root.as_ref()), ("k", &limit.to_string())])
            .send()
            .await
            .map_err(|e| crate::NexusError::Engine(e.to_string()))?;
        if !res.status().is_success() {
            return err_result(
                &request.call_id,
                &format!("semantic search HTTP {}", res.status()),
            );
        }
        let body: serde_json::Value = res.json().await.map_err(|e| crate::NexusError::Engine(e.to_string()))?;
        ok_result(&request.call_id, body)
    }
}

#[async_trait]
impl ToolHandler for GlobFilesTool {
    fn definition(&self) -> ToolDefinition {
        tool_def(
            "glob_files",
            "Find files by glob pattern (e.g. **/*.md, docs/*.md)",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "glob relative to project root" },
                    "max_results": { "type": "integer" }
                },
                "required": ["pattern"]
            }),
        )
    }

    async fn execute(&self, request: ToolCallRequest) -> Result<ToolCallResult> {
        let ctx = ctx_from_request(&request)?;
        let pattern = request
            .arguments
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("**/*.md");
        let max = request
            .arguments
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;
        let root = &ctx.project.root;
        let mut matches = Vec::new();
        for entry in walkdir::WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path.components().any(|c| {
                let s = c.as_os_str();
                s == ".git" || s == "node_modules" || s == "target" || s == ".venv"
            }) {
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            if simple_glob_match(pattern, &rel) {
                matches.push(rel);
                if matches.len() >= max {
                    break;
                }
            }
        }
        matches.sort();
        ok_result(
            &request.call_id,
            serde_json::json!({ "pattern": pattern, "files": matches, "count": matches.len() }),
        )
    }
}

fn simple_glob_match(pattern: &str, path: &str) -> bool {
    let pattern = pattern.replace('\\', "/");
    if pattern == "**/*" || pattern == "*" {
        return true;
    }
    if let Some(ext) = pattern.strip_prefix("**/*.") {
        return path.ends_with(&format!(".{ext}")) || path.contains(&format!("/.{ext}"));
    }
    if let Some(ext) = pattern.strip_prefix("*.") {
        return path.ends_with(&format!(".{ext}")) && !path[..path.len().saturating_sub(ext.len() + 1)].contains('/');
    }
    if pattern.contains('*') {
        let parts: Vec<_> = pattern.split('*').collect();
        if parts.len() == 2 {
            return path.starts_with(parts[0]) && path.ends_with(parts[1]);
        }
    }
    path == pattern || path.ends_with(&pattern)
}

async fn run_git(cwd: &std::path::Path, args: &[&str]) -> Result<String> {
    let output = tokio::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .await?;
    Ok(format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn tool_def(name: &str, desc: &str, schema: serde_json::Value) -> ToolDefinition {
    ToolDefinition {
        id: Uuid::nil(),
        name: name.into(),
        description: desc.into(),
        input_schema: schema,
        source: "builtin".into(),
        enabled: true,
    }
}

fn ctx_from_request(request: &ToolCallRequest) -> Result<Arc<WorkspaceToolContext>> {
    request.workspace.clone().ok_or_else(|| {
        crate::NexusError::Other(anyhow::anyhow!("missing workspace context"))
    })
}

// Workspace context is injected by executor, not LLM — use separate field on ToolCallRequest

fn ok_result(call_id: &str, output: serde_json::Value) -> Result<ToolCallResult> {
    Ok(ToolCallResult {
        call_id: call_id.into(),
        status: ToolResultStatus::Ok,
        output: Some(output),
        error: None,
    })
}

fn err_result(call_id: &str, msg: &str) -> Result<ToolCallResult> {
    Ok(ToolCallResult {
        call_id: call_id.into(),
        status: ToolResultStatus::Error,
        output: None,
        error: Some(msg.into()),
    })
}

fn pending(call_id: &str) -> Result<ToolCallResult> {
    Ok(ToolCallResult {
        call_id: call_id.into(),
        status: ToolResultStatus::PendingApproval,
        output: None,
        error: None,
    })
}

async fn read_capped(path: PathBuf, max_bytes: usize) -> Result<String> {
    let meta = tokio::fs::metadata(&path).await?;
    if meta.len() as usize > max_bytes {
        let bytes = tokio::fs::read(&path).await?;
        let truncated = String::from_utf8_lossy(&bytes[..max_bytes]).into_owned();
        return Ok(format!("{truncated}\n... [truncated]"));
    }
    tokio::fs::read_to_string(&path).await.map_err(Into::into)
}

pub fn workspace_registry(ctx: Arc<WorkspaceToolContext>) -> super::ToolRegistry {
    let policies = vec![
        PermissionPolicy {
            id: Uuid::new_v4(),
            workspace_id: None,
            tool_name: Some("read_file".into()),
            resource_pattern: None,
            action: PermissionAction::Allow,
            priority: 10,
        },
        PermissionPolicy {
            id: Uuid::new_v4(),
            workspace_id: None,
            tool_name: Some("git_status".into()),
            resource_pattern: None,
            action: PermissionAction::Allow,
            priority: 10,
        },
        PermissionPolicy {
            id: Uuid::new_v4(),
            workspace_id: None,
            tool_name: Some("git_diff".into()),
            resource_pattern: None,
            action: PermissionAction::Allow,
            priority: 10,
        },
        PermissionPolicy {
            id: Uuid::new_v4(),
            workspace_id: None,
            tool_name: Some("git_branch".into()),
            resource_pattern: None,
            action: PermissionAction::Allow,
            priority: 10,
        },
        PermissionPolicy {
            id: Uuid::new_v4(),
            workspace_id: None,
            tool_name: Some("git_log".into()),
            resource_pattern: None,
            action: PermissionAction::Allow,
            priority: 10,
        },
        PermissionPolicy {
            id: Uuid::new_v4(),
            workspace_id: None,
            tool_name: Some("git_commit".into()),
            resource_pattern: None,
            action: PermissionAction::Ask,
            priority: 5,
        },
        PermissionPolicy {
            id: Uuid::new_v4(),
            workspace_id: None,
            tool_name: Some("git_push".into()),
            resource_pattern: None,
            action: PermissionAction::Ask,
            priority: 5,
        },
        PermissionPolicy {
            id: Uuid::new_v4(),
            workspace_id: None,
            tool_name: Some("glob_files".into()),
            resource_pattern: None,
            action: PermissionAction::Allow,
            priority: 10,
        },
        PermissionPolicy {
            id: Uuid::new_v4(),
            workspace_id: None,
            tool_name: Some("semantic_search".into()),
            resource_pattern: None,
            action: PermissionAction::Allow,
            priority: 10,
        },
        PermissionPolicy {
            id: Uuid::new_v4(),
            workspace_id: None,
            tool_name: Some("write_file".into()),
            resource_pattern: None,
            action: PermissionAction::Ask,
            priority: 5,
        },
        PermissionPolicy {
            id: Uuid::new_v4(),
            workspace_id: None,
            tool_name: Some("edit_file".into()),
            resource_pattern: None,
            action: PermissionAction::Ask,
            priority: 5,
        },
        PermissionPolicy {
            id: Uuid::new_v4(),
            workspace_id: None,
            tool_name: Some("run_shell".into()),
            resource_pattern: None,
            action: PermissionAction::Ask,
            priority: 5,
        },
    ];
    let mut registry = super::ToolRegistry::new(policies);
    registry.set_workspace(ctx);
    registry.register(Arc::new(ReadFileTool));
    registry.register(Arc::new(WriteFileTool));
    registry.register(Arc::new(EditFileTool));
    registry.register(Arc::new(RunShellTool));
    registry.register(Arc::new(GitStatusTool));
    registry.register(Arc::new(GitDiffTool));
    registry.register(Arc::new(GitBranchTool));
    registry.register(Arc::new(GitCommitTool));
    registry.register(Arc::new(GitLogTool));
    registry.register(Arc::new(GitPushTool));
    registry.register(Arc::new(SemanticSearchTool));
    registry.register(Arc::new(GlobFilesTool));
    registry
}
