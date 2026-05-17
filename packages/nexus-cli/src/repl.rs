use crate::approval::ApprovalMode;
use crate::chat::ChatRunner;
use crate::completer::NexusHelper;
use crate::input;
use crate::mode::ChatMode;
use crate::session_pin::SessionPin;
use crate::skills;
use crate::skills_ext;
use crate::slash;
use crate::ui;
use colored::Colorize;
use nexus_core::models::MessageRole;
use nexus_core::config::NexusConfig;
use nexus_core::project::ProjectContext;
use nexus_core::providers::ProvidersStore;
use nexus_core::storage::sqlite::SqliteStore;
use nexus_core::storage::SessionRepository;
use nexus_core::tools::{workspace_registry, WorkspaceToolContext};
use reqwest::Client;
use rustyline::config::{ColorMode, CompletionType};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct ReplSession {
    session_id: uuid::Uuid,
    store: SqliteStore,
    config: NexusConfig,
    project: ProjectContext,
    workspace_root: String,
    model: String,
    provider_name: Option<String>,
    tools: nexus_core::tools::ToolRegistry,
    http: Client,
    cancelled: Arc<AtomicBool>,
    mode: ChatMode,
    skills_context: String,
    session_approve_all: Arc<AtomicBool>,
    approval_mode: ApprovalMode,
}

impl ReplSession {
    pub async fn new(
        store: SqliteStore,
        config: NexusConfig,
        cwd: PathBuf,
    ) -> anyhow::Result<Self> {
        let project = ProjectContext::detect(&cwd)?
            .ok_or_else(|| anyhow::anyhow!("no project root (.git / Cargo.toml / PROJECT.md)"))?;
        let ws = WorkspaceToolContext {
            project: Arc::new(project.clone()),
            auto_approve: false,
            engine_url: config.engine_url.clone(),
            sandbox_mode: config.sandbox_mode.clone(),
        };
        let tools = workspace_registry(Arc::new(ws));
        let session_id = if let Some(pin) = SessionPin::load(&project.root) {
            if store.get_session(pin.session_id).await?.is_some() {
                pin.session_id
            } else {
                store.create_session(None, Some("REPL")).await?.id
            }
        } else {
            store.create_session(None, Some("REPL")).await?.id
        };
        SessionPin::save(&project.root, session_id, Some("REPL"))?;
        let workspace_root = project.root.to_string_lossy().into_owned();

        let providers = ProvidersStore::new(&config.data_dir).load().ok();
        let (provider_name, model) = providers
            .as_ref()
            .and_then(|p| {
                p.active_provider().map(|prof| {
                    (Some(prof.name.clone()), prof.model.clone())
                })
            })
            .unwrap_or((None, config.default_model.clone()));

        let skills_context =
            skills::build_skills_context(&config.data_dir, &project.root).unwrap_or_default();

        Ok(Self {
            session_id,
            store,
            config,
            project,
            workspace_root,
            model,
            provider_name,
            tools,
            http: Client::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
            mode: ChatMode::Default,
            skills_context,
            session_approve_all: Arc::new(AtomicBool::new(false)),
            approval_mode: ApprovalMode::Suggest,
        })
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        crate::engine_cmd::ensure_engine(&self.config.engine_url, true).await?;

        if !crate::trust::prompt_trust_workspace(&self.config.data_dir, &self.project.root)? {
            anyhow::bail!("workspace not trusted — exiting");
        }

        crate::collab::acquire_lock(&self.project.root, self.session_id, "REPL")?;

        let skill_count = skills::list_skills(&self.config.data_dir, &self.project.root)
            .map(|s| s.len())
            .unwrap_or(0);

        ui::print_welcome(ui::WelcomeInfo {
            version: env!("CARGO_PKG_VERSION"),
            cwd: &self.project.root,
            provider_name: self.provider_name.as_deref(),
            model: &self.model,
            mode: self.mode,
            skill_count,
            instruction_count: self.project.instructions.files.len(),
        });
        if std::env::var("NEXUS_VERBOSE_WELCOME").is_ok() {
            let peers = crate::collab::read_peers(&self.project.root);
            ui::print_collab_strip(&peers);
        }

        if let Ok(()) = self.runner().hydrate_engine_session().await {
            let n = self
                .store
                .list_messages(self.session_id, 500)
                .await
                .map(|m| m.len())
                .unwrap_or(0);
            if n > 0 {
                ui::print_resume_hint(&self.session_id, n);
            }
        }

        let mut helper = NexusHelper::new();
        helper.set_project_root(self.project.root.clone());
        let config = rustyline::Config::builder()
            .tab_stop(4)
            .completion_type(CompletionType::List)
            .color_mode(ColorMode::Forced)
            .build();
        let mut rl = Editor::with_config(config)?;
        rl.set_helper(Some(helper));
        input::bind_multiline_keys(&mut rl);
        rl.load_history(".nexus_history").ok();

        loop {
            self.cancelled.store(false, Ordering::SeqCst);
            if let Some(h) = rl.helper_mut() {
                h.set_mode(self.mode);
            }
            let prompt = ui::prompt(self.mode);
            match rl.readline(prompt) {
                Ok(line) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    let _ = rl.add_history_entry(line);

                    if line == "/" || line == "/?" {
                        crate::completer::print_slash_menu(line);
                        continue;
                    }

                    if let Some(cmd) = line.split_whitespace().next() {
                        if let Some(new_mode) = ChatMode::from_slash(cmd) {
                            self.mode = new_mode;
                            ui::print_mode_switch(self.mode);
                            continue;
                        }
                    }

                    match self.handle_slash_or_chat(line).await {
                        Ok(true) => break,
                        Ok(false) => {}
                        Err(e) => crate::errors::print_anyhow(&e),
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    self.cancelled.store(true, Ordering::SeqCst);
                    ui::print_info("^C — cancelled (use /exit to quit)");
                }
                Err(ReadlineError::Eof) => break,
                Err(e) => return Err(e.into()),
            }
        }
        rl.save_history(".nexus_history").ok();
        crate::collab::release_lock(&self.project.root);
        println!();
        Ok(())
    }

    /// Returns `true` when the REPL should exit.
    async fn handle_slash_or_chat(&mut self, line: &str) -> anyhow::Result<bool> {
        match line {
            "/exit" | "/quit" => return Ok(true),
            "/cancel" => {
                self.cancelled.store(true, Ordering::SeqCst);
                ui::print_cancelled();
            }
            "/help" => ui::print_help(),
            "/clear" => println!(),
            "/mode" => ui::print_info(format!(
                "mode: {} · model: {}",
                self.mode.label(),
                self.model
            )),
            "/init" => self.cmd_init()?,
            "/docs" => self.cmd_docs()?,
            "/index" => {
                let idx = nexus_core::project::ProjectIndexer::new(&self.project.root)?;
                let n = idx.walk(&self.project.root)?.len();
                ui::print_success(format!("{n} files indexed"));
            }
            "/git-status" => {
                self.runner()
                    .execute_tools(&[serde_json::json!({
                        "call_id": "1", "tool_name": "git_status", "arguments": {}
                    })])
                    .await?;
            }
            "/skills" => self.cmd_skills_list()?,
            s if s.starts_with("/skills install ") => {
                let arg = s.trim_start_matches("/skills install ").trim();
                self.cmd_skills_install(arg)?;
            }
            "/skills sync" => self.cmd_skills_sync()?,
            "/skills discover" => self.cmd_skills_discover()?,
            "/sessions" => self.cmd_sessions_browse().await?,
            "/sessions list" => self.cmd_sessions_list().await?,
            "/resume" => self.cmd_resume().await?,
            "/new" => self.cmd_new_session().await?,
            "/approve" | "/yolo" => {
                self.session_approve_all.store(true, Ordering::SeqCst);
                ui::print_success("tools auto-approved for this session");
            }
            "/provider" => {
                if let Some(name) = &self.provider_name {
                    ui::print_info(format!("provider: {name} · model: {}", self.model));
                } else {
                    ui::print_info(format!("model: {} (env default)", self.model));
                }
            }
            "/mcp" => self.cmd_mcp_status().await?,
            "/mcp reload" => self.cmd_mcp_reload().await?,
            "/theme" => self.cmd_theme(line)?,
            "/approvals" => self.cmd_approvals(line)?,
            "/context" => self.cmd_context().await?,
            s if s.starts_with("/compact") => self.cmd_compact(s).await?,
            "/fork" => self.cmd_fork().await?,
            s if s.starts_with("/export") => self.cmd_export(s).await?,
            s if s.starts_with("/history") => self.cmd_history(s).await?,
            s if s.starts_with("/team ") => {
                let goal = s.trim_start_matches("/team ").trim();
                self.cmd_team(goal).await?;
            }
            "/team" => ui::print_info("usage: /team <goal>  (architect→code→review→test pipeline)"),
            s if s.starts_with("/plugins install ") => {
                let id = s.trim_start_matches("/plugins install ").trim();
                self.cmd_plugins_install(id)?;
            }
            "/plugins" => self.cmd_plugins_browse().await?,
            "/hooks" => self.cmd_hooks(line)?,
            s if s.starts_with("/sandbox") => self.cmd_sandbox(s)?,
            "/vector-index" | "/vector" => self.cmd_vector_index().await?,
            s if s.starts_with("/sync") => self.cmd_sync(s).await?,
            "/collab" => self.cmd_collab()?,
            "/profile" | "/stats" => self.cmd_profile().await?,
            s if s.starts_with("/session ") => {
                let id_str = s.trim_start_matches("/session ").trim();
                self.cmd_switch_session(id_str).await?;
            }
            _ if line.starts_with('/') => {
                ui::print_error(format!("unknown command: {line} (try /help)"));
            }
            _ => {
                let expanded = input::expand_message(line, &self.project.root);
                ui::print_user_turn_separator();
                self.runner().send_and_stream(&expanded).await?;
                SessionPin::save(&self.project.root, self.session_id, Some("REPL"))?;
            }
        }
        Ok(false)
    }

    fn cmd_init(&mut self) -> anyhow::Result<()> {
        const TEMPLATE: &str = "# Project instructions for Nexus / Claude Code\n\n\
            Describe your project, conventions, and how the AI should help.\n\n\
            ## Goals\n\n\
            ## Commands\n\n\
            ## Code style\n\n\
            ## Do not\n\n";
        let mut created = Vec::new();
        for name in ["CLAUDE.md", "PROJECT.md", "NEXUS.md"] {
            let path = self.project.root.join(name);
            if path.exists() {
                continue;
            }
            std::fs::write(&path, TEMPLATE)?;
            created.push(name.to_string());
        }
        if created.is_empty() {
            ui::print_info("CLAUDE.md / PROJECT.md / NEXUS.md already exist");
        } else {
            ui::print_success(format!("created: {}", created.join(", ")));
        }
        nexus_core::project::reload_instructions(&mut self.project)?;
        ui::print_info(format!(
            "loaded {} instruction file(s) into context",
            self.project.instructions.files.len()
        ));
        Ok(())
    }

    fn cmd_docs(&self) -> anyhow::Result<()> {
        use nexus_core::project::instructions;
        if self.project.instructions.files.is_empty() {
            ui::print_info("no instruction files loaded — run /init or add CLAUDE.md");
        } else {
            let rows: Vec<_> = self
                .project
                .instructions
                .files
                .iter()
                .map(|f| (f.label.clone(), f.path.display().to_string()))
                .collect();
            ui::print_instruction_files(&rows);
        }
        let discovered =
            instructions::discover_markdown(&self.project.root, 40).unwrap_or_default();
        if !discovered.is_empty() {
            ui::print_info(format!(
                "{} additional .md/.mdc files under project (use read_file)",
                discovered.len()
            ));
        }
        Ok(())
    }

    fn cmd_skills_list(&self) -> anyhow::Result<()> {
        let entries = skills::list_skills(&self.config.data_dir, &self.project.root)?;
        let rows: Vec<_> = entries
            .iter()
            .map(|e| (e.name.clone(), e.scope.to_string()))
            .collect();
        ui::print_skills(&rows);
        Ok(())
    }

    fn cmd_skills_sync(&mut self) -> anyhow::Result<()> {
        let installed = skills_ext::sync_external_skills(&self.config.data_dir)?;
        if installed.is_empty() {
            ui::print_info("no external skills found under ~/.cursor, ~/.claude, ~/.codex");
        } else {
            for p in &installed {
                ui::print_success(format!("synced → {p}"));
            }
        }
        self.skills_context =
            skills::build_skills_context(&self.config.data_dir, &self.project.root)?;
        Ok(())
    }

    fn cmd_skills_discover(&self) -> anyhow::Result<()> {
        let found = skills_ext::discover_external_skills();
        if found.is_empty() {
            ui::print_info("no external SKILL.md bundles found");
            return Ok(());
        }
        let rows: Vec<_> = found
            .iter()
            .map(|(name, path, scope)| (name.clone(), format!("{scope} · {}", path.display())))
            .collect();
        ui::print_skills(&rows);
        ui::print_info("install with: /skills sync");
        Ok(())
    }

    async fn cmd_plugins_browse(&self) -> anyhow::Result<()> {
        let plugins_dir = nexus_core::plugins::default_plugins_dir(&self.config.data_dir);
        let mut mgr = nexus_core::plugins::PluginManager::new(plugins_dir);
        mgr.scan()?;
        let catalog = crate::plugin_tui::load_catalog();
        let rows = crate::plugin_tui::build_rows(&mgr, &catalog);
        if rows.is_empty() {
            ui::print_info("no plugins — see /plugins or install under nexus-ide/plugins/");
            return Ok(());
        }
        let install_id = tokio::task::spawn_blocking(move || {
            crate::plugin_tui::run_plugin_browser(rows)
        })
        .await??;
        if let Some(id) = install_id {
            self.cmd_plugins_install(&id)?;
        }
        Ok(())
    }

    fn cmd_collab(&self) -> anyhow::Result<()> {
        let peers = crate::collab::read_peers(&self.project.root);
        ui::print_collab_strip(&peers);
        Ok(())
    }

    async fn cmd_profile(&self) -> anyhow::Result<()> {
        let snap = crate::profiler::collect(
            &self.store,
            &self.config.engine_url,
            &self.config.data_dir,
            self.session_id,
        )
        .await?;
        ui::print_profile_box(&crate::profiler::format_snapshot(&snap));
        Ok(())
    }

    fn cmd_approvals(&mut self, line: &str) -> anyhow::Result<()> {
        let arg = line.trim_start_matches("/approvals").trim();
        if arg.is_empty() {
            ui::print_info(format!(
                "approvals: {} (reads auto; writes/shell: {})",
                self.approval_mode.label(),
                match self.approval_mode {
                    ApprovalMode::Suggest => "prompt",
                    ApprovalMode::AutoEdit => "writes auto, shell prompt",
                    ApprovalMode::FullAuto => "all auto",
                }
            ));
            ui::print_info("usage: /approvals suggest | auto-edit | full-auto");
            return Ok(());
        }
        if let Some(mode) = ApprovalMode::parse(arg) {
            self.approval_mode = mode;
            ui::print_success(format!("approvals → {}", mode.label()));
        } else {
            anyhow::bail!("unknown mode: {arg}");
        }
        Ok(())
    }

    async fn cmd_context(&self) -> anyhow::Result<()> {
        let snap = crate::profiler::collect(
            &self.store,
            &self.config.engine_url,
            &self.config.data_dir,
            self.session_id,
        )
        .await?;
        let instr_kb = self
            .project
            .project_md
            .as_ref()
            .map(|s| s.len())
            .unwrap_or(0) as f64
            / 1024.0;
        let skills_kb = self.skills_context.len() as f64 / 1024.0;
        ui::print_profile_box(&format!(
            "Context (estimate)\n\
             Instructions   ~{instr_kb:.1} KB (CLAUDE.md / PROJECT.md / …)\n\
             Skills         ~{skills_kb:.1} KB\n\
             Messages       {} in DB (this session)\n\
             Engine         {} · {} ms\n\
             Mode           {} · approvals {}",
            snap.message_count,
            if snap.engine_online { "online" } else { "offline" },
            snap.engine_latency_ms,
            self.mode.label(),
            self.approval_mode.label(),
        ));
        Ok(())
    }

    async fn cmd_compact(&mut self, line: &str) -> anyhow::Result<()> {
        let arg = line.trim_start_matches("/compact").trim();
        let semantic = arg != "fast" && arg != "rule";
        let url = format!(
            "{}/v1/sessions/{}/compact",
            self.config.engine_url.trim_end_matches('/'),
            self.session_id
        );
        if semantic {
            ui::print_info("compacting with LLM summary (use `/compact fast` for rule-only)…");
        }
        let res = self
            .http
            .post(&url)
            .json(&serde_json::json!({ "semantic": semantic }))
            .send()
            .await?;
        if !res.status().is_success() {
            anyhow::bail!("compact HTTP {}", res.status());
        }
        let body: serde_json::Value = res.json().await?;
        let empty: Vec<serde_json::Value> = vec![];
        let pairs: Vec<(MessageRole, String)> = body["messages"]
            .as_array()
            .unwrap_or(&empty)
            .iter()
            .filter_map(|m| {
                let role = match m.get("role")?.as_str()? {
                    "assistant" => MessageRole::Assistant,
                    "user" => MessageRole::User,
                    _ => return None,
                };
                Some((role, m.get("content")?.as_str()?.to_string()))
            })
            .collect();
        self.store
            .replace_session_messages(self.session_id, &pairs)
            .await?;
        self.runner().hydrate_engine_session().await?;
        let sem = body["semantic"].as_bool().unwrap_or(false);
        ui::print_success(format!(
            "compact{}: {} → {} messages · {} → {} chars",
            if sem { " (semantic)" } else { "" },
            body["messages_before"].as_u64().unwrap_or(0),
            body["messages_after"].as_u64().unwrap_or(0),
            body["chars_before"].as_u64().unwrap_or(0),
            body["chars_after"].as_u64().unwrap_or(0),
        ));
        Ok(())
    }

    async fn cmd_team(&mut self, goal: &str) -> anyhow::Result<()> {
        if goal.is_empty() {
            anyhow::bail!("empty goal");
        }
        let prev = self.mode;
        self.mode = ChatMode::Agent;
        ui::print_user_turn_separator();
        self.runner().send_and_stream(goal).await?;
        self.mode = prev;
        Ok(())
    }

    fn cmd_plugins_install(&self, id: &str) -> anyhow::Result<()> {
        use nexus_core::plugins::{default_plugins_dir, PluginManager, PluginPermission};

        let catalog = crate::plugin_tui::load_catalog();
        let entry = catalog
            .plugins
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| anyhow::anyhow!("unknown plugin id: {id}"))?;
        let perms: Vec<PluginPermission> = entry
            .permissions
            .iter()
            .filter_map(|s| match s.as_str() {
                "read_files" => Some(PluginPermission::ReadFiles),
                "write_files" => Some(PluginPermission::WriteFiles),
                "run_shell" => Some(PluginPermission::RunShell),
                "network" => Some(PluginPermission::Network),
                "mcp_bridge" => Some(PluginPermission::McpBridge),
                _ => None,
            })
            .collect();
        let dir = default_plugins_dir(&self.config.data_dir);
        let mut mgr = PluginManager::new(dir);
        mgr.scan()?;
        let root = mgr.install_scaffold(
            &entry.id,
            &entry.name,
            &entry.version,
            &entry.description,
            &perms,
        )?;
        ui::print_success(format!("installed plugin → {}", root.display()));
        Ok(())
    }

    fn cmd_sandbox(&mut self, line: &str) -> anyhow::Result<()> {
        let arg = line.trim_start_matches("/sandbox").trim();
        if arg.is_empty() {
            ui::print_info(format!(
                "sandbox: {} (shell via run_shell; docker needs Docker daemon)",
                self.config.sandbox_mode
            ));
            ui::print_info("usage: /sandbox local | docker");
            return Ok(());
        }
        let mode = match arg.to_lowercase().as_str() {
            "local" | "off" => "local",
            "docker" | "container" => "docker",
            _ => {
                ui::print_error("use: /sandbox local | docker");
                return Ok(());
            }
        };
        self.config.sandbox_mode = mode.into();
        self.config.validate()?;
        self.config.save_to_data_dir()?;
        std::env::set_var("NEXUS_SANDBOX", mode);
        ui::print_success(format!("sandbox_mode = {mode}"));
        Ok(())
    }

    async fn cmd_vector_index(&self) -> anyhow::Result<()> {
        let url = format!(
            "{}/v1/vector/index",
            self.config.engine_url.trim_end_matches('/')
        );
        let res = self
            .http
            .post(url)
            .json(&serde_json::json!({
                "workspace_root": self.project.root.to_string_lossy(),
                "max_files": 800
            }))
            .send()
            .await?;
        let body: serde_json::Value = res.json().await?;
        ui::print_success(format!(
            "vector index: {} files · {} chunks",
            body["files_indexed"].as_u64().unwrap_or(0),
            body["chunks"].as_u64().unwrap_or(0)
        ));
        Ok(())
    }

    async fn cmd_sync(&self, _line: &str) -> anyhow::Result<()> {
        let url = format!(
            "{}/v1/sync/export",
            self.config.engine_url.trim_end_matches('/')
        );
        let res = self
            .http
            .post(url)
            .query(&[
                ("workspace_root", self.project.root.to_string_lossy().as_ref()),
                ("session_id", self.session_id.to_string().as_str()),
            ])
            .send()
            .await?;
        let body: serde_json::Value = res.json().await?;
        ui::print_success(format!(
            "sync snapshot → {}",
            body["path"].as_str().unwrap_or("?")
        ));
        Ok(())
    }

    fn cmd_hooks(&self, line: &str) -> anyhow::Result<()> {
        let arg = line.trim_start_matches("/hooks").trim();
        let path = self.project.root.join(".nexus").join("hooks.toml");
        if arg == "init" {
            std::fs::create_dir_all(path.parent().unwrap())?;
            if path.is_file() {
                ui::print_info(format!("exists: {}", path.display()));
            } else {
                std::fs::write(&path, crate::hooks::example_hooks_toml())?;
                ui::print_success(format!("created {}", path.display()));
            }
            return Ok(());
        }
        ui::print_info(format!(
            "hooks: {} ({})",
            if path.is_file() { "configured" } else { "none" },
            path.display()
        ));
        ui::print_info("usage: /hooks init");
        Ok(())
    }

    async fn cmd_export(&self, line: &str) -> anyhow::Result<()> {
        let arg = line.trim_start_matches("/export").trim();
        let path = if arg.is_empty() {
            self.project
                .root
                .join(".nexus")
                .join(format!("session-{}.json", &self.session_id.to_string()[..8]))
        } else {
            std::path::PathBuf::from(arg)
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let msgs = self.store.list_messages(self.session_id, 2000).await?;
        let mut ordered = msgs;
        ordered.sort_by_key(|m| m.sequence);
        let payload: Vec<_> = ordered
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                        MessageRole::System => "system",
                        MessageRole::Tool => "tool",
                    },
                    "content": m.content,
                    "sequence": m.sequence,
                })
            })
            .collect();
        std::fs::write(&path, serde_json::to_string_pretty(&payload)?)?;
        ui::print_success(format!("exported {} messages → {}", ordered.len(), path.display()));
        Ok(())
    }

    async fn cmd_history(&self, line: &str) -> anyhow::Result<()> {
        let arg = line.trim_start_matches("/history").trim();
        let limit: u32 = if arg.is_empty() {
            12
        } else {
            arg.parse().unwrap_or(12)
        };
        let msgs = self.store.list_messages(self.session_id, limit).await?;
        let mut ordered = msgs;
        ordered.sort_by_key(|m| m.sequence);
        for m in ordered {
            let role = match m.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::System => "system",
                MessageRole::Tool => "tool",
            };
            let preview: String = m.content.chars().take(120).collect();
            println!(
                "{} {}",
                format!("[{role}]").truecolor(198, 120, 73),
                if m.content.len() > 120 {
                    format!("{preview}…")
                } else {
                    preview
                }
            );
        }
        Ok(())
    }

    async fn cmd_fork(&mut self) -> anyhow::Result<()> {
        let msgs = self.store.list_messages(self.session_id, 500).await?;
        let mut ordered: Vec<_> = msgs
            .into_iter()
            .filter(|m| matches!(m.role, MessageRole::User | MessageRole::Assistant))
            .collect();
        ordered.sort_by_key(|m| m.sequence);
        let new_session = self.store.create_session(None, Some("Fork")).await?;
        for m in &ordered {
            self.store
                .append_message(new_session.id, m.role.clone(), &m.content, None)
                .await?;
        }
        self.session_id = new_session.id;
        SessionPin::save(&self.project.root, self.session_id, Some("Fork"))?;
        self.session_approve_all.store(false, Ordering::SeqCst);
        self.runner().hydrate_engine_session().await?;
        ui::print_success(format!(
            "forked {} messages → session {}",
            ordered.len(),
            ui::short_session_id(&self.session_id)
        ));
        Ok(())
    }

    fn cmd_theme(&mut self, line: &str) -> anyhow::Result<()> {
        let arg = line.trim_start_matches("/theme").trim();
        if arg.is_empty() {
            ui::print_info(format!("theme: {}", self.config.theme));
            ui::print_info("usage: /theme light | dark | carbon");
            return Ok(());
        }
        let id = match arg.to_lowercase().as_str() {
            "light" => crate::theme::ThemeId::Light,
            "carbon" => crate::theme::ThemeId::Carbon,
            "dark" => crate::theme::ThemeId::Dark,
            other => anyhow::bail!("unknown theme: {other}"),
        };
        crate::theme::apply_theme(&mut self.config, id)?;
        ui::print_success(format!("theme → {arg}"));
        Ok(())
    }

    async fn cmd_sessions_browse(&mut self) -> anyhow::Result<()> {
        let rows = self.store.list_sessions(100).await?;
        if rows.is_empty() {
            ui::print_info("no sessions yet");
            return Ok(());
        }
        let items: Vec<_> = rows
            .iter()
            .map(crate::session_tui::SessionRow::from_session)
            .collect();
        let picked = tokio::task::spawn_blocking({
            let current = self.session_id;
            move || crate::session_tui::run_session_browser(items, current)
        })
        .await??;
        if let Some(id) = picked {
            if id != self.session_id {
                self.session_id = id;
                SessionPin::save(&self.project.root, self.session_id, Some("REPL"))?;
                self.runner().hydrate_engine_session().await?;
                ui::print_success(format!("switched to {}", ui::short_session_id(&self.session_id)));
            }
        }
        Ok(())
    }

    async fn cmd_sessions_list(&self) -> anyhow::Result<()> {
        let rows = self.store.list_sessions(15).await?;
        if rows.is_empty() {
            ui::print_info("no sessions yet");
            return Ok(());
        }
        println!();
        for s in rows {
            let mark = if s.id == self.session_id { "●" } else { "○" };
            println!(
                "  {} {}  {}  {}",
                mark.green(),
                ui::short_session_id(&s.id),
                s.title.as_deref().unwrap_or("—"),
                format!("{:?}", s.status).dimmed()
            );
        }
        println!();
        Ok(())
    }

    async fn cmd_resume(&mut self) -> anyhow::Result<()> {
        let pin = SessionPin::load(&self.project.root)
            .ok_or_else(|| anyhow::anyhow!("no pinned session — start chatting first"))?;
        self.session_id = pin.session_id;
        self.runner().hydrate_engine_session().await?;
        let msgs = self.store.list_messages(self.session_id, 8).await?;
        ui::print_success(format!(
            "resumed {} · last {} message(s)",
            ui::short_session_id(&self.session_id),
            msgs.len()
        ));
        for m in msgs.iter().rev().take(4) {
            let preview: String = m.content.chars().take(80).collect();
            let role = match m.role {
                MessageRole::User => "you",
                MessageRole::Assistant => "nexus",
                _ => "sys",
            };
            println!("    {} {}", role.dimmed(), preview.dimmed());
        }
        Ok(())
    }

    async fn cmd_new_session(&mut self) -> anyhow::Result<()> {
        let session = self.store.create_session(None, Some("REPL")).await?;
        self.session_id = session.id;
        SessionPin::save(&self.project.root, self.session_id, Some("REPL"))?;
        self.session_approve_all.store(false, Ordering::SeqCst);
        self.runner().hydrate_engine_session().await.ok();
        ui::print_success(format!("new session {}", ui::short_session_id(&self.session_id)));
        Ok(())
    }

    async fn cmd_switch_session(&mut self, id_str: &str) -> anyhow::Result<()> {
        let id = if let Ok(uuid) = uuid::Uuid::parse_str(id_str) {
            uuid
        } else {
            let rows = self.store.list_sessions(50).await?;
            rows.into_iter()
                .find(|s| s.id.to_string().starts_with(id_str))
                .map(|s| s.id)
                .ok_or_else(|| anyhow::anyhow!("session not found: {id_str}"))?
        };
        self.store
            .get_session(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("session not found"))?;
        self.session_id = id;
        SessionPin::save(&self.project.root, self.session_id, Some("REPL"))?;
        self.runner().hydrate_engine_session().await?;
        ui::print_success(format!("switched to {}", ui::short_session_id(&self.session_id)));
        Ok(())
    }

    async fn cmd_mcp_status(&self) -> anyhow::Result<()> {
        let url = format!(
            "{}/v1/mcp/status",
            self.config.engine_url.trim_end_matches('/')
        );
        let res = self
            .http
            .get(&url)
            .query(&[("workspace_root", self.workspace_root.as_str())])
            .send()
            .await?;
        if !res.status().is_success() {
            anyhow::bail!("mcp status HTTP {}", res.status());
        }
        let body: serde_json::Value = res.json().await?;
        let paths = body["config_paths"].as_array().cloned().unwrap_or_default();
        if paths.is_empty() {
            let dest = self.config.data_dir.join("mcp.toml");
            if !dest.exists() {
                const EXAMPLE: &str =
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../config.mcp.example.toml"));
                std::fs::create_dir_all(&self.config.data_dir)?;
                std::fs::write(&dest, EXAMPLE)?;
                ui::print_success(format!("created {}", dest.display()));
                ui::print_info("Edit mcp.toml, set enabled = true, then /mcp reload");
            }
        }
        let servers = body["servers"].as_array().cloned().unwrap_or_default();
        println!();
        ui::print_info("MCP servers");
        for s in &servers {
            let name = s["name"].as_str().unwrap_or("-");
            let connected = s["connected"].as_bool().unwrap_or(false);
            let tools = s["tools"].as_u64().unwrap_or(0);
            let mark = if connected { "●" } else { "○" };
            println!(
                "  {} {} — {} tools",
                mark,
                name,
                tools
            );
        }
        if let Some(p) = paths.first() {
            ui::print_info(format!("config: {}", p.as_str().unwrap_or("")));
        }
        println!();
        Ok(())
    }

    async fn cmd_mcp_reload(&self) -> anyhow::Result<()> {
        let url = format!(
            "{}/v1/mcp/reload",
            self.config.engine_url.trim_end_matches('/')
        );
        let res = self
            .http
            .post(&url)
            .query(&[("workspace_root", self.workspace_root.as_str())])
            .send()
            .await?;
        if !res.status().is_success() {
            anyhow::bail!("mcp reload HTTP {}", res.status());
        }
        ui::print_success("MCP registry reloaded");
        self.cmd_mcp_status().await
    }

    fn cmd_skills_install(&mut self, arg: &str) -> anyhow::Result<()> {
        let path = if slash::bundled_skill_names().contains(&arg) {
            skills::install_bundled(arg, &self.config.data_dir)?
        } else {
            let p = PathBuf::from(arg);
            skills::install_from_path(&p, &self.config.data_dir, None)?
        };
        self.skills_context =
            skills::build_skills_context(&self.config.data_dir, &self.project.root)?;
        ui::print_success(format!("installed → {}", path.display()));
        Ok(())
    }

    fn runner(&self) -> ChatRunner<'_> {
        ChatRunner {
            http: &self.http,
            engine_url: &self.config.engine_url,
            session_id: self.session_id,
            model: &self.model,
            project_md: self.project.project_md.as_deref(),
            workspace_root: &self.workspace_root,
            skills_context: if self.skills_context.is_empty() {
                None
            } else {
                Some(&self.skills_context)
            },
            mode: self.mode,
            tools: &self.tools,
            store: &self.store,
            cancelled: self.cancelled.clone(),
            session_approve_all: self.session_approve_all.clone(),
            approval_mode: self.approval_mode,
        }
    }
}
