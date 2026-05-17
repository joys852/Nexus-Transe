//! Unified diff display for terminal (ROADMAP v2 §4.1).

use colored::Colorize;

const MAX_LINES: usize = 40;

pub fn render_unified(old_text: &str, new_text: &str, path: Option<&str>) -> String {
    let old_lines: Vec<&str> = old_text.lines().collect();
    let new_lines: Vec<&str> = new_text.lines().collect();
    let mut out = String::new();

    let title = path.unwrap_or("changes");
    out.push_str(&format!(
        "\n{}\n",
        format!("┌─ {title} ─").truecolor(255, 200, 80).dimmed()
    ));

    let mut shown = 0usize;
    let max_i = old_lines.len().max(new_lines.len());
    for i in 0..max_i {
        if shown >= MAX_LINES {
            out.push_str(&format!(
                "│  {}\n",
                format!("… {} more lines", max_i - shown).dimmed()
            ));
            break;
        }
        let o = old_lines.get(i).copied().unwrap_or("");
        let n = new_lines.get(i).copied().unwrap_or("");
        if o == n {
            if !o.is_empty() {
                out.push_str(&format!("│    {}\n", o.dimmed()));
            }
            continue;
        }
        if !o.is_empty() {
            out.push_str(&format!("│  {} {}\n", "-".red(), o.red()));
            shown += 1;
        }
        if !n.is_empty() {
            out.push_str(&format!("│  {} {}\n", "+".green(), n.green()));
            shown += 1;
        }
    }
    out.push_str(&format!("{}\n", "└─".dimmed()));
    out
}

pub fn render_git_diff(text: &str) -> String {
    let mut out = String::new();
    let mut in_hunk = false;
    for line in text.lines().take(MAX_LINES * 2) {
        if line.starts_with("diff --git") || line.starts_with("---") || line.starts_with("+++") {
            if !in_hunk {
                out.push_str(&format!(
                    "\n{}\n",
                    format!("┌─ diff ─").truecolor(255, 200, 80).dimmed()
                ));
                in_hunk = true;
            }
            out.push_str(&format!("│  {}\n", line.dimmed()));
        } else if line.starts_with('+') && !line.starts_with("+++") {
            out.push_str(&format!("│  {}\n", line.green()));
        } else if line.starts_with('-') && !line.starts_with("---") {
            out.push_str(&format!("│  {}\n", line.red()));
        } else if line.starts_with("@@") {
            out.push_str(&format!("│  {}\n", line.truecolor(198, 120, 73)));
        } else {
            out.push_str(&format!("│  {}\n", line.dimmed()));
        }
    }
    if in_hunk {
        out.push_str(&format!("{}\n", "└─".dimmed()));
    } else {
        out.push_str(text);
    }
    out
}
