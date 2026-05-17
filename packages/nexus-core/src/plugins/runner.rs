//! Execute installed plugins (script entry or WASM via wasmtime CLI).

use std::path::Path;
use std::process::Command;

use super::{LoadedPlugin, PluginPermission};
use crate::Result;

/// Run plugin: WASM (`wasmtime run`) when entry ends with `.wasm`, else script fallback.
pub fn run_plugin(
    plugin: &LoadedPlugin,
    workspace: &Path,
    args: &[&str],
) -> Result<String> {
    let entry = plugin.root.join(&plugin.manifest.entry);
    if entry.extension().and_then(|e| e.to_str()) == Some("wasm") && entry.is_file() {
        return run_plugin_wasm(&entry, workspace, args);
    }
    run_plugin_script(plugin, workspace, args)
}

pub fn run_plugin_wasm(wasm_path: &Path, workspace: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("wasmtime")
        .arg("run")
        .arg(wasm_path)
        .args(args)
        .current_dir(workspace)
        .output()
        .map_err(|e| {
            crate::NexusError::Other(anyhow::anyhow!(
                "wasmtime not found or failed ({e}); install: https://wasmtime.dev/"
            ))
        })?;
    Ok(format!(
        "exit={}\n{}{}",
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

/// Run plugin `run.cmd` / `run.sh` / `main.py` if present.
pub fn run_plugin_script(
    plugin: &LoadedPlugin,
    workspace: &Path,
    args: &[&str],
) -> Result<String> {
    let candidates = ["run.ps1", "run.cmd", "run.sh", "main.py"];
    let script = candidates
        .iter()
        .map(|name| plugin.root.join(name))
        .find(|p| p.is_file());
    let Some(script) = script else {
        return Ok(format!(
            "plugin {} installed at {} (no run.ps1/run.sh; set entry = main.wasm for wasmtime)",
            plugin.manifest.id,
            plugin.root.display()
        ));
    };
    let ext = script.extension().and_then(|e| e.to_str()).unwrap_or("");
    let output = match ext {
        "ps1" => Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&script)
            .args(args)
            .current_dir(workspace)
            .output(),
        "sh" => Command::new("sh")
            .arg(&script)
            .args(args)
            .current_dir(workspace)
            .output(),
        "py" => Command::new("python")
            .arg(&script)
            .args(args)
            .current_dir(workspace)
            .output(),
        _ => Command::new(&script).args(args).current_dir(workspace).output(),
    }
    .map_err(|e| crate::NexusError::Other(anyhow::anyhow!("plugin run: {e}")))?;
    Ok(format!(
        "exit={}\n{}{}",
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

pub fn permission_label(p: &PluginPermission) -> &'static str {
    match p {
        PluginPermission::ReadFiles => "read_files",
        PluginPermission::WriteFiles => "write_files",
        PluginPermission::RunShell => "run_shell",
        PluginPermission::Network => "network",
        PluginPermission::McpBridge => "mcp_bridge",
    }
}
