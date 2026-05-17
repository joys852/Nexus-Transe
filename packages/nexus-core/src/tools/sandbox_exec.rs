//! Shell execution modes: local, Docker-isolated.

use std::path::Path;
use std::process::Output;

use crate::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxMode {
    Local,
    Docker,
}

pub fn detect_sandbox_mode() -> SandboxMode {
    match std::env::var("NEXUS_SANDBOX")
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "docker" | "container" => SandboxMode::Docker,
        _ => SandboxMode::Local,
    }
}

pub fn sandbox_mode_from_config(value: &str) -> SandboxMode {
    match value.to_lowercase().as_str() {
        "docker" | "container" => SandboxMode::Docker,
        _ => SandboxMode::Local,
    }
}

/// Config file value, overridden by `NEXUS_SANDBOX` when set.
pub fn effective_sandbox_mode(config_value: &str) -> SandboxMode {
    if let Ok(v) = std::env::var("NEXUS_SANDBOX") {
        if !v.is_empty() {
            return sandbox_mode_from_config(&v);
        }
    }
    sandbox_mode_from_config(config_value)
}

/// Run shell command in the configured sandbox.
pub async fn run_shell(command: &str, cwd: &Path, mode: SandboxMode) -> Result<Output> {
    match mode {
        SandboxMode::Local => run_local_shell(command, cwd).await,
        SandboxMode::Docker => run_docker_shell(command, cwd).await,
    }
}

async fn run_local_shell(command: &str, cwd: &Path) -> Result<Output> {
    #[cfg(windows)]
    let output = tokio::process::Command::new("cmd")
        .args(["/C", command])
        .current_dir(cwd)
        .output()
        .await?;
    #[cfg(not(windows))]
    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .output()
        .await?;
    Ok(output)
}

async fn run_docker_shell(command: &str, cwd: &Path) -> Result<Output> {
    let mount = cwd
        .canonicalize()
        .unwrap_or_else(|_| cwd.to_path_buf());
    let mount_s = mount.to_string_lossy().replace('\\', "/");
    let image = std::env::var("NEXUS_DOCKER_IMAGE").unwrap_or_else(|_| "alpine:3.19".into());
    let output = tokio::process::Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "none",
            "-v",
            &format!("{mount_s}:/work"),
            "-w",
            "/work",
            &image,
            "sh",
            "-c",
            command,
        ])
        .output()
        .await
        .map_err(|e| {
            crate::NexusError::Other(anyhow::anyhow!(
                "docker sandbox failed (is Docker running?): {e}"
            ))
        })?;
    Ok(output)
}
