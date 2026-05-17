//! Multiline input helpers, image placeholders, slash preprocessing.

use std::path::{Path, PathBuf};

use rustyline::history::DefaultHistory;
use rustyline::{Cmd, Editor, EventHandler, KeyCode, KeyEvent, Modifiers};

/// Shift+Enter = newline; Enter submits.
pub fn bind_multiline_keys(rl: &mut Editor<crate::completer::NexusHelper, DefaultHistory>) {
    rl.bind_sequence(
        KeyEvent(KeyCode::Enter, Modifiers::NONE),
        EventHandler::Simple(Cmd::AcceptLine),
    );
    rl.bind_sequence(
        KeyEvent(KeyCode::Enter, Modifiers::SHIFT),
        EventHandler::Simple(Cmd::Newline),
    );
}

/// Expand `@path` file snippets, then `[#image]` placeholders.
pub fn expand_message(line: &str, project_root: &Path) -> String {
    let with_images = expand_image_tags(line, project_root);
    expand_at_files(&with_images, project_root)
}

/// `@filename` — inline file excerpt into the user message.
fn expand_at_files(text: &str, project_root: &Path) -> String {
    let mut rebuilt = String::new();
    for (i, token) in text.split_whitespace().enumerate() {
        if i > 0 {
            rebuilt.push(' ');
        }
        if let Some(path_bit) = token.strip_prefix('@') {
            if !path_bit.is_empty() {
                rebuilt.push_str(&resolve_file_snippet(path_bit, project_root));
                continue;
            }
        }
        rebuilt.push_str(token);
    }
    if rebuilt.is_empty() && !text.is_empty() {
        text.to_string()
    } else {
        rebuilt
    }
}

fn resolve_file_snippet(path_bit: &str, project_root: &Path) -> String {
    const MAX: usize = 12_000;
    let resolved = crate::at_resolve::resolve_at_path(path_bit, project_root);
    let full = match resolved.resolved {
        Some(p) => p,
        None if resolved.candidates.len() > 1 => {
            return crate::at_resolve::format_candidates(path_bit, &resolved.candidates);
        }
        None => {
            return format!(
                "[file not found: {}]",
                project_root.join(path_bit.trim_matches('"')).display()
            );
        }
    };
    if !full.is_file() {
        return format!("[file not found: {}]", full.display());
    }
    match std::fs::read_to_string(&full) {
        Ok(content) => {
            let excerpt = if content.len() > MAX {
                format!(
                    "{}\n… [truncated, {} bytes total]",
                    &content[..MAX],
                    content.len()
                )
            } else {
                content
            };
            format!(
                "\n\n--- file: {} ---\n{}\n--- end ---\n",
                full.display(),
                excerpt
            )
        }
        Err(e) => format!("[cannot read {}: {e}]", full.display()),
    }
}

fn expand_image_tags(text: &str, project_root: &Path) -> String {
    let mut result = text.to_string();
    while let Some(start) = result.find("[#image") {
        let rest = &result[start..];
        let end = rest.find(']').unwrap_or(rest.len());
        let tag = &rest[..end + 1];
        let replacement = if tag == "[#image]" {
            "[attached image — describe what you see or paste a file path with [#image:path]]"
                .to_string()
        } else if let Some(path_bit) = tag.strip_prefix("[#image:").and_then(|s| s.strip_suffix(']')) {
            resolve_image_attachment(path_bit.trim(), project_root)
        } else {
            tag.to_string()
        };
        result.replace_range(start..start + end + 1, &replacement);
    }
    result
}

fn resolve_image_attachment(path_bit: &str, project_root: &Path) -> String {
    let p = PathBuf::from(path_bit);
    let full = if p.is_absolute() {
        p
    } else {
        project_root.join(p)
    };
    if !full.is_file() {
        return format!("[image not found: {}]", full.display());
    }
    let meta = std::fs::metadata(&full).ok();
    let size = meta.map(|m| m.len()).unwrap_or(0);
    const MAX_B64: u64 = 400_000;
    let ext = full
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin");
    if size > 0 && size <= MAX_B64 {
        if let Ok(bytes) = std::fs::read(&full) {
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
            let mime = match ext {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "webp" => "image/webp",
                _ => "application/octet-stream",
            };
            return format!(
                "[NEXUS_VISION mime={mime} path={}]\n{b64}",
                full.display()
            );
        }
    }
    format!(
        "[Image attachment: {} ({ext}, {} bytes). File too large for inline vision; describe path only.]",
        full.display(),
        size
    )
}
