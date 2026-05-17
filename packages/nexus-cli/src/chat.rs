use colored::Colorize;
use crate::approval::ApprovalMode;
use crate::menu::{confirm_tool_run, ToolApprovalChoice};
use crate::mode::ChatMode;
use crate::session_ui::TurnTimer;
use crate::ui;
use nexus_core::engine::read_sse_stream;
use nexus_core::models::{MessageRole, ToolCallRequest, ToolResultStatus};
use nexus_core::storage::SessionRepository;
use nexus_core::tools::ToolRegistry;
use reqwest::Client;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;

pub struct ChatRunner<'a> {
    pub http: &'a Client,
    pub engine_url: &'a str,
    pub session_id: Uuid,
    pub model: &'a str,
    pub project_md: Option<&'a str>,
    pub workspace_root: &'a str,
    pub skills_context: Option<&'a str>,
    pub mode: ChatMode,
    pub tools: &'a ToolRegistry,
    pub store: &'a nexus_core::storage::sqlite::SqliteStore,
    pub cancelled: Arc<AtomicBool>,
    pub session_approve_all: Arc<AtomicBool>,
    pub approval_mode: ApprovalMode,
}

struct StreamOutcome {
    assistant: String,
    tool_calls: Vec<serde_json::Value>,
    done_status: String,
    reply_started: bool,
}

impl<'a> ChatRunner<'a> {
    pub async fn send_and_stream(&self, user_msg: &str) -> anyhow::Result<String> {
        let label = self.turn_label();
        let timer = TurnTimer::start(&label);
        let result = if self.mode == ChatMode::Agent {
            self.send_agent_pipeline(user_msg).await
        } else {
            self.send_chat_turn(user_msg).await
        };
        if self.cancelled.load(Ordering::SeqCst) {
            timer.finish_cancelled();
        } else if result.is_ok() {
            timer.finish();
        } else {
            timer.finish_cancelled();
        }
        result
    }

    fn turn_label(&self) -> String {
        match self.mode {
            ChatMode::Default => "chat".into(),
            ChatMode::Plan => "plan".into(),
            ChatMode::Agent => "agent pipeline".into(),
        }
    }

    async fn send_chat_turn(&self, user_msg: &str) -> anyhow::Result<String> {
        self.store
            .append_message(self.session_id, MessageRole::User, user_msg, None)
            .await?;

        let mut assistant = String::new();
        let mut body = ChatBody::Message(user_msg.to_string());

        loop {
            if self.cancelled.load(Ordering::SeqCst) {
                ui::print_cancelled();
                break;
            }

            let response = match &body {
                ChatBody::Message(m) => self.post_chat(m).await?,
                ChatBody::ToolResults(r) => self.post_tool_results(r).await?,
            };

            let outcome = self.consume_stream(response).await?;
            assistant.push_str(&outcome.assistant);

            if self.mode == ChatMode::Plan {
                break;
            }

            if outcome.done_status == "awaiting_tools" && !outcome.tool_calls.is_empty() {
                crate::session_ui::set_activity("tools");
                let results = self.execute_tools(&outcome.tool_calls).await?;
                body = ChatBody::ToolResults(results);
                continue;
            }
            break;
        }

        if !assistant.is_empty() {
            self.store
                .append_message(
                    self.session_id,
                    MessageRole::Assistant,
                    &assistant,
                    None,
                )
                .await?;
        }
        Ok(assistant)
    }

    async fn send_agent_pipeline(&self, user_msg: &str) -> anyhow::Result<String> {
        self.store
            .append_message(self.session_id, MessageRole::User, user_msg, None)
            .await?;

        let url = format!(
            "{}/v1/sessions/{}/orchestrate",
            self.engine_url.trim_end_matches('/'),
            self.session_id
        );
        let res = self
            .http
            .post(url)
            .json(&serde_json::json!({
                "message": user_msg,
                "model_id": self.model,
                "project_md": self.project_md,
                "workspace_root": self.workspace_root,
                "mode": self.mode.as_str(),
                "skills_context": self.skills_context,
            }))
            .send()
            .await?;

        if !res.status().is_success() {
            anyhow::bail!("orchestrate HTTP {}", res.status());
        }

        let body: serde_json::Value = res.json().await?;
        let phases = body["phases"].as_array().cloned().unwrap_or_default();
        let mut combined = String::new();

        let stage_names: Vec<&str> = phases
            .iter()
            .map(|p| p["agent"].as_str().unwrap_or("agent"))
            .collect();
        let prog = crate::progress::StageProgress::new(
            if stage_names.is_empty() {
                &["pipeline"]
            } else {
                &stage_names
            },
        );

        for (i, phase) in phases.iter().enumerate() {
            let agent = phase["agent"].as_str().unwrap_or("agent");
            let content = phase["content"].as_str().unwrap_or("");
            prog.set_active(i, agent);
            ui::print_agent_phase(agent, "");
            if !content.is_empty() {
                println!("{}", crate::markdown::render(content));
            }
            prog.complete(i, &format!("{agent} done"));
            combined.push_str(&format!("## {agent}\n{content}\n\n"));
        }
        println!();

        if combined.is_empty() {
            ui::print_info("pipeline returned no phases");
        } else {
            self.store
                .append_message(
                    self.session_id,
                    MessageRole::Assistant,
                    &combined,
                    None,
                )
                .await?;
        }
        Ok(combined)
    }

    async fn consume_stream(&self, response: reqwest::Response) -> anyhow::Result<StreamOutcome> {
        let mut assistant = String::new();
        let mut tool_calls = Vec::new();
        let mut done_status = String::from("completed");
        let cancelled = self.cancelled.clone();
        let mut reply_started = false;

        read_sse_stream(response, |ev| {
            if cancelled.load(Ordering::SeqCst) {
                return Ok(());
            }
            match ev.event.as_str() {
                "token" => {
                    if let Some(d) = ev.data.get("delta").and_then(|v| v.as_str()) {
                        if !reply_started {
                            ui::begin_assistant();
                            reply_started = true;
                        }
                        ui::stream_assistant_delta(d);
                        assistant.push_str(d);
                    }
                }
                "tool_call" => tool_calls.push(ev.data),
                "status" | "thinking" => {
                    if let Some(phase) = ev.data.get("phase").and_then(|v| v.as_str()) {
                        let label = match phase {
                            "thinking" => "thinking",
                            "continuing" => "continuing",
                            other => other,
                        };
                        crate::session_ui::set_activity(label);
                    }
                }
                "progress" => {
                    if let (Some(stage), Some(pct)) = (
                        ev.data.get("stage").and_then(|v| v.as_str()),
                        ev.data.get("percent").and_then(|v| v.as_u64()),
                    ) {
                        crate::session_ui::set_activity(&format!("{stage} {pct}%"));
                    }
                }
                "error" => {
                    if let Some(m) = ev.data.get("message").and_then(|v| v.as_str()) {
                        println!();
                        ui::print_error(m);
                    }
                }
                "done" => {
                    if let Some(s) = ev.data.get("status").and_then(|v| v.as_str()) {
                        done_status = s.to_string();
                    }
                    if let Some(c) = ev.data.get("content").and_then(|v| v.as_str()) {
                        if !c.is_empty() && assistant.is_empty() && !reply_started {
                            ui::begin_assistant();
                            reply_started = true;
                            ui::stream_assistant_delta(c);
                            assistant.push_str(c);
                        }
                    }
                }
                _ => {}
            }
            Ok(())
        })
        .await?;

        if reply_started {
            ui::end_assistant_stream();
        }

        Ok(StreamOutcome {
            assistant,
            tool_calls,
            done_status,
            reply_started,
        })
    }

    async fn invoke_mcp_tool(
        &self,
        server: &str,
        tool: &str,
        arguments: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let url = format!(
            "{}/v1/mcp/call",
            self.engine_url.trim_end_matches('/')
        );
        let res = self
            .http
            .post(&url)
            .json(&serde_json::json!({
                "server": server,
                "tool": tool,
                "arguments": arguments,
                "workspace_root": self.workspace_root,
            }))
            .send()
            .await?;
        let body: serde_json::Value = res.json().await?;
        if body.get("ok").and_then(|v| v.as_bool()) == Some(true) {
            Ok(serde_json::json!({ "output": body.get("output") }))
        } else {
            Ok(serde_json::json!({
                "error": body.get("error").and_then(|e| e.as_str()).unwrap_or("mcp call failed")
            }))
        }
    }

    async fn post_chat(&self, message: &str) -> anyhow::Result<reqwest::Response> {
        let url = format!(
            "{}/v1/sessions/{}/chat",
            self.engine_url.trim_end_matches('/'),
            self.session_id
        );
        let res = self
            .http
            .post(url)
            .json(&serde_json::json!({
                "message": message,
                "model_id": self.model,
                "project_md": self.project_md,
                "workspace_root": self.workspace_root,
                "mode": self.mode.as_str(),
                "skills_context": self.skills_context,
            }))
            .send()
            .await?;
        if !res.status().is_success() {
            anyhow::bail!("chat HTTP {}", res.status());
        }
        Ok(res)
    }

    async fn post_tool_results(
        &self,
        results: &[serde_json::Value],
    ) -> anyhow::Result<reqwest::Response> {
        let url = format!(
            "{}/v1/sessions/{}/tool-results",
            self.engine_url.trim_end_matches('/'),
            self.session_id
        );
        let items: Vec<_> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "call_id": r.get("call_id"),
                    "tool_name": r.get("tool_name"),
                    "output": r.get("output"),
                    "error": r.get("error"),
                })
            })
            .collect();
        let res = self
            .http
            .post(url)
            .json(&serde_json::json!({ "results": items }))
            .send()
            .await?;
        if !res.status().is_success() {
            anyhow::bail!("tool-results HTTP {}", res.status());
        }
        Ok(res)
    }

    pub(crate) async fn execute_tools(
        &self,
        calls: &[serde_json::Value],
    ) -> anyhow::Result<Vec<serde_json::Value>> {
        let mut results = Vec::new();
        for c in calls {
            let call_id = c["call_id"].as_str().unwrap_or("").to_string();
            let tool_name = c["tool_name"].as_str().unwrap_or("").to_string();
            let arguments = c.get("arguments").cloned().unwrap_or_default();
            let args_display = ui::format_tool_args(&arguments);

            ui::print_tool_begin(&tool_name, &args_display);

            let hook = crate::hooks::pre_tool_use(
                std::path::Path::new(self.workspace_root),
                &tool_name,
                &arguments,
            );
            if let crate::hooks::HookOutcome::Deny(msg) = &hook {
                ui::print_tool_done_fail(msg);
                results.push(serde_json::json!({
                    "call_id": call_id,
                    "tool_name": tool_name,
                    "error": msg,
                }));
                continue;
            }

            if let (Some(server), Some(mcp_tool)) = (
                c.get("mcp_server").and_then(|v| v.as_str()),
                c.get("mcp_tool").and_then(|v| v.as_str()),
            ) {
                let mcp_result = self.invoke_mcp_tool(server, mcp_tool, &arguments).await?;
                if mcp_result.get("error").is_some() {
                    ui::print_tool_done_fail(
                        mcp_result
                            .get("error")
                            .and_then(|e| e.as_str())
                            .unwrap_or("mcp error"),
                    );
                } else {
                    ui::print_tool_done_ok();
                    if let Some(out) = mcp_result.get("output") {
                        let preview =
                            crate::tool_format::format_tool_output(&tool_name, out);
                        if !preview.is_empty() {
                            println!("{}", preview.dimmed());
                        }
                    }
                }
                results.push(serde_json::json!({
                    "call_id": call_id,
                    "tool_name": tool_name,
                    "output": mcp_result.get("output").cloned(),
                    "error": mcp_result.get("error").and_then(|e| e.as_str()),
                }));
                continue;
            }

            let mut req = ToolCallRequest {
                session_id: self.session_id,
                tool_name: tool_name.clone(),
                arguments,
                call_id: call_id.clone(),
                approved: false,
                workspace: None,
            };
            let mut result = self.tools.invoke(req.clone()).await?;

            let hook_auto = matches!(hook, crate::hooks::HookOutcome::Allow);
            let hook_force_prompt = matches!(hook, crate::hooks::HookOutcome::RequireApproval);
            let mode_auto = self.approval_mode.auto_approve_pending_tool(&tool_name);
            if matches!(result.status, ToolResultStatus::PendingApproval)
                && !self.session_approve_all.load(Ordering::SeqCst)
                && !hook_auto
                && (hook_force_prompt || (!mode_auto))
            {
                match confirm_tool_run(&tool_name, &args_display)? {
                    ToolApprovalChoice::RunOnce => {
                        req.approved = true;
                        result = self.tools.invoke(req).await?;
                    }
                    ToolApprovalChoice::RunSession => {
                        self.session_approve_all.store(true, Ordering::SeqCst);
                        ui::print_info("auto-approve enabled for this session");
                        req.approved = true;
                        result = self.tools.invoke(req).await?;
                    }
                    ToolApprovalChoice::Deny => {
                        result.status = ToolResultStatus::Denied;
                        result.error = Some("user denied".into());
                    }
                }
            } else if matches!(result.status, ToolResultStatus::PendingApproval) {
                req.approved = true;
                result = self.tools.invoke(req).await?;
            }

            if matches!(result.status, ToolResultStatus::Ok) {
                ui::print_tool_done_ok();
                if let Some(out) = &result.output {
                    let preview = crate::tool_format::format_tool_output(&tool_name, out);
                    if !preview.is_empty() {
                        println!("{}", preview.dimmed());
                    }
                }
            } else {
                ui::print_tool_done_fail(&format!("{:?}", result.status));
            }

            results.push(serde_json::json!({
                "call_id": result.call_id,
                "tool_name": tool_name,
                "output": result.output,
                "error": result.error,
            }));
        }
        Ok(results)
    }

    pub async fn hydrate_engine_session(&self) -> anyhow::Result<()> {
        let messages = self.store.list_messages(self.session_id, 500).await?;
        if messages.is_empty() {
            return Ok(());
        }
        let mut ordered = messages;
        ordered.sort_by_key(|m| m.sequence);
        let payload: Vec<_> = ordered
            .iter()
            .filter_map(|m| {
                let role = match m.role {
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::System => "system",
                    MessageRole::Tool => return None,
                };
                Some(serde_json::json!({
                    "role": role,
                    "content": m.content,
                }))
            })
            .collect();
        let url = format!(
            "{}/v1/sessions/{}/hydrate",
            self.engine_url.trim_end_matches('/'),
            self.session_id
        );
        let res = self
            .http
            .post(url)
            .json(&serde_json::json!({ "messages": payload }))
            .send()
            .await?;
        if !res.status().is_success() {
            anyhow::bail!("hydrate HTTP {}", res.status());
        }
        Ok(())
    }
}

enum ChatBody {
    Message(String),
    ToolResults(Vec<serde_json::Value>),
}
