# NexusIDE local dev helpers (PowerShell)
param(
    [ValidateSet("engine", "cli", "all")]
    [string]$Target = "all"
)

$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

function Start-Engine {
    Set-Location "$Root\packages\nexus-engine"
    uv sync --extra dev
    uv run nexus-engine
}

function Build-Cli {
    Set-Location $Root
    cargo build -p nexus-cli
    Write-Host "CLI: cargo run -p nexus-cli -- engine status"
}

switch ($Target) {
    "engine" { Start-Engine }
    "cli"    { Build-Cli }
    "all"    {
        Write-Host "Terminal 1: .\scripts\dev.ps1 -Target engine"
        Write-Host "Terminal 2: cargo run -p nexus-cli -- engine status"
        Build-Cli
    }
}
