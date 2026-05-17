use nexus_core::models::{PermissionAction, PermissionPolicy, ToolCallRequest, ToolResultStatus};
use nexus_core::project::{ProjectContext, WorkspaceToolContext};
use nexus_core::tools::{workspace_registry, ReadFileTool, ToolRegistry};
use std::sync::Arc;
use tempfile::tempdir;
use uuid::Uuid;

#[tokio::test]
async fn read_file_tool_reads_content() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("hello.txt");
    std::fs::write(&file, "world").unwrap();

    let project = ProjectContext {
        root: dir.path().to_path_buf(),
        project_md: None,
        instructions: nexus_core::project::instructions::InstructionBundle::default(),
        name: Some("test".into()),
    };
    let ws = Arc::new(WorkspaceToolContext {
        project: Arc::new(project),
        auto_approve: true,
        engine_url: "http://127.0.0.1:8765".into(),
        sandbox_mode: "local".into(),
    });

    let mut registry = ToolRegistry::new(vec![PermissionPolicy {
        id: Uuid::new_v4(),
        workspace_id: None,
        tool_name: Some("read_file".into()),
        resource_pattern: None,
        action: PermissionAction::Allow,
        priority: 10,
    }]);
    registry.set_workspace(ws);
    registry.register(Arc::new(ReadFileTool));

    let result = registry
        .invoke(ToolCallRequest {
            session_id: Uuid::new_v4(),
            tool_name: "read_file".into(),
            arguments: serde_json::json!({ "path": "hello.txt" }),
            call_id: "c1".into(),
            approved: false,
            workspace: None,
        })
        .await
        .unwrap();

    assert!(matches!(result.status, ToolResultStatus::Ok));
    assert_eq!(result.output.unwrap()["content"], "world");
}
