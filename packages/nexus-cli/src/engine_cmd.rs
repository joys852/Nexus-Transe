//! Engine lifecycle: status, start, wait for health.

use nexus_core::engine::HttpEngineClient;
use nexus_core::sync::EngineClient;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

fn is_engine_root(p: &Path) -> bool {
    p.join("pyproject.toml").is_file() && p.join("nexus_engine").is_dir()
}

fn push_candidate(out: &mut Vec<PathBuf>, p: PathBuf) {
    if is_engine_root(&p) && !out.iter().any(|x| x == &p) {
        out.push(p);
    }
}

/// Locate Python engine package (portable install, dev repo, or env override).
pub fn discover_engine_package() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(p) = std::env::var("NEXUS_ENGINE_DIR") {
        push_candidate(&mut candidates, PathBuf::from(p));
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            push_candidate(&mut candidates, exe_dir.join("engine"));
            push_candidate(&mut candidates, exe_dir.join("nexus-engine"));
            if let Some(parent) = exe_dir.parent() {
                push_candidate(&mut candidates, parent.join("engine"));
                push_candidate(&mut candidates, parent.join("share").join("nexus-engine"));
                push_candidate(&mut candidates, parent.join("packages").join("nexus-engine"));
            }
            if let Some(grand) = exe_dir.parent().and_then(|p| p.parent()) {
                push_candidate(
                    &mut candidates,
                    grand.join("packages").join("nexus-engine"),
                );
            }
        }
    }

    if let Ok(mut dir) = std::env::current_dir() {
        for _ in 0..8 {
            push_candidate(&mut candidates, dir.join("packages").join("nexus-engine"));
            push_candidate(&mut candidates, dir.join("nexus-engine"));
            if !dir.pop() {
                break;
            }
        }
    }

    candidates.into_iter().next()
}

fn venv_engine_entrypoint(engine_dir: &Path) -> Option<PathBuf> {
    let venv = engine_dir.join(".venv");
    #[cfg(windows)]
    {
        let script = venv.join("Scripts").join("nexus-engine.exe");
        if script.is_file() {
            return Some(script);
        }
        let py = venv.join("Scripts").join("python.exe");
        if py.is_file() {
            return Some(py);
        }
    }
    #[cfg(not(windows))]
    {
        let script = venv.join("bin").join("nexus-engine");
        if script.is_file() {
            return Some(script);
        }
        let py = venv.join("bin").join("python");
        if py.is_file() {
            return Some(py);
        }
    }
    None
}

pub fn start_engine_detached(engine_dir: &Path) -> anyhow::Result<()> {
    if let Some(entry) = venv_engine_entrypoint(engine_dir) {
        let mut cmd = Command::new(&entry);
        cmd.current_dir(engine_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(not(windows))]
        if entry.file_name().is_some_and(|n| n == "python") {
            cmd.arg("-m").arg("nexus_engine.api.server");
        }
        cmd.spawn()?;
        return Ok(());
    }
    let uv = which_uv()?;
    Command::new(&uv)
        .args(["run", "nexus-engine"])
        .current_dir(engine_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

fn which_uv() -> anyhow::Result<String> {
    if Command::new("uv").arg("--version").output().is_ok() {
        return Ok("uv".into());
    }
    anyhow::bail!(
        "`uv` not found and no engine `.venv` under NEXUS_ENGINE_DIR.\n\
         Install uv: https://docs.astral.sh/uv/  then run: uv sync --directory <engine>"
    )
}

pub async fn wait_for_health(url: &str, timeout: Duration) -> anyhow::Result<bool> {
    let client = HttpEngineClient::new(url);
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if client.health().await.unwrap_or(false) {
            return Ok(true);
        }
        tokio::time::sleep(Duration::from_millis(400)).await;
    }
    Ok(false)
}

/// True when `/health` returns ok; connection errors count as offline (not fatal).
pub async fn is_engine_online(url: &str) -> bool {
    HttpEngineClient::new(url)
        .health()
        .await
        .unwrap_or(false)
}

pub async fn ensure_engine(url: &str, auto_start: bool) -> anyhow::Result<()> {
    if is_engine_online(url).await {
        return Ok(());
    }
    if !auto_start {
        anyhow::bail!(
            "engine offline at {url}\n\
             start: nexus engine start\n\
             or set NEXUS_ENGINE_DIR to your engine folder and ensure .venv exists"
        );
    }
    let dir = discover_engine_package().ok_or_else(|| {
        anyhow::anyhow!(
            "engine offline — set NEXUS_ENGINE_DIR to the nexus-engine directory,\n\
             or install with scripts/install.ps1 (see docs/DISTRIBUTION.md)"
        )
    })?;
    crate::ui::print_info(format!("starting engine in {} …", dir.display()));
    start_engine_detached(&dir)?;
    if !wait_for_health(url, Duration::from_secs(45)).await? {
        anyhow::bail!("engine did not become healthy within 45s at {url}");
    }
    crate::ui::print_success("engine online");
    Ok(())
}
