# Install Nexus CLI + engine to a fixed prefix (portable across directories).
# Usage:
#   .\scripts\install.ps1
#   .\scripts\install.ps1 -Prefix "D:\Tools\NexusIDE"
#   .\scripts\install.ps1 -AddToPath

param(
    [string]$Prefix = "$env:LOCALAPPDATA\NexusIDE",
    [switch]$AddToPath,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$BinDir = Join-Path $Prefix "bin"
$EngineDir = Join-Path $Prefix "engine"

Write-Host "Nexus-Transe install -> $Prefix" -ForegroundColor Cyan

if (-not $SkipBuild) {
    Write-Host "Building nexus-cli (release)..." -ForegroundColor Yellow
    Push-Location $Root
    cargo build -p nexus-cli --release
    if ($LASTEXITCODE -ne 0) { Pop-Location; exit $LASTEXITCODE }
    Pop-Location
}

New-Item -ItemType Directory -Force -Path $BinDir, $EngineDir | Out-Null

$Release = Join-Path $Root "target\release\nexus.exe"
if (-not (Test-Path $Release)) {
    throw "Missing $Release — run without -SkipBuild or build manually."
}
Copy-Item $Release (Join-Path $BinDir "nexus.exe") -Force
Copy-Item $Release (Join-Path $BinDir "nx.exe") -Force

Write-Host "Copying engine..." -ForegroundColor Yellow
$SrcEngine = Join-Path $Root "packages\nexus-engine"
if (-not (Test-Path (Join-Path $SrcEngine "pyproject.toml"))) {
    throw "packages/nexus-engine not found"
}
robocopy $SrcEngine $EngineDir /E /XD .pytest_cache __pycache__ .venv /NFL /NDL /NJH /NJS /nc /ns /np | Out-Null
if ($LASTEXITCODE -ge 8) { throw "robocopy failed: $LASTEXITCODE" }

if (-not (Get-Command uv -ErrorAction SilentlyContinue)) {
    Write-Host "WARNING: uv not found — install https://docs.astral.sh/uv/ then run:" -ForegroundColor Red
    Write-Host "  uv sync --directory `"$EngineDir`" --extra dev" -ForegroundColor Red
} else {
    Write-Host "Syncing engine venv (one-time, may take a few minutes)..." -ForegroundColor Yellow
    uv sync --directory $EngineDir --extra dev
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

$Wrapper = Join-Path $BinDir "nexus.cmd"
$template = Get-Content (Join-Path $Root "scripts\nexus.cmd.template") -Raw
$template.Replace("__ENGINE_DIR__", $EngineDir) | Set-Content -Path $Wrapper -Encoding ASCII

$NxWrapper = Join-Path $BinDir "nx.cmd"
@(
    "@echo off",
    "set NEXUS_ENGINE_DIR=$EngineDir",
    "%~dp0nx.exe %*"
) | Set-Content -Path $NxWrapper -Encoding ASCII

$configExample = @"
# Copy to %APPDATA%\nexus-ide\config.toml or set NEXUS_CONFIG
engine_url = "http://127.0.0.1:8765"
default_model = "gpt-4o-mini"
"@
$configExample | Set-Content (Join-Path $Prefix "config.example.toml") -Encoding UTF8

Write-Host ""
Write-Host "Installed:" -ForegroundColor Green
Write-Host "  CLI:    $BinDir\nexus.exe"
Write-Host "  Engine: $EngineDir"
Write-Host "  Wrapper (sets NEXUS_ENGINE_DIR): $Wrapper"
Write-Host ""
Write-Host "Run from ANY folder:" -ForegroundColor Green
Write-Host "  & `"$Wrapper`""
Write-Host "  # or add to PATH:" -ForegroundColor DarkGray
Write-Host "  `$env:Path = `"$BinDir;`" + `$env:Path"
Write-Host "  nexus.cmd"

if ($AddToPath) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$BinDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$BinDir;$userPath", "User")
        Write-Host "Added $BinDir to user PATH (restart terminal)." -ForegroundColor Green
    }
}
