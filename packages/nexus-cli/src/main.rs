mod approval;
mod at_resolve;
mod chat;
mod completer;
mod diff;
mod engine_cmd;
mod errors;
mod input;
mod logo;
mod markdown;
mod menu;
mod mode;
mod progress;
mod provider_cmd;
mod repl;
mod search_fmt;
mod session_pin;
mod session_tui;
mod session_ui;
mod theme;
mod fuzzy;
mod hooks;
mod json_schema;
mod collab;
mod profiler;
mod plugin_tui;
mod skills;
mod skills_ext;
mod slash;
mod tool_format;
mod trust;
mod ui;

use clap::{Parser, Subcommand};
use colored::Colorize;
use nexus_core::config::NexusConfig;
use nexus_core::project::{ProjectContext, ProjectIndexer};
use nexus_core::storage::sqlite::SqliteStore;
use nexus_core::storage::SessionRepository;
use nexus_core::tools::{workspace_registry, WorkspaceToolContext};
use std::env;
use std::io::Read;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Parser)]
#[command(
    name = "nexus",
    version,
    about = "Nexus-Transe — Cybertron terminal command interface",
    after_help = "Quick start: nexus   (or nx) — opens chat REPL in current directory"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Interactive chat (REPL) — default when no subcommand
    #[command(alias = "c")]
    Chat {
        #[arg(short, long)]
        model: Option<String>,
        /// One-shot prompt; use `-` for stdin
        #[arg(short = 'p', long)]
        prompt: Option<String>,
        /// Auto-approve tools for one-shot runs
        #[arg(long)]
        yes: bool,
        /// Do not auto-start Python engine when offline
        #[arg(long)]
        no_start_engine: bool,
        #[arg(long)]
        json_schema: Option<std::path::PathBuf>,
        #[arg(long)]
        json_out: Option<std::path::PathBuf>,
    },
    /// One-shot task
    Run {
        prompt: String,
        #[arg(short, long)]
        model: Option<String>,
        #[arg(long)]
        yes: bool,
        /// Validate final assistant JSON against this schema file
        #[arg(long)]
        json_schema: Option<std::path::PathBuf>,
        /// Write validated JSON to file (stdout if omitted)
        #[arg(long)]
        json_out: Option<std::path::PathBuf>,
    },
    /// Index project files
    Index,
    /// Initialize PROJECT.md
    Init,
    /// Index workspace for semantic search (Chroma via engine)
    VectorIndex,
    /// Search codebase (regex)
    Search {
        pattern: String,
        #[arg(long, default_value = "50")]
        limit: usize,
    },
    /// LLM providers (relay / CC Switch compatible)
    Provider {
        #[command(subcommand)]
        action: ProviderAction,
    },
    /// Manage encrypted API keys
    Secrets {
        #[command(subcommand)]
        action: SecretsAction,
    },
    Engine {
        #[command(subcommand)]
        action: EngineAction,
    },
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// Collaboration relay (WebSocket on engine)
    Collab {
        #[command(subcommand)]
        action: CollabAction,
    },
    /// List installed plugins (JSON)
    Plugins {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum CollabAction {
    /// Print WebSocket relay URL for this workspace
    Url,
}

#[derive(Subcommand)]
enum ProviderAction {
    /// List configured providers
    List,
    /// Switch active provider
    Use { id: String },
    /// Import from CC Switch / external settings.json
    ImportCcSwitch {
        #[arg(long, short)]
        from: Option<std::path::PathBuf>,
    },
    /// Create default providers.toml
    Init,
    /// Diagnose active provider API key (401 errors)
    Doctor,
}

#[derive(Subcommand)]
enum SecretsAction {
    /// Set provider API key (encrypted)
    Set { provider: String, key: String },
}

#[derive(Subcommand)]
enum EngineAction {
    Status,
    /// Start Python sidecar in background (requires `uv` + packages/nexus-engine)
    Start,
}

#[derive(Subcommand)]
enum SessionAction {
    List {
        #[arg(long, default_value = "20")]
        limit: u32,
    },
    Show { id: Uuid },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("nexus=warn".parse()?),
        )
        .init();

    let cli = Cli::parse();
    let mut config = load_config()?;
    theme::init_from_config(&config);

    let db_path = config.data_dir.join("nexus.db");
    let store = SqliteStore::connect(&db_path).await?;
    let cwd = env::current_dir()?;

    let command = cli.command.unwrap_or(Commands::Chat {
        model: None,
        prompt: None,
        yes: false,
        no_start_engine: false,
        json_schema: None,
        json_out: None,
    });

    match command {
        Commands::Chat {
            model,
            prompt,
            yes,
            no_start_engine,
            json_schema,
            json_out,
        } => {
            let model_override = model.clone();
            if let Some(m) = model {
                config.default_model = m;
            }
            if !no_start_engine {
                engine_cmd::ensure_engine(&config.engine_url, true).await?;
            }
            if let Some(p) = prompt {
                run_oneshot(
                    store,
                    config,
                    cwd,
                    &p,
                    model_override,
                    yes,
                    json_schema,
                    json_out,
                )
                .await?;
            } else {
                let mut repl = repl::ReplSession::new(store, config, cwd).await?;
                repl.run().await?;
            }
        }
        Commands::Run {
            prompt,
            model,
            yes,
            json_schema,
            json_out,
        } => {
            let project = ProjectContext::detect(&cwd)?
                .ok_or_else(|| anyhow::anyhow!("no project root"))?;
            let model_id = model.unwrap_or(config.default_model.clone());
            let ws = WorkspaceToolContext {
                project: Arc::new(project.clone()),
                auto_approve: yes,
                engine_url: config.engine_url.clone(),
                sandbox_mode: config.sandbox_mode.clone(),
            };
            let tools = workspace_registry(Arc::new(ws));
            let session = store.create_session(None, Some("Run")).await?;
            engine_cmd::ensure_engine(&config.engine_url, true).await?;
            let skills_ctx =
                skills::build_skills_context(&config.data_dir, &project.root).unwrap_or_default();
            let workspace = project.root.to_string_lossy().into_owned();
            let runner = chat::ChatRunner {
                http: &reqwest::Client::new(),
                engine_url: &config.engine_url,
                session_id: session.id,
                model: &model_id,
                project_md: project.project_md.as_deref(),
                workspace_root: &workspace,
                skills_context: if skills_ctx.is_empty() {
                    None
                } else {
                    Some(skills_ctx.as_str())
                },
                mode: mode::ChatMode::Default,
                tools: &tools,
                store: &store,
                cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                session_approve_all: Arc::new(std::sync::atomic::AtomicBool::new(yes)),
                approval_mode: if yes {
                    approval::ApprovalMode::FullAuto
                } else {
                    approval::ApprovalMode::Suggest
                },
            };
            let mut expanded = input::expand_message(&prompt, &project.root);
            if let Some(ref schema_path) = json_schema {
                let schema = json_schema::load_schema(schema_path)?;
                expanded = json_schema::wrap_prompt_for_json(&expanded, &schema);
            }
            ui::print_user_turn_separator();
            let assistant = runner.send_and_stream(&expanded).await?;
            if let Some(ref schema_path) = json_schema {
                let schema = json_schema::load_schema(schema_path)?;
                let value = json_schema::extract_json(&assistant)?;
                if let Err(e) =
                    json_schema::validate_via_engine(&config.engine_url, &value, &schema).await
                {
                    json_schema::validate_against_schema(&value, &schema)
                        .map_err(|_| e)?;
                }
                let out_text = serde_json::to_string_pretty(&value)?;
                if let Some(ref out_path) = json_out {
                    std::fs::write(out_path, &out_text)?;
                    ui::print_success(format!("JSON → {}", out_path.display()));
                } else {
                    println!("{out_text}");
                }
            }
            ui::print_info(format!("session {}", ui::short_session_id(&session.id)));
        }
        Commands::Collab { action } => match action {
            CollabAction::Url => {
                engine_cmd::ensure_engine(&config.engine_url, false).await?;
                let ws = cwd.to_string_lossy().replace('\\', "/");
                let base = config.engine_url.trim_end_matches('/');
                let ws_enc = percent_encode_path(&ws);
                ui::print_info(format!(
                    "collab WebSocket: {base}/v1/collab/ws?workspace={ws_enc}"
                ));
                ui::print_info("send JSON: {\"type\":\"presence\",\"user\":\"you\",\"message\":\"...\"}");
            }
        },
        Commands::VectorIndex => {
            let project = ProjectContext::detect(&cwd)?
                .ok_or_else(|| anyhow::anyhow!("no project root"))?;
            engine_cmd::ensure_engine(&config.engine_url, true).await?;
            let url = format!("{}/v1/vector/index", config.engine_url.trim_end_matches('/'));
            let res = reqwest::Client::new()
                .post(url)
                .json(&serde_json::json!({
                    "workspace_root": project.root.to_string_lossy(),
                    "max_files": 800
                }))
                .send()
                .await?;
            let body: serde_json::Value = res.json().await?;
            ui::print_success(format!(
                "indexed {} files · {} chunks",
                body["files_indexed"].as_u64().unwrap_or(0),
                body["chunks"].as_u64().unwrap_or(0)
            ));
        }
        Commands::Index => {
            let project = ProjectContext::detect(&cwd)?
                .ok_or_else(|| anyhow::anyhow!("no project root"))?;
            let indexer = ProjectIndexer::new(&project.root)?;
            let files = indexer.walk(&project.root)?;
            println!("{} files indexed under {}", files.len().to_string().green(), project.root.display());
        }
        Commands::Search { pattern, limit } => {
            let project = ProjectContext::detect(&cwd)?
                .ok_or_else(|| anyhow::anyhow!("no project root"))?;
            let matches = nexus_core::search::search_codebase(&project.root, &pattern, limit)?;
            search_fmt::print_matches_highlighted(&matches, Some(&pattern));
            println!("{} matches", matches.len().to_string().green());
        }
        Commands::Provider { action } => match action {
            ProviderAction::List => provider_cmd::run_list(&config)?,
            ProviderAction::Use { id } => provider_cmd::run_use(&config, &id)?,
            ProviderAction::ImportCcSwitch { from } => {
                provider_cmd::run_import(&config, from)?
            }
            ProviderAction::Init => provider_cmd::run_init(&config)?,
            ProviderAction::Doctor => provider_cmd::run_doctor(&config)?,
        },
        Commands::Secrets { action } => {
            use nexus_core::secrets::SecretVault;
            let SecretsAction::Set { provider, key } = action;
            let vault = SecretVault::open(&config.data_dir)?;
            let mut store = vault.load()?;
            vault.set_api_key(&mut store, &provider, &key)?;
            ui::print_success(format!("stored key for {provider}"));
        }
        Commands::Init => {
            let project = ProjectContext::detect(&cwd)?.unwrap_or_else(|| {
                let instructions =
                    nexus_core::project::instructions::InstructionBundle::default();
                ProjectContext {
                    root: cwd.clone(),
                    project_md: None,
                    instructions,
                    name: None,
                }
            });
            let path = project.root.join("PROJECT.md");
            if path.exists() {
                println!("{}", "PROJECT.md already exists".yellow());
            } else {
                std::fs::write(
                    &path,
                    "# Project\n\nDescribe your project, conventions, and AI instructions here.\n",
                )?;
                println!("{} {}", "Created".green(), path.display());
            }
        }
        Commands::Engine { action } => match action {
            EngineAction::Status => {
                let ok = engine_cmd::is_engine_online(&config.engine_url).await;
                ui::print_engine_status(&config.engine_url, ok);
            }
            EngineAction::Start => {
                let dir = engine_cmd::discover_engine_package().ok_or_else(|| {
                    anyhow::anyhow!(
                        "cannot find packages/nexus-engine — set NEXUS_ENGINE_DIR or run from repo root"
                    )
                })?;
                engine_cmd::start_engine_detached(&dir)?;
                ui::print_info(format!("spawned engine in {}", dir.display()));
                if engine_cmd::wait_for_health(&config.engine_url, std::time::Duration::from_secs(45))
                    .await?
                {
                    ui::print_success(format!("engine online at {}", config.engine_url));
                } else {
                    anyhow::bail!("engine not healthy at {}", config.engine_url);
                }
            }
        },
        Commands::Plugins { json } => {
            use nexus_core::plugins::{default_plugins_dir, PluginManager};
            let dir = default_plugins_dir(&config.data_dir);
            let mut mgr = PluginManager::new(dir);
            mgr.scan()?;
            if json {
                let items: Vec<_> = mgr
                    .list()
                    .iter()
                    .map(|p| {
                        serde_json::json!({
                            "id": p.manifest.id,
                            "name": p.manifest.name,
                            "version": p.manifest.version,
                            "permissions": p.manifest.permissions,
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string(&items)?);
            } else {
                for p in mgr.list() {
                    println!("{} {} v{}", p.manifest.id, p.manifest.name, p.manifest.version);
                }
            }
        },
        Commands::Session { action } => match action {
            SessionAction::List { limit } => {
                for s in store.list_sessions(limit).await? {
                    println!(
                        "{}  {}  {:?}",
                        s.id,
                        s.title.as_deref().unwrap_or("-"),
                        s.status
                    );
                }
            }
            SessionAction::Show { id } => {
                if let Some(s) = store.get_session(id).await? {
                    println!("{}", serde_json::to_string_pretty(&s)?);
                }
            }
        },
    }
    Ok(())
}

async fn run_oneshot(
    store: SqliteStore,
    config: NexusConfig,
    cwd: std::path::PathBuf,
    prompt_arg: &str,
    model: Option<String>,
    yes: bool,
    json_schema: Option<std::path::PathBuf>,
    json_out: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    let prompt = if prompt_arg == "-" {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf.trim().to_string()
    } else {
        prompt_arg.to_string()
    };
    if prompt.is_empty() {
        anyhow::bail!("empty prompt");
    }

    engine_cmd::ensure_engine(&config.engine_url, true).await?;

    let project = ProjectContext::detect(&cwd)?.ok_or_else(|| anyhow::anyhow!("no project root"))?;
    let model_id = model.unwrap_or(config.default_model.clone());
    let ws = WorkspaceToolContext {
        project: Arc::new(project.clone()),
        auto_approve: yes,
        engine_url: config.engine_url.clone(),
        sandbox_mode: config.sandbox_mode.clone(),
    };
    let tools = workspace_registry(Arc::new(ws));
    let skills_ctx = skills::build_skills_context(&config.data_dir, &project.root).unwrap_or_default();
    let workspace = project.root.to_string_lossy().into_owned();
    let session = store.create_session(None, Some("One-shot")).await?;
    let mut expanded = input::expand_message(&prompt, &project.root);
    if let Some(ref schema_path) = json_schema {
        let schema = json_schema::load_schema(schema_path)?;
        expanded = json_schema::wrap_prompt_for_json(&expanded, &schema);
    }
    let runner = chat::ChatRunner {
        http: &reqwest::Client::new(),
        engine_url: &config.engine_url,
        session_id: session.id,
        model: &model_id,
        project_md: project.project_md.as_deref(),
        workspace_root: &workspace,
        skills_context: if skills_ctx.is_empty() {
            None
        } else {
            Some(skills_ctx.as_str())
        },
        mode: mode::ChatMode::Default,
        tools: &tools,
        store: &store,
        cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        session_approve_all: Arc::new(std::sync::atomic::AtomicBool::new(yes)),
        approval_mode: if yes {
            approval::ApprovalMode::FullAuto
        } else {
            approval::ApprovalMode::Suggest
        },
    };
    ui::print_user_turn_separator();
    let assistant = runner.send_and_stream(&expanded).await?;
    if let Some(ref schema_path) = json_schema {
        let schema = json_schema::load_schema(schema_path)?;
        let value = json_schema::extract_json(&assistant)?;
        if let Err(e) =
            json_schema::validate_via_engine(&config.engine_url, &value, &schema).await
        {
            json_schema::validate_against_schema(&value, &schema).map_err(|_| e)?;
        }
        let out_text = serde_json::to_string_pretty(&value)?;
        if let Some(ref out_path) = json_out {
            std::fs::write(out_path, &out_text)?;
            ui::print_success(format!("JSON → {}", out_path.display()));
        } else {
            println!("{out_text}");
        }
    }
    ui::print_info(format!("session {}", ui::short_session_id(&session.id)));
    Ok(())
}

fn percent_encode_path(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' | b':' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn load_config() -> anyhow::Result<NexusConfig> {
    NexusConfig::load_merged().map_err(Into::into)
}
