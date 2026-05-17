//! Format tool outputs for terminal display (ROADMAP v2 §4.2).

use colored::Colorize;
use serde_json::Value;

const PREVIEW_LINES: usize = 24;
const PREVIEW_CHARS: usize = 2000;

pub fn format_tool_output(tool_name: &str, output: &Value) -> String {
    if tool_name == "git_diff" {
        if let Some(text) = output.get("diff").and_then(|v| v.as_str()) {
            return crate::diff::render_git_diff(text);
        }
        if let Some(text) = output.get("output").and_then(|v| v.as_str()) {
            return crate::diff::render_git_diff(text);
        }
    }
    if tool_name == "edit_file" {
        if let (Some(path), Some(old), Some(new)) = (
            output.get("path").and_then(|v| v.as_str()),
            output.get("old_string").and_then(|v| v.as_str()),
            output.get("new_string").and_then(|v| v.as_str()),
        ) {
            return crate::diff::render_unified(old, new, Some(path));
        }
    }
    if tool_name == "read_file" {
        if let Some(content) = output.get("content").and_then(|v| v.as_str()) {
            return format_file_preview(content);
        }
    }
    if tool_name == "glob_files" {
        if let Some(files) = output.get("files").and_then(|v| v.as_array()) {
            let lines: Vec<String> = files
                .iter()
                .filter_map(|f| f.as_str().map(String::from))
                .take(30)
                .collect();
            let mut s = lines.join("\n");
            if files.len() > 30 {
                s.push_str(&format!("\n  … and {} more", files.len() - 30));
            }
            return s;
        }
    }
    if tool_name.starts_with("mcp_") {
        return format_mcp_result(output);
    }
    truncate_json(output)
}

fn format_file_preview(content: &str) -> String {
    let lines: Vec<&str> = content.lines().take(PREVIEW_LINES).collect();
    let numbered: Vec<String> = lines
        .iter()
        .enumerate()
        .map(|(i, l)| format!("{:>4} │ {}", i + 1, l))
        .collect();
    let mut out = numbered.join("\n");
    if content.lines().count() > PREVIEW_LINES {
        out.push_str(&format!(
            "\n  {}",
            format!("… {} more lines", content.lines().count() - PREVIEW_LINES).dimmed()
        ));
    }
    out
}

fn format_mcp_result(output: &Value) -> String {
    if let Some(content) = output.get("content").and_then(|v| v.as_array()) {
        let mut parts = Vec::new();
        for block in content {
            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                parts.push(text.to_string());
            }
        }
        if !parts.is_empty() {
            return truncate_str(&parts.join("\n"));
        }
    }
    truncate_json(output)
}

fn truncate_json(v: &Value) -> String {
    let s = serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string());
    truncate_str(&s)
}

fn truncate_str(s: &str) -> String {
    if s.len() <= PREVIEW_CHARS {
        return s.to_string();
    }
    format!(
        "{}\n  {}",
        &s[..PREVIEW_CHARS],
        format!("… {} more chars", s.len() - PREVIEW_CHARS).dimmed()
    )
}
