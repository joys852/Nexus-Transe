# Build production distribution zip for Nexus-Transe.
# Usage:
#   .\scripts\package-release.ps1
#   .\scripts\package-release.ps1 -Install
#   .\scripts\package-release.ps1 -Version 1.0.0

param(
    [string]$Version = "",
    [switch]$Install,
    [switch]$SkipTests
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot

if (-not $Version) {
    $Version = (Get-Content (Join-Path $Root "VERSION") -Raw).Trim()
}

$Os = "windows"
$Arch = if ([Environment]::Is64BitOperatingSystem) { "x64" } else { "x86" }
$Name = "Nexus-Transe-$Version-$Os-$Arch"
$Dist = Join-Path $Root "dist"
$Stage = Join-Path $Dist $Name

Write-Host "Nexus-Transe production package v$Version" -ForegroundColor Cyan

if (-not $SkipTests) {
    Write-Host "Running tests..." -ForegroundColor Yellow
    Push-Location $Root
    cargo test -p nexus-core -q
    if ($LASTEXITCODE -ne 0) { Pop-Location; exit $LASTEXITCODE }
    cargo test -p nexus-cli -q
    if ($LASTEXITCODE -ne 0) { Pop-Location; exit $LASTEXITCODE }
    Pop-Location
}

Write-Host "Building release CLI..." -ForegroundColor Yellow
Push-Location $Root
cargo build -p nexus-cli --release
if ($LASTEXITCODE -ne 0) { Pop-Location; exit $LASTEXITCODE }
Pop-Location

if (-not (Get-Command uv -ErrorAction SilentlyContinue)) {
    throw "uv is required to bundle engine — https://docs.astral.sh/uv/"
}

Remove-Item $Stage -Recurse -Force -ErrorAction SilentlyContinue
$BinDir = Join-Path $Stage "bin"
$EngineDir = Join-Path $Stage "engine"
$DocsDir = Join-Path $Stage "docs"
$AssetsDir = Join-Path $Stage "assets"
New-Item -ItemType Directory -Force -Path $BinDir, $EngineDir, $DocsDir, $AssetsDir | Out-Null

Copy-Item (Join-Path $Root "target\release\nexus.exe") (Join-Path $BinDir "nexus.exe") -Force
Copy-Item (Join-Path $BinDir "nexus.exe") (Join-Path $BinDir "nx.exe") -Force

$SrcEngine = Join-Path $Root "packages\nexus-engine"
robocopy $SrcEngine $EngineDir /E /XD .pytest_cache __pycache__ .venv dist /NFL /NDL /NJH /NJS /nc /ns /np | Out-Null
if ($LASTEXITCODE -ge 8) { throw "robocopy engine failed: $LASTEXITCODE" }

Write-Host "Syncing engine venv (bundled for offline target machines)..." -ForegroundColor Yellow
uv sync --directory $EngineDir --extra dev
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$template = Get-Content (Join-Path $Root "scripts\nexus.cmd.template") -Raw
$template.Replace("__ENGINE_DIR__", $EngineDir) | Set-Content (Join-Path $BinDir "nexus.cmd") -Encoding ASCII
Copy-Item (Join-Path $BinDir "nexus.cmd") (Join-Path $BinDir "nx.cmd") -Force

foreach ($f in @("LICENSE", "NOTICE", "README.md", "VERSION", "CHANGELOG.md")) {
    Copy-Item (Join-Path $Root $f) $Stage -Force -ErrorAction SilentlyContinue
}
Copy-Item (Join-Path $Root "assets\logo.png") (Join-Path $AssetsDir "logo.png") -Force
foreach ($d in @("INSTALL.md", "CLI.md", "DISTRIBUTION.md", "PRODUCTION.md", "RELEASE.md", "known-issues.md")) {
    Copy-Item (Join-Path $Root "docs\$d") $DocsDir -Force -ErrorAction SilentlyContinue
}

$configExample = @"
engine_url = "http://127.0.0.1:8765"
default_model = "gpt-4o-mini"
# Set OPENAI_API_KEY or use: nexus provider init
"@
$configExample | Set-Content (Join-Path $Stage "config.example.toml") -Encoding UTF8

$ZipPath = Join-Path $Dist "$Name.zip"
if (Test-Path $ZipPath) { Remove-Item $ZipPath -Force }
Compress-Archive -Path $Stage -DestinationPath $ZipPath -Force

$SizeMb = [math]::Round((Get-Item $ZipPath).Length / 1MB, 1)
Write-Host ""
Write-Host "Package ready:" -ForegroundColor Green
Write-Host "  $ZipPath  ($SizeMb MB)"
Write-Host "  Unzip and run: bin\nexus.cmd"

if ($Install) {
    & (Join-Path $Root "scripts\install.ps1") -SkipBuild
    Copy-Item (Join-Path $BinDir "nexus.exe") "$env:LOCALAPPDATA\NexusIDE\bin\" -Force
    Copy-Item (Join-Path $BinDir "nx.exe") "$env:LOCALAPPDATA\NexusIDE\bin\" -Force
    Copy-Item (Join-Path $BinDir "nexus.cmd") "$env:LOCALAPPDATA\NexusIDE\bin\" -Force
    Write-Host "  Installed CLI to %LOCALAPPDATA%\NexusIDE\bin" -ForegroundColor Green
}
