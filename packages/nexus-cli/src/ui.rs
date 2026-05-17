//! Terminal presentation — Claude Code–inspired boxed layout for Nexus CLI.

use colored::Colorize;
use std::path::{Path, PathBuf};

use crate::mode::ChatMode;

const W: usize = 76;

pub struct WelcomeInfo<'a> {
    pub version: &'a str,
    pub cwd: &'a Path,
    pub provider_name: Option<&'a str>,
    pub model: &'a str,
    pub mode: ChatMode,
    pub skill_count: usize,
    pub instruction_count: usize,
}

pub fn print_welcome(info: WelcomeInfo<'_>) {
    crate::logo::print_nexus_transformers_logo(
        info.version,
        info.model,
        info.mode,
        true,
    );
    if std::env::var("NEXUS_VERBOSE_WELCOME").is_ok() {
        print_compact_welcome(info);
    } else {
        print_startup_status(info);
    }
}

/// Dim single line after welcome when continuing a session.
pub fn print_resume_hint(session_id: &uuid::Uuid, message_count: usize) {
    let t = crate::theme::active();
    println!(
        "  {} {}",
        "↳".truecolor(t.muted.0, t.muted.1, t.muted.2),
        crate::theme::muted_text(&format!(
            "resumed {} · {message_count} message(s)",
            short_session_id(session_id)
        ))
    );
}

/// One-line status after logo (default — no extra boxes).
fn print_startup_status(info: WelcomeInfo<'_>) {
    let t = crate::theme::active();
    let provider = info
        .provider_name
        .map(|p| format!("{p} · "))
        .unwrap_or_default();
    println!(
        "  {}  ·  {}{}  ·  {}  ·  {}",
        short_path(info.cwd).truecolor(t.fg.0, t.fg.1, t.fg.2),
        provider,
        info.model.truecolor(86, 156, 214),
        info.mode.label().truecolor(t.accent.0, t.accent.1, t.accent.2),
        crate::theme::muted_text("/help · Shift+Enter newline")
    );
    println!();
}

fn print_compact_welcome(info: WelcomeInfo<'_>) {
    let title = format!(" NexusIDE v{} ", info.version);
    println!();
    top_border(&title);
    let provider = info
        .provider_name
        .map(|p| format!("{p} · "))
        .unwrap_or_default();
    println!(
        "{}",
        row_content(&format!(
            "  {} {}",
            crate::theme::label_text("workspace"),
            short_path(info.cwd).truecolor(
                crate::theme::active().fg.0,
                crate::theme::active().fg.1,
                crate::theme::active().fg.2,
            )
        ))
    );
    println!(
        "{}",
        row_content(&format!(
            "  {} {}{}  {} {}",
            crate::theme::label_text("model"),
            provider,
            info.model.truecolor(86, 156, 214),
            crate::theme::label_text("mode"),
            info.mode.label().truecolor(
                crate::theme::active().accent.0,
                crate::theme::active().accent.1,
                crate::theme::active().accent.2,
            )
        ))
    );
    println!(
        "{}",
        row_content(&format!(
            "  {} {} skills · {} instructions",
            crate::theme::label_text("ctx"),
            info.skill_count,
            info.instruction_count
        )
        .dimmed()
        .to_string())
    );
    println!(
        "{}",
        row_content(&format!(
            "  {} Shift+Enter newline · Tab · /help",
            crate::theme::muted_text("·")
        ))
    );
    bottom_border();
    print_status_line(info);
    println!();
}

fn print_status_line(info: WelcomeInfo<'_>) {
    let t = crate::theme::active();
    let bar = "─".repeat(W.saturating_sub(4));
    println!(
        "  {}{}{}",
        crate::theme::border_text("├"),
        crate::theme::border_text(&bar),
        crate::theme::border_text("┤")
    );
    let mode = format!("[{}]", info.mode.label());
    println!(
        "  {} {}  {}",
        prompt_glyph(info.mode),
        mode.truecolor(t.accent.0, t.accent.1, t.accent.2),
        crate::theme::muted_text("Ctrl+C cancel · /exit quit")
    );
}

fn prompt_glyph(mode: ChatMode) -> colored::ColoredString {
    match mode {
        ChatMode::Default => {
            let t = crate::theme::active();
            "❯".truecolor(t.fg.0, t.fg.1, t.fg.2)
        }
        ChatMode::Plan => {
            let t = crate::theme::active();
            "◎".truecolor(t.accent.0, t.accent.1, t.accent.2)
        }
        ChatMode::Agent => {
            let t = crate::theme::active();
            "⚡".truecolor(t.accent_soft.0, t.accent_soft.1, t.accent_soft.2)
        }
    }
}

pub fn hint_style(hint: &str) -> String {
    hint.truecolor(120, 120, 120).to_string()
}

pub fn print_slash_completions(rows: &[(String, String)]) {
    if rows.is_empty() {
        return;
    }
    println!();
    top_border(" Slash ");
    for (cmd, desc) in rows {
        println!(
            "{}",
            row_two(
                &format!("  {}", cmd.truecolor(86, 156, 214)),
                30,
                desc,
                W - 33,
            )
        );
    }
    bottom_border();
    println!();
}

/// Plain ASCII prompt for readline (no ANSI / wide Unicode — avoids cursor drift on Windows).
pub fn prompt(mode: ChatMode) -> &'static str {
    match mode {
        ChatMode::Default => "> ",
        ChatMode::Plan => "plan> ",
        ChatMode::Agent => "agent> ",
    }
}

/// RGB for readline `highlight_prompt` (must match `prompt()` prefix).
pub fn prompt_color(mode: ChatMode) -> (u8, u8, u8) {
    match mode {
        ChatMode::Default => {
            let t = crate::theme::active();
            (t.fg.0, t.fg.1, t.fg.2)
        }
        ChatMode::Plan => {
            let t = crate::theme::active();
            (t.accent.0, t.accent.1, t.accent.2)
        }
        ChatMode::Agent => {
            let t = crate::theme::active();
            (t.accent_soft.0, t.accent_soft.1, t.accent_soft.2)
        }
    }
}

pub fn print_mode_switch(mode: ChatMode) {
    let hint = match mode {
        ChatMode::Default => "tools enabled",
        ChatMode::Plan => "planning only, no tools",
        ChatMode::Agent => "multi-agent pipeline per message",
    };
    println!();
    println!(
        "  {} {} — {}",
        symbol_ok().truecolor(
            crate::theme::active().success.0,
            crate::theme::active().success.1,
            crate::theme::active().success.2,
        ),
        format!("mode → {}", mode.label()).truecolor(86, 156, 214),
        hint.dimmed()
    );
}

/// Light separator before assistant reply (readline already echoed `> your text`).
pub fn print_user_turn_separator() {
    let t = crate::theme::active();
    println!();
    println!(
        "  {}",
        "─".repeat(crate::session_ui::TURN_W)
            .truecolor(t.border.0, t.border.1, t.border.2)
    );
}

#[allow(dead_code)]
pub fn print_user_line(text: &str) {
    print_user_turn_separator();
    let _ = text;
}

pub fn begin_assistant() {
    crate::session_ui::clear_activity();
    let t = crate::theme::active();
    println!();
    println!(
        "  {} {}",
        symbol_assistant().truecolor(t.accent.0, t.accent.1, t.accent.2),
        "Nexus".truecolor(t.accent.0, t.accent.1, t.accent.2).bold()
    );
    print!("  ");
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

/// Stream assistant tokens with consistent left indent (no raw wall of text).
pub fn stream_assistant_delta(delta: &str) {
    let indented = delta.replace('\n', "\n  ");
    print!("{indented}");
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

pub fn end_assistant_stream() {
    println!();
}

pub fn print_agent_phase(agent: &str, preview: &str) {
    println!();
    println!(
        "  {} {}",
        crate::theme::accent_text(&format!("◆ {agent}")),
        truncate(preview, 64).dimmed()
    );
}

/// Compact JSON for tool argument display (single line when possible).
pub fn format_tool_args(value: &serde_json::Value) -> String {
    let compact = serde_json::to_string(value).unwrap_or_else(|_| value.to_string());
    if compact.chars().count() <= 72 {
        return compact;
    }
    serde_json::to_string_pretty(value).unwrap_or(compact)
}

pub fn print_tool_begin(name: &str, args: &str) {
    let t = crate::theme::active();
    let border = crate::theme::border_text;
    let args_line = args.trim();
    let name_styled = crate::theme::tool_name_text(name);
    println!();
    if args_line.contains('\n') {
        println!("    {} {} {}", border("╭─"), name_styled, border("─"));
        for line in args_line.lines() {
            println!("    {}  {}", border("│"), line.dimmed());
        }
    } else {
        println!(
            "    {} {} {} {}",
            border("╭─"),
            name_styled,
            args_line.dimmed(),
            border("─")
        );
    }
    let _ = t;
}

pub fn print_tool_done_ok() {
    let t = crate::theme::active();
    println!(
        "    {} {}",
        crate::theme::border_text("╰─"),
        crate::theme::success_text("✓ done")
    );
    let _ = t;
}

pub fn print_tool_done_fail(status: &str) {
    let t = crate::theme::active();
    println!(
        "    {} {}",
        crate::theme::border_text("╰─"),
        format!("✗ {status}").truecolor(t.error.0, t.error.1, t.error.2)
    );
}

#[allow(dead_code)]
pub fn print_tool_header(name: &str, detail: &str) {
    print_tool_begin(name, detail);
}

#[allow(dead_code)]
pub fn print_tool_ok() {
    print_tool_done_ok();
}

#[allow(dead_code)]
pub fn print_tool_fail(status: &str) {
    print_tool_done_fail(status);
}

pub fn print_error(msg: impl std::fmt::Display) {
    println!();
    println!("  {} {}", symbol_error().red(), format!("{msg}").red());
}

pub fn print_info(msg: impl std::fmt::Display) {
    println!("  {} {}", symbol_info().blue(), format!("{msg}").dimmed());
}

pub fn print_success(msg: impl std::fmt::Display) {
    println!("  {} {}", symbol_ok().green(), format!("{msg}").green());
}

pub fn print_cancelled() {
    println!();
    println!(
        "  {}",
        crate::theme::muted_text("╰─ cancelled")
    );
}

pub fn print_help() {
    println!();
    top_border(" Commands ");
    print_help_section(
        "Chat & modes",
        &["/help", "/mode", "/plan", "/agent", "/chat", "/compact", "/context", "/team"],
    );
    print_help_section(
        "Session",
        &[
            "/new",
            "/resume",
            "/fork",
            "/sessions",
            "/export",
            "/history",
            "/session",
        ],
    );
    print_help_section(
        "Project",
        &["/init", "/index", "/docs", "/vector-index", "/sandbox"],
    );
    print_help_section(
        "Tools & safety",
        &[
            "/mcp",
            "/hooks",
            "/plugins",
            "/approvals",
            "/approve",
            "/git-status",
        ],
    );
    print_help_section(
        "Skills & collab",
        &["/skills", "/collab", "/sync", "/profile", "/provider", "/theme"],
    );
    print_help_section("Exit", &["/exit", "/quit", "/cancel", "/clear"]);
    bottom_border();
    println!(
        "  {}",
        crate::theme::muted_text("Tab complete · NEXUS_VERBOSE_WELCOME=1 详细启动信息")
    );
    println!();
}

fn print_help_section(title: &str, names: &[&str]) {
    println!(
        "{}",
        row_content(&format!(
            "  {}",
            title.truecolor(
                crate::theme::active().accent.0,
                crate::theme::active().accent.1,
                crate::theme::active().accent.2,
            )
        ))
    );
    for c in crate::slash::COMMANDS {
        if !names.contains(&c.name) {
            continue;
        }
        if !c.args.is_empty() && c.args.starts_with(" install") {
            continue;
        }
        let cmd = if c.args.is_empty() {
            c.name.to_string()
        } else {
            format!("{}{}", c.name, c.args)
        };
        println!(
            "{}",
            row_two(
                &format!("    {}", cmd.truecolor(86, 156, 214)),
                30,
                c.summary,
                W - 33,
            )
        );
    }
}

pub fn print_instruction_files(rows: &[(String, String)]) {
    println!();
    top_border(" Instruction files ");
    for (label, path) in rows {
        println!(
            "{}",
            row_two(&format!("  ● {}", label.white()), 28, path, W - 31)
        );
    }
    bottom_border();
    println!();
}

pub fn print_skills(entries: &[(String, String)]) {
    println!();
    top_border(" Skills ");
    if entries.is_empty() {
        println!("{}", row_content("  (none) — try: /skills install rust-helper"));
    } else {
        for (name, scope) in entries {
            println!(
                "{}",
                row_two(
                    &format!("  ● {}", name.white()),
                    28,
                    &format!("scope: {scope}"),
                    W - 31,
                )
            );
        }
    }
    bottom_border();
    println!();
}

pub fn print_provider_table(
    active_id: Option<&str>,
    rows: &[(&str, &str, &str, &str, bool)],
) {
    println!();
    println!(
        "  {} {}",
        "Providers".bold(),
        active_id
            .map(|id| format!("· active: {}", id.cyan()))
            .unwrap_or_default()
            .dimmed()
    );
    let rule = "─".repeat(W);
    println!("  {}", rule.dimmed());
    for (id, name, protocol, endpoint, is_active) in rows {
        let mark = if *is_active { "●".green() } else { "○".dimmed() };
        println!("  {} {:<20} {}", mark, id.white(), name.dimmed());
        println!("      {} · {}", protocol.yellow(), truncate(endpoint, 56).dimmed());
    }
    println!();
}

pub fn print_collab_strip(peers: &[crate::collab::CollabPeer]) {
    println!();
    top_border(" Collaboration ");
    if peers.is_empty() {
        println!("{}", row_content("  solo · no other Nexus lock on this workspace"));
    } else {
        for peer in peers {
            let line = crate::collab::format_peer_line(peer);
            let styled = if peer.is_self {
                format!("  ● {line}").green().to_string()
            } else if peer.stale {
                format!("  ○ {line} (stale lock)").dimmed().to_string()
            } else {
                format!("  ⚠ {line} — another client may be active")
                    .truecolor(
                        crate::theme::active().accent.0,
                        crate::theme::active().accent.1,
                        crate::theme::active().accent.2,
                    )
                    .to_string()
            };
            println!("{}", row_content(&styled));
        }
    }
    bottom_border();
}

pub fn print_profile_box(text: &str) {
    println!();
    top_border(" Profile ");
    for line in text.lines() {
        println!("{}", row_content(&format!("  {line}")));
    }
    bottom_border();
    println!();
}

pub fn print_engine_status(url: &str, online: bool) {
    println!();
    if online {
        println!(
            "  {} {} {}",
            symbol_ok().green(),
            url.white(),
            "online".green()
        );
    } else {
        println!(
            "  {} {} {}",
            symbol_error().red(),
            url.white(),
            "offline".red()
        );
        println!(
            "  {}",
            "Start: cd packages/nexus-engine && uv run nexus-engine".dimmed()
        );
    }
    println!();
}

pub fn short_session_id(id: &uuid::Uuid) -> String {
    id.to_string().chars().take(8).collect()
}

pub fn short_path(path: &Path) -> String {
    let s = path.to_string_lossy();
    let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
    if s.len() <= 48 {
        return s.to_string();
    }
    PathBuf::from(s)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| format!("…/{n}"))
        .unwrap_or_else(|| format!("…{}", &s[s.len().saturating_sub(40)..]))
}

fn top_border(title: &str) {
    let inner = W - 2;
    let pad = inner.saturating_sub(title.len());
    let left = pad / 2;
    let right = pad - left;
    println!(
        "{}{}{}{}{}",
        corner_tl(),
        brand(&"─".repeat(left)),
        crate::theme::accent_text(title),
        brand(&"─".repeat(right)),
        corner_tr()
    );
}

fn bottom_border() {
    println!("{}{}{}", corner_bl(), brand(&"─".repeat(W)), corner_br());
}

#[allow(dead_code)]
fn row_split() -> String {
    let a = brand("├");
    let b = brand(&"─".repeat(28));
    let c = brand("┼");
    let d = brand(&"─".repeat(W - 31));
    let e = brand("┤");
    format!("{a}{b}{c}{d}{e}")
}

fn row_two(left: &str, lw: usize, right: &str, rw: usize) -> String {
    format!(
        "{} {} {} {}",
        brand("│"),
        pad_visible(left, lw),
        brand("│"),
        pad_visible(right, rw)
    )
}

fn row_content(s: &str) -> String {
    format!("{} {} {}", brand("│"), pad_visible(s, W - 2), brand("│"))
}

fn pad_visible(s: &str, width: usize) -> String {
    let plain = strip_ansi(s);
    let vis = plain.chars().count();
    if vis >= width {
        format!("{s}")
    } else {
        format!("{s}{}", " ".repeat(width - vis))
    }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut esc = false;
    for c in s.chars() {
        if esc {
            if c == 'm' {
                esc = false;
            }
            continue;
        }
        if c == '\x1b' {
            esc = true;
            continue;
        }
        out.push(c);
    }
    out
}

fn brand(s: &str) -> String {
    crate::theme::border_text(s)
}

fn corner_tl() -> String {
    crate::theme::accent_text("╭")
}
fn corner_tr() -> String {
    crate::theme::accent_text("╮")
}
fn corner_bl() -> String {
    crate::theme::accent_text("╰")
}
fn corner_br() -> String {
    crate::theme::accent_text("╯")
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max.saturating_sub(1)).collect::<String>())
    }
}

fn symbol_user() -> &'static str {
    "›"
}

fn symbol_assistant() -> &'static str {
    "◆"
}

fn symbol_error() -> &'static str {
    "✗"
}

fn symbol_ok() -> &'static str {
    "✓"
}

fn symbol_info() -> &'static str {
    "·"
}
