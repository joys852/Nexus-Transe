/**
 * Shared protocol types for CLI ↔ Engine HTTP API.
 * Keep in sync with packages/shared/proto and Rust models.
 */

export type SessionStatus = 'active' | 'paused' | 'completed' | 'failed';

export type MessageRole = 'user' | 'assistant' | 'system' | 'tool';

export type TaskRunStatus =
  | 'idle'
  | 'planning'
  | 'acting'
  | 'observing'
  | 'paused'
  | 'done'
  | 'failed';

export type PermissionAction = 'allow' | 'deny' | 'ask';

export type ToolResultStatus = 'ok' | 'denied' | 'error' | 'pending_approval';

export interface Session {
  id: string;
  workspaceId?: string;
  title?: string;
  status: SessionStatus;
  revision: number;
  modelId?: string;
  agentProfile: string;
  createdAt: string;
  updatedAt: string;
}

export interface Message {
  id: string;
  sessionId: string;
  role: MessageRole;
  content: string;
  metadata?: Record<string, unknown>;
  parentId?: string;
  sequence: number;
  createdAt: string;
}

export interface ToolDefinition {
  id: string;
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
  source: 'builtin' | 'mcp' | 'plugin';
  enabled: boolean;
}

export interface ToolCallRequest {
  sessionId: string;
  toolName: string;
  arguments: Record<string, unknown>;
  callId: string;
}

export interface ToolCallResult {
  callId: string;
  status: ToolResultStatus;
  output?: unknown;
  error?: string;
}

export type SyncEvent =
  | { type: 'session_updated'; sessionId: string; revision: number }
  | { type: 'message_appended'; sessionId: string; messageId: string; sequence: number }
  | { type: 'task_status_changed'; sessionId: string; status: TaskRunStatus }
  | {
      type: 'tool_approval_required';
      sessionId: string;
      callId: string;
      toolName: string;
    }
  | { type: 'stream_delta'; sessionId: string; delta: string };

/** Engine HTTP API */
export interface RunTaskRequest {
  sessionId: string;
  prompt: string;
  modelId?: string;
  agentProfile?: 'default' | 'architect' | 'code' | 'review' | 'test';
}

export interface RunTaskResponse {
  taskId: string;
  status: TaskRunStatus;
}

export interface EngineHealthResponse {
  ok: boolean;
  version: string;
}
