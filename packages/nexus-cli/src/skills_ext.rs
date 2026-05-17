//! Discover skills from external agent skill directories.

use std::fs;
use std::path::{Path, PathBuf};

use crate::skills;

pub fn external_skill_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = dirs::home_dir() {
        for sub in [
            ".cursor/skills",
            ".cursor/skills-cursor",
            ".claude/skills",
            ".codex/skills",
            ".agents/skills",
            ".config/nexus/skills",
        ] {
            let p = home.join(sub);
            if p.is_dir() {
                roots.push(p);
            }
        }
    }
    roots
}

pub fn discover_external_skills() -> Vec<(String, PathBuf, String)> {
    let mut found = Vec::new();
    for root in external_skill_roots() {
        let scope = format!("external:{}", root.display());
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("SKILL.md").is_file() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    found.push((name, path, scope.clone()));
                }
            }
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

pub fn sync_external_skills(data_dir: &Path) -> anyhow::Result<Vec<String>> {
    let mut installed = Vec::new();
    for (name, path, _) in discover_external_skills() {
        let dest_name = format!("ext-{name}");
        let dest = skills::install_from_path(&path, data_dir, Some(&dest_name))?;
        installed.push(dest.display().to_string());
    }
    Ok(installed)
}
