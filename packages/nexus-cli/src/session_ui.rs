//! Per-turn timing and a single updating activity line (Cursor / Claude Code style).

use std::io::Write;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use colored::Colorize;

/// Terminal content width for rules and activity bars (matches `ui::W` minus margins).
pub const TURN_W: usize = 72;

static ACTIVITY: Mutex<Option<String>> = Mutex::new(None);

pub struct TurnTimer {
    started: Instant,
    label: String,
}

impl TurnTimer {
    pub fn start(label: impl Into<String>) -> Self {
        let label = label.into();
        set_activity(&label);
        Self {
            started: Instant::now(),
            label,
        }
    }

    pub fn finish(self) {
        crate::profiler::record_turn(&self.label, self.started.elapsed());
        clear_activity();
        print_worked(self.started.elapsed(), &self.label);
    }

    pub fn finish_cancelled(self) {
        clear_activity();
        print_turn_rule(
            &format!("Stopped · {}", format_duration(self.started.elapsed())),
            false,
        );
    }
}

/// Update the single in-place activity line (replaces prior phase: chat → thinking → tools).
pub fn set_activity(label: &str) {
    let line = format_activity_line(label);
    let mut guard = ACTIVITY.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_some() {
        print!("\r\x1b[K{line}");
    } else {
        println!();
        print!("{line}");
    }
    let _ = std::io::stdout().flush();
    *guard = Some(label.to_string());
}

#[allow(dead_code)]
pub fn print_working(label: &str) {
    set_activity(label);
}

pub fn clear_activity() {
    let mut guard = ACTIVITY.lock().unwrap_or_else(|e| e.into_inner());
    if guard.take().is_some() {
        print!("\r\x1b[K");
        let _ = std::io::stdout().flush();
    }
}

pub fn format_activity_line(label: &str) -> String {
    let t = crate::theme::active();
    let accent = "◆".truecolor(t.accent.0, t.accent.1, t.accent.2);
    let title = format!(" {label} ").truecolor(t.accent.0, t.accent.1, t.accent.2);
    let core_len = label.chars().count() + 2;
    let fill = TURN_W.saturating_sub(4 + core_len);
    format!(
        "  {accent}{}{}",
        title,
        crate::theme::border_text(&"─".repeat(fill))
    )
}

pub fn print_worked(elapsed: Duration, label: &str) {
    let dur = format_duration(elapsed);
    print_turn_rule(&format!("Worked for {dur} · {label}"), true);
}

pub fn print_turn_rule(msg: &str, success: bool) {
    let msg_len = msg.chars().count();
    let pad = TURN_W.saturating_sub(msg_len);
    let left = pad / 2;
    let right = pad - left;
    let line = format!(
        "{}{}{}",
        "─".repeat(left),
        msg,
        "─".repeat(right)
    );
    println!();
    if success {
        let t = crate::theme::active();
        println!(
            "  {}",
            line.truecolor(t.muted.0, t.muted.1, t.muted.2)
        );
    } else {
        println!("  {}", line.yellow());
    }
    println!();
}

pub fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}
