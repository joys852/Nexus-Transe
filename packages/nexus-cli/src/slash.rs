//! Slash-command registry for REPL completion and help.

pub struct SlashCommand {
    pub name: &'static str,
    pub args: &'static str,
    pub summary: &'static str,
}

pub const COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "/help",
        args: "",
        summary: "show commands and modes",
    },
    SlashCommand {
        name: "/init",
        args: "",
        summary: "create CLAUDE.md / PROJECT.md / NEXUS.md",
    },
    SlashCommand {
        name: "/docs",
        args: "",
        summary: "list loaded instruction markdown files",
    },
    SlashCommand {
        name: "/plan",
        args: "",
        summary: "plan mode — design before editing (no tools)",
    },
    SlashCommand {
        name: "/agent",
        args: "",
        summary: "agent mode — multi-agent pipeline",
    },
    SlashCommand {
        name: "/chat",
        args: "",
        summary: "default chat mode with tools",
    },
    SlashCommand {
        name: "/mode",
        args: "",
        summary: "show current mode",
    },
    SlashCommand {
        name: "/skills",
        args: "",
        summary: "list installed skills",
    },
    SlashCommand {
        name: "/skills",
        args: " install <path|name>",
        summary: "install skill from path or bundled name",
    },
    SlashCommand {
        name: "/index",
        args: "",
        summary: "index project files",
    },
    SlashCommand {
        name: "/git-status",
        args: "",
        summary: "run git status tool",
    },
    SlashCommand {
        name: "/mcp",
        args: "",
        summary: "list MCP servers and tools",
    },
    SlashCommand {
        name: "/mcp",
        args: " reload",
        summary: "reload MCP config and reconnect",
    },
    SlashCommand {
        name: "/cancel",
        args: "",
        summary: "stop current generation",
    },
    SlashCommand {
        name: "/clear",
        args: "",
        summary: "visual spacer between turns",
    },
    SlashCommand {
        name: "/sessions",
        args: "",
        summary: "interactive session browser (TUI)",
    },
    SlashCommand {
        name: "/sessions",
        args: " list",
        summary: "plain-text session list",
    },
    SlashCommand {
        name: "/theme",
        args: " [light|dark|carbon]",
        summary: "terminal color theme",
    },
    SlashCommand {
        name: "/approvals",
        args: " [suggest|auto-edit|full-auto]",
        summary: "tool approval mode (Codex-style)",
    },
    SlashCommand {
        name: "/context",
        args: "",
        summary: "show context / session usage estimate",
    },
    SlashCommand {
        name: "/compact",
        args: " [fast]",
        summary: "compact context (LLM summary; fast=rule-only)",
    },
    SlashCommand {
        name: "/export",
        args: " [path]",
        summary: "export session messages to JSON",
    },
    SlashCommand {
        name: "/history",
        args: " [n]",
        summary: "show last n messages in session",
    },
    SlashCommand {
        name: "/team",
        args: " <goal>",
        summary: "multi-agent pipeline (architect→code→review→test)",
    },
    SlashCommand {
        name: "/hooks",
        args: " init",
        summary: "PreToolUse hooks (.nexus/hooks.toml)",
    },
    SlashCommand {
        name: "/plugins",
        args: " install <id>",
        summary: "install marketplace plugin scaffold",
    },
    SlashCommand {
        name: "/fork",
        args: "",
        summary: "fork session — copy history to new session",
    },
    SlashCommand {
        name: "/plugins",
        args: "",
        summary: "plugin marketplace browser (TUI)",
    },
    SlashCommand {
        name: "/collab",
        args: "",
        summary: "workspace collaboration / lock status",
    },
    SlashCommand {
        name: "/sandbox",
        args: " [local|docker]",
        summary: "shell isolation mode (Docker needs daemon)",
    },
    SlashCommand {
        name: "/vector-index",
        args: "",
        summary: "Chroma semantic index for workspace",
    },
    SlashCommand {
        name: "/sync",
        args: "",
        summary: "export collab snapshot to .nexus/sync/",
    },
    SlashCommand {
        name: "/profile",
        args: "",
        summary: "engine & database performance stats",
    },
    SlashCommand {
        name: "/stats",
        args: "",
        summary: "alias for /profile",
    },
    SlashCommand {
        name: "/resume",
        args: "",
        summary: "resume pinned session for this project",
    },
    SlashCommand {
        name: "/new",
        args: "",
        summary: "start a new chat session",
    },
    SlashCommand {
        name: "/session",
        args: " <uuid>",
        summary: "switch to session by id",
    },
    SlashCommand {
        name: "/skills",
        args: " sync",
        summary: "import skills from ~/.cursor, ~/.claude, ~/.codex",
    },
    SlashCommand {
        name: "/skills",
        args: " discover",
        summary: "list discoverable external skills",
    },
    SlashCommand {
        name: "/approve",
        args: "",
        summary: "auto-approve all tools this session",
    },
    SlashCommand {
        name: "/provider",
        args: "",
        summary: "show active provider (use: nexus provider list)",
    },
    SlashCommand {
        name: "/exit",
        args: "",
        summary: "quit REPL",
    },
    SlashCommand {
        name: "/quit",
        args: "",
        summary: "quit REPL",
    },
];

pub fn all_completion_candidates() -> Vec<String> {
    let mut v: Vec<String> = COMMANDS
        .iter()
        .map(|c| format!("{}{}", c.name, c.args))
        .collect();
    v.push("/skills install ".to_string());
    v.push("/skills sync".to_string());
    v.push("/skills discover".to_string());
    v.push("/session ".to_string());
    v.sort();
    v.dedup();
    v
}

pub fn bundled_skill_names() -> &'static [&'static str] {
    &["code-review", "rust-helper"]
}
