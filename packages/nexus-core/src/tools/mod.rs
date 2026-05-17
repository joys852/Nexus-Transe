use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::{PermissionAction, PermissionPolicy, ToolCallRequest, ToolCallResult, ToolDefinition};
use crate::Result;

pub mod builtin;
pub mod executor;
pub mod sandbox;
pub mod sandbox_exec;
pub use crate::project::WorkspaceToolContext;
pub use builtin::{workspace_registry, ReadFileTool};
pub use executor::ToolExecutor;

/// Dynamic tool registry with permission checks.
#[async_trait]
pub trait ToolHandler: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    async fn execute(&self, request: ToolCallRequest) -> Result<ToolCallResult>;
}

pub struct ToolRegistry {
    handlers: HashMap<String, Arc<dyn ToolHandler>>,
    policies: Vec<PermissionPolicy>,
    workspace: Option<Arc<WorkspaceToolContext>>,
}

impl ToolRegistry {
    pub fn new(policies: Vec<PermissionPolicy>) -> Self {
        Self {
            handlers: HashMap::new(),
            policies,
            workspace: None,
        }
    }

    pub fn set_workspace(&mut self, ctx: Arc<WorkspaceToolContext>) {
        self.workspace = Some(ctx);
    }

    pub fn register(&mut self, handler: Arc<dyn ToolHandler>) {
        let name = handler.definition().name.clone();
        self.handlers.insert(name, handler);
    }

    pub fn list(&self) -> Vec<ToolDefinition> {
        self.handlers
            .values()
            .map(|h| h.definition())
            .collect()
    }

    pub async fn invoke(&self, mut request: ToolCallRequest) -> Result<ToolCallResult> {
        request.workspace = self.workspace.clone();

        if !request.approved {
            let action = self.resolve_policy(&request);
            match action {
                PermissionAction::Deny => {
                    return Ok(ToolCallResult {
                        call_id: request.call_id,
                        status: crate::models::ToolResultStatus::Denied,
                        output: None,
                        error: Some("denied by policy".into()),
                    });
                }
                PermissionAction::Ask => {
                    return Ok(ToolCallResult {
                        call_id: request.call_id,
                        status: crate::models::ToolResultStatus::PendingApproval,
                        output: None,
                        error: None,
                    });
                }
                PermissionAction::Allow => {}
            }
        }

        let handler = self.handlers.get(&request.tool_name).ok_or_else(|| {
            crate::NexusError::Other(anyhow::anyhow!("unknown tool: {}", request.tool_name))
        })?;
        sandbox::run_in_sandbox(handler.as_ref(), request).await
    }

    fn resolve_policy(&self, request: &ToolCallRequest) -> PermissionAction {
        let mut best: Option<&PermissionPolicy> = None;
        for p in &self.policies {
            if let Some(ref name) = p.tool_name {
                if name != &request.tool_name {
                    continue;
                }
            }
            if best.map(|b| p.priority > b.priority).unwrap_or(true) {
                best = Some(p);
            }
        }
        best.map(|p| p.action.clone())
            .unwrap_or(PermissionAction::Ask)
    }
}
