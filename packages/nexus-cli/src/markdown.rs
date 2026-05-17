//! Terminal Markdown rendering (ROADMAP v2 §4.1).

use colored::Colorize;

pub fn render(md: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    let mut code_lang = String::new();
    let lines: Vec<&str> = md.lines().collect();
    let mut i = 0usize;

    while i < lines.len() {
        let line = lines[i];

        if line.trim().starts_with("```") {
            if in_code {
                out.push_str(&format!("{}\n", "└─".dimmed()));
                in_code = false;
                code_lang.clear();
            } else {
                code_lang = line.trim().trim_start_matches('`').to_string();
                let label = if code_lang.is_empty() {
                    "code".to_string()
                } else {
                    code_lang.clone()
                };
                out.push_str(&format!(
                    "\n{}\n",
                    format!("┌─ {label} ─").truecolor(255, 200, 80).dimmed()
                ));
                in_code = true;
            }
            i += 1;
            continue;
        }

        if in_code {
            out.push_str(&format!("│ {}\n", line.dimmed()));
            i += 1;
            continue;
        }

        if line.contains('|') && i + 1 < lines.len() && is_table_sep(lines[i + 1]) {
            let (table, consumed) = parse_table(&lines[i..]);
            out.push_str(&render_table(&table));
            i += consumed;
            continue;
        }

        if let Some(stripped) = line.strip_prefix("### ") {
            out.push_str(&format!("{}\n", stripped.bold()));
        } else if let Some(stripped) = line.strip_prefix("## ") {
            out.push_str(&format!(
                "\n{}\n",
                stripped.truecolor(198, 120, 73).bold()
            ));
        } else if let Some(stripped) = line.strip_prefix("# ") {
            out.push_str(&format!(
                "\n{}\n",
                stripped.white().bold()
            ));
        } else if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            out.push_str(&format!("  {} {}\n", "•".dimmed(), rest));
        } else if let Some(rest) = strip_ordered_prefix(line) {
            out.push_str(&format!("  {} {}\n", "•".dimmed(), rest));
        } else if let Some(rest) = line.strip_prefix("> ") {
            out.push_str(&format!("  {} {}\n", "▏".dimmed(), rest.italic()));
        } else if line.trim().is_empty() {
            out.push('\n');
        } else {
            out.push_str(line);
            out.push('\n');
        }
        i += 1;
    }

    if in_code {
        out.push_str(&format!("{}\n", "└─".dimmed()));
    }
    out
}

fn is_table_sep(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') && t.contains('-')
}

fn parse_table(lines: &[&str]) -> (Vec<Vec<String>>, usize) {
    let mut rows = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i].trim();
        if !line.starts_with('|') {
            break;
        }
        if is_table_sep(line) {
            i += 1;
            continue;
        }
        let cells: Vec<String> = line
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim().to_string())
            .collect();
        rows.push(cells);
        i += 1;
    }
    (rows, i.max(1))
}

fn render_table(rows: &[Vec<String>]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut widths = vec![0usize; cols];
    for row in rows {
        for (ci, cell) in row.iter().enumerate() {
            if ci < cols {
                widths[ci] = widths[ci].max(cell.chars().count());
            }
        }
    }
    let mut out = String::new();
    out.push('\n');
    for (ri, row) in rows.iter().enumerate() {
        let mut line = String::from("  ");
        for ci in 0..cols {
            let cell = row.get(ci).map(|s| s.as_str()).unwrap_or("");
            let w = widths[ci];
            if ri == 0 {
                line.push_str(&format!("{:<w$}  ", cell.bold(), w = w));
            } else {
                line.push_str(&format!("{:<w$}  ", cell, w = w));
            }
        }
        out.push_str(line.trim_end());
        out.push('\n');
        if ri == 0 {
            let rule: String = widths
                .iter()
                .map(|w| "─".repeat(*w))
                .collect::<Vec<_>>()
                .join("  ");
            out.push_str(&format!("  {}\n", rule.dimmed()));
        }
    }
    out
}

fn strip_ordered_prefix(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let mut digits = 0usize;
    for c in trimmed.chars() {
        if c.is_ascii_digit() {
            digits += 1;
        } else {
            break;
        }
    }
    if digits == 0 {
        return None;
    }
    trimmed.get(digits..)?.strip_prefix(". ").or_else(|| trimmed.get(digits..)?.strip_prefix('.'))
}
