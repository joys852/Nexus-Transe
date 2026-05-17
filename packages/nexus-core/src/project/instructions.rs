//! Load instruction markdown files (PROJECT.md, NEXUS.md, and common aliases).

use std::path::{Path, PathBuf};

use crate::Result;

/// Root-level instruction files, highest priority first.
pub const ROOT_INSTRUCTION_FILES: &[&str] = &[
    "CLAUDE.md",
    "PROJECT.md",
    "NEXUS.md",
    "AGENTS.md",
];

/// Optional paths relative to project root.
pub const NESTED_INSTRUCTION_PATHS: &[&str] = &[
    ".cursor/rules",
    "conductor",
    ".nexus/instructions",
];

#[derive(Debug, Clone)]
pub struct InstructionFile {
    pub label: String,
    pub path: PathBuf,
    pub content: String,
}

#[derive(Debug, Clone, Default)]
pub struct InstructionBundle {
    pub files: Vec<InstructionFile>,
    /// Merged text for LLM system prompt.
    pub merged: String,
}

impl InstructionBundle {
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

pub fn load_instructions(root: &Path) -> Result<InstructionBundle> {
    let mut files = Vec::new();

    for name in ROOT_INSTRUCTION_FILES {
        let path = root.join(name);
        if path.is_file() {
            let content = std::fs::read_to_string(&path)?;
            files.push(InstructionFile {
                label: name.to_string(),
                path,
                content,
            });
        }
    }

    for nested in NESTED_INSTRUCTION_PATHS {
        let dir = root.join(nested);
        if !dir.is_dir() {
            continue;
        }
        collect_md_in_dir(&dir, root, nested, &mut files)?;
    }

    let merged = merge_instruction_files(&files);
    Ok(InstructionBundle { files, merged })
}

fn collect_md_in_dir(
    dir: &Path,
    root: &Path,
    label_prefix: &str,
    out: &mut Vec<InstructionFile>,
) -> Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            let sub = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            collect_md_in_dir(&path, root, &format!("{label_prefix}/{sub}"), out)?;
            continue;
        }
        let is_md = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("md") || e == "mdc")
            .unwrap_or(false);
        if !is_md {
            continue;
        }
        let content = std::fs::read_to_string(&path)?;
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        out.push(InstructionFile {
            label: format!("{label_prefix}/{}", path.file_name().and_then(|s| s.to_str()).unwrap_or("")),
            path,
            content,
        });
        let _ = rel; // label uses file name; rel available for future UI
    }
    Ok(())
}

fn merge_instruction_files(files: &[InstructionFile]) -> String {
    if files.is_empty() {
        return String::new();
    }
    let mut buf = String::from("# Project instructions (loaded markdown)\n\n");
    for f in files {
        buf.push_str(&format!(
            "## File: {}\nPath: {}\n\n{}\n\n---\n\n",
            f.label,
            f.path.display(),
            f.content
        ));
    }
    buf
}

/// List discoverable `*.md` / `*.mdc` under root (for /docs, capped).
pub fn discover_markdown(root: &Path, max_files: usize) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    walk_md(root, root, &mut found, max_files)?;
    found.sort();
    Ok(found)
}

fn walk_md(
    root: &Path,
    dir: &Path,
    out: &mut Vec<PathBuf>,
    max: usize,
) -> Result<()> {
    if out.len() >= max {
        return Ok(());
    }
    if dir
        .components()
        .any(|c| {
            let s = c.as_os_str();
            s == ".git" || s == "node_modules" || s == "target" || s == ".venv"
        })
    {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            walk_md(root, &path, out, max)?;
        } else if is_instruction_md(&path) {
            out.push(path);
        }
        if out.len() >= max {
            break;
        }
    }
    Ok(())
}

pub fn is_instruction_md(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("md") || e == "mdc")
        .unwrap_or(false)
}
