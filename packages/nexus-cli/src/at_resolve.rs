//! Fuzzy `@path` resolution (Codex-style file references).

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::fuzzy;

const MAX_CANDIDATES: usize = 5;
const MAX_FILES: usize = 8_000;

fn file_cache() -> &'static Mutex<Option<(PathBuf, Vec<String>)>> {
    static C: OnceLock<Mutex<Option<(PathBuf, Vec<String>)>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(None))
}

fn list_project_files(root: &Path) -> Vec<String> {
    let mut g = file_cache().lock().unwrap_or_else(|e| e.into_inner());
    if let Some((cached_root, files)) = g.as_ref() {
        if cached_root == root {
            return files.clone();
        }
    }
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.components().any(|c| {
            let s = c.as_os_str();
            s == ".git" || s == "node_modules" || s == "target" || s == ".venv"
        }) {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        files.push(rel);
        if files.len() >= MAX_FILES {
            break;
        }
    }
    files.sort();
    *g = Some((root.to_path_buf(), files.clone()));
    files
}

pub struct AtResolveResult {
    pub resolved: Option<PathBuf>,
    pub candidates: Vec<String>,
}

/// Resolve `@query` to a project-relative file path.
pub fn resolve_at_path(query: &str, project_root: &Path) -> AtResolveResult {
    let q = query.trim().trim_matches('"');
    if q.is_empty() {
        return AtResolveResult {
            resolved: None,
            candidates: Vec::new(),
        };
    }
    let direct = if Path::new(q).is_absolute() {
        PathBuf::from(q)
    } else {
        project_root.join(q)
    };
    if direct.is_file() {
        return AtResolveResult {
            resolved: Some(direct),
            candidates: vec![q.to_string()],
        };
    }

    let files = list_project_files(project_root);
    let mut scored: Vec<(u32, String)> = files
        .iter()
        .filter_map(|f| {
            let name = f.rsplit('/').next().unwrap_or(f);
            let score = fuzzy::score(f, q).or_else(|| fuzzy::score(name, q))?;
            Some((score, f.clone()))
        })
        .collect();
    scored.sort_by_key(|(s, _)| *s);
    let candidates: Vec<String> = scored
        .into_iter()
        .take(MAX_CANDIDATES)
        .map(|(_, p)| p)
        .collect();

    let resolved = if candidates.len() == 1 {
        Some(project_root.join(&candidates[0]))
    } else {
        candidates
            .iter()
            .find(|c| c.ends_with(q) || c.as_str() == q)
            .map(|c| project_root.join(c))
    };

    AtResolveResult {
        resolved,
        candidates,
    }
}

/// Top fuzzy path matches for `@` tab completion (relative paths).
pub fn list_candidates(query: &str, project_root: &Path, limit: usize) -> Vec<String> {
    let q = query.trim();
    if q.is_empty() {
        return list_project_files(project_root)
            .into_iter()
            .take(limit)
            .collect();
    }
    let files = list_project_files(project_root);
    let mut scored: Vec<(u32, String)> = files
        .iter()
        .filter_map(|f| {
            let name = f.rsplit('/').next().unwrap_or(f);
            let score = fuzzy::score(f, q).or_else(|| fuzzy::score(name, q))?;
            Some((score, f.clone()))
        })
        .collect();
    scored.sort_by_key(|(s, _)| *s);
    scored.into_iter().take(limit).map(|(_, p)| p).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn fuzzy_lists_readme() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("README.md"), "hi").unwrap();
        let paths = list_candidates("readme", tmp.path(), 5);
        assert!(paths.iter().any(|p| p.contains("README")));
    }
}

pub fn format_candidates(query: &str, candidates: &[String]) -> String {
    if candidates.is_empty() {
        return format!("[no files match `@{}`]", query);
    }
    let mut lines = vec![format!("@{} — pick one:", query)];
    for (i, c) in candidates.iter().enumerate() {
        lines.push(format!("  {}. {}", i + 1, c));
    }
    lines.join("\n")
}
