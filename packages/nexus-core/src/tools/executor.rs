use crate::models::{ToolCallRequest, ToolCallResult};
use crate::Result;

use super::ToolRegistry;

/// Runs tool calls through a [`ToolRegistry`].
pub struct ToolExecutor {
    registry: ToolRegistry,
}

impl ToolExecutor {
    pub fn new(registry: ToolRegistry) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut ToolRegistry {
        &mut self.registry
    }

    pub async fn execute(&self, request: ToolCallRequest) -> Result<ToolCallResult> {
        self.registry.invoke(request).await
    }

    pub async fn execute_batch(
        &self,
        requests: Vec<ToolCallRequest>,
    ) -> Result<Vec<ToolCallResult>> {
        let mut results = Vec::with_capacity(requests.len());
        for request in requests {
            results.push(self.registry.invoke(request).await?);
        }
        Ok(results)
    }
}
