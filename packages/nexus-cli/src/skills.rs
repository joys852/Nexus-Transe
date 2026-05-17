//! Project + global skills (`SKILL.md` bundles).

use std::fs;
use std::path::{Path, PathBuf};

pub struct SkillEntry {
    pub name: String,
    pub path: PathBuf,
    pub scope: &'static str,
}

pub fn global_skills_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("skills")
}

pub fn project_skills_dir(project_root: &Path) -> PathBuf {
    project_root.join(".nexus").join("skills")
}

pub fn list_skills(data_dir: &Path, project_root: &Path) -> anyhow::Result<Vec<SkillEntry>> {
    let mut out = Vec::new();
    for (dir, scope) in [
        (global_skills_dir(data_dir), "global"),
        (project_skills_dir(project_root), "project"),
    ] {
        if !dir.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let path = entry.path();
            if path.join("SKILL.md").is_file() {
                out.push(SkillEntry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    path,
                    scope,
                });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub fn install_from_path(source: &Path, data_dir: &Path, name: Option<&str>) -> anyhow::Result<PathBuf> {
    let root = if source.join("SKILL.md").is_file() {
        source.to_path_buf()
    } else if source.ends_with("SKILL.md") && source.is_file() {
        source
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid skill path"))?
            .to_path_buf()
    } else {
        anyhow::bail!("no SKILL.md in {}", source.display());
    };

    let dir_name = name
        .map(String::from)
        .or_else(|| {
            source
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "skill".into());

    let dest = global_skills_dir(data_dir).join(&dir_name);
    if dest.exists() {
        fs::remove_dir_all(&dest)?;
    }
    copy_dir_recursive(&root, &dest)?;
    Ok(dest)
}

pub fn install_bundled(name: &str, data_dir: &Path) -> anyhow::Result<PathBuf> {
    let embedded = match name {
        "code-review" => include_str!("../skills/code-review/SKILL.md"),
        "rust-helper" => include_str!("../skills/rust-helper/SKILL.md"),
        _ => anyhow::bail!("unknown bundled skill: {name}. Try: code-review, rust-helper"),
    };

    let dest = global_skills_dir(data_dir).join(name);
    fs::create_dir_all(&dest)?;
    fs::write(dest.join("SKILL.md"), embedded)?;
    Ok(dest)
}

/// Concatenate skill bodies for system prompt injection.
pub fn build_skills_context(data_dir: &Path, project_root: &Path) -> anyhow::Result<String> {
    let skills = list_skills(data_dir, project_root)?;
    if skills.is_empty() {
        return Ok(String::new());
    }
    let mut buf = String::from(
        "# Installed skills\n\n\
         (Third-party skill text may mention other products; you are still NexusIDE.)\n\n",
    );
    for s in skills {
        let text = fs::read_to_string(s.path.join("SKILL.md"))?;
        buf.push_str(&format!("## Skill: {} ({})\n\n{}\n\n", s.name, s.scope, text));
    }
    Ok(buf)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}
