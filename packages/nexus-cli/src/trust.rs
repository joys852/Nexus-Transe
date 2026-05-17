//! Workspace trust (project-local tools / hooks).

use std::fs;
use std::path::{Path, PathBuf};

use crate::menu;
use crate::ui;

pub fn is_trusted(data_dir: &Path, workspace: &Path) -> bool {
    let key = normalize_path(workspace);
    load_list(data_dir)
        .map(|list| list.iter().any(|p| p == &key))
        .unwrap_or(false)
}

pub fn mark_trusted(data_dir: &Path, workspace: &Path) -> anyhow::Result<()> {
    let key = normalize_path(workspace);
    let mut list = load_list(data_dir).unwrap_or_default();
    if !list.contains(&key) {
        list.push(key);
    }
    save_list(data_dir, &list)?;
    Ok(())
}

pub fn prompt_trust_workspace(data_dir: &Path, workspace: &Path) -> anyhow::Result<bool> {
    if is_trusted(data_dir, workspace) {
        return Ok(true);
    }

    let header = format!("You are in {}", ui::short_path(workspace));
    let body = "Do you trust the contents of this directory? Working with untrusted contents \
comes with higher risk of prompt injection. Trusting the directory allows project-local \
config, hooks, and exec policies to load.";

    let yes = menu::confirm_yes_no(&header, body, true)?;
    if yes {
        mark_trusted(data_dir, workspace)?;
        println!();
        ui::print_success("trusted this workspace");
    }
    Ok(yes)
}

fn trust_file(data_dir: &Path) -> PathBuf {
    data_dir.join("trusted_workspaces.json")
}

fn load_list(data_dir: &Path) -> anyhow::Result<Vec<String>> {
    let path = trust_file(data_dir);
    if !path.exists() {
        return Ok(vec![]);
    }
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw).unwrap_or_default())
}

fn save_list(data_dir: &Path, list: &[String]) -> anyhow::Result<()> {
    fs::create_dir_all(data_dir)?;
    fs::write(trust_file(data_dir), serde_json::to_string_pretty(list)?)?;
    Ok(())
}

fn normalize_path(p: &Path) -> String {
    let s = fs::canonicalize(p)
        .unwrap_or_else(|_| p.to_path_buf())
        .to_string_lossy()
        .to_string();
    s.strip_prefix(r"\\?\")
        .unwrap_or(&s)
        .replace('/', "\\")
        .to_lowercase()
}
