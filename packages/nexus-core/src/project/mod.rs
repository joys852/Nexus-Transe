//! Project root detection, instruction markdown, and incremental indexing.

pub mod instructions;

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use std::sync::Arc;

use crate::Result;

/// Passed into tool execution for path resolution and approvals.
#[derive(Clone, Debug)]
pub struct WorkspaceToolContext {
    pub project: Arc<ProjectContext>,
    pub auto_approve: bool,
    pub engine_url: String,
    /// `local` | `docker` — shell isolation for `run_shell`
    pub sandbox_mode: String,
}

impl WorkspaceToolContext {
    pub fn resolve(&self, path: &str) -> PathBuf {
        self.project.resolve_path(path)
    }

    pub fn ensure_in_workspace(&self, path: &Path) -> Result<()> {
        if !self.project.is_under_root(path) {
            return Err(crate::NexusError::ToolDenied {
                reason: format!("path outside workspace: {}", path.display()),
            });
        }
        Ok(())
    }
}

const PROJECT_MARKERS: &[&str] = &[
    "PROJECT.md",
    "CLAUDE.md",
    ".git",
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
];

#[derive(Debug, Clone)]
pub struct ProjectContext {
    pub root: PathBuf,
    /// Merged instruction markdown (CLAUDE.md, PROJECT.md, …).
    pub project_md: Option<String>,
    pub instructions: instructions::InstructionBundle,
    pub name: Option<String>,
}

impl ProjectContext {
    pub fn detect(from: impl AsRef<Path>) -> Result<Option<Self>> {
        let start = from.as_ref().canonicalize().unwrap_or_else(|_| from.as_ref().to_path_buf());
        let Some(root) = detect_root(&start) else {
            return Ok(None);
        };
        let instructions = instructions::load_instructions(&root)?;
        let project_md = if instructions.merged.is_empty() {
            None
        } else {
            Some(instructions.merged.clone())
        };
        let name = root.file_name().and_then(|s| s.to_str()).map(String::from);
        Ok(Some(Self {
            root,
            project_md,
            instructions,
            name,
        }))
    }

    pub fn resolve_path(&self, path: &str) -> PathBuf {
        let p = Path::new(path);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.root.join(p)
        }
    }

    pub fn is_under_root(&self, path: &Path) -> bool {
        path.canonicalize()
            .ok()
            .zip(self.root.canonicalize().ok())
            .map(|(p, r)| p.starts_with(&r))
            .unwrap_or_else(|| path.starts_with(&self.root))
    }
}

pub fn detect_root(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        for marker in PROJECT_MARKERS {
            if dir.join(marker).exists() {
                return Some(dir);
            }
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

/// Reload instruction markdown from disk (after `/init` or edits).
pub fn reload_instructions(ctx: &mut ProjectContext) -> Result<()> {
    ctx.instructions = instructions::load_instructions(&ctx.root)?;
    ctx.project_md = if ctx.instructions.merged.is_empty() {
        None
    } else {
        Some(ctx.instructions.merged.clone())
    };
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexedFile {
    pub path: String,
    pub hash: String,
    pub size: u64,
}

pub struct ProjectIndexer {
    gitignore: Gitignore,
}

impl ProjectIndexer {
    pub fn new(root: &Path) -> Result<Self> {
        let mut builder = GitignoreBuilder::new(root);
        let gi_path = root.join(".gitignore");
        if gi_path.is_file() {
            builder.add(gi_path);
        }
        let gitignore = builder.build()?;
        Ok(Self { gitignore })
    }

    pub fn walk(&self, root: &Path) -> Result<Vec<IndexedFile>> {
        let mut files = Vec::new();
        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !entry.file_type().is_file() {
                continue;
            }
            if path
                .components()
                .any(|c| c.as_os_str() == ".git" || c.as_os_str() == "node_modules" || c.as_os_str() == "target")
            {
                continue;
            }
            let rel = path.strip_prefix(root).unwrap_or(path);
            if self.gitignore.matched(rel, false).is_ignore() {
                continue;
            }
            let meta = std::fs::metadata(path)?;
            let bytes = std::fs::read(path)?;
            let hash = format!("{:x}", Sha256::digest(&bytes));
            files.push(IndexedFile {
                path: rel.to_string_lossy().replace('\\', "/"),
                hash,
                size: meta.len(),
            });
        }
        Ok(files)
    }
}
