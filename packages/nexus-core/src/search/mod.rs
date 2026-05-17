//! Codebase search — ripgrep-backed with gitignore respect.

use grep_regex::RegexMatcher;
use grep_searcher::sinks::UTF8;
use grep_searcher::Searcher;
use std::path::Path;

use crate::project::ProjectIndexer;
use crate::Result;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchMatch {
    pub path: String,
    pub line: u64,
    pub text: String,
}

pub fn search_codebase(root: &Path, pattern: &str, max_results: usize) -> Result<Vec<SearchMatch>> {
    let indexer = ProjectIndexer::new(root)?;
    let files: Vec<_> = indexer
        .walk(root)?
        .into_iter()
        .map(|f| root.join(&f.path))
        .collect();

    let matcher = RegexMatcher::new(pattern).map_err(|e| {
        crate::NexusError::Other(anyhow::anyhow!("invalid pattern: {e}"))
    })?;
    let mut searcher = Searcher::new();
    let mut matches = Vec::new();

    for path in files {
        if matches.len() >= max_results {
            break;
        }
        let mut sink = UTF8(|line_num, line| {
            if matches.len() < max_results {
                matches.push(SearchMatch {
                    path: path
                        .strip_prefix(root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/"),
                    line: line_num as u64,
                    text: line.trim().to_string(),
                });
            }
            Ok(true)
        });
        let _ = searcher.search_path(&matcher, &path, &mut sink);
    }
    Ok(matches)
}
