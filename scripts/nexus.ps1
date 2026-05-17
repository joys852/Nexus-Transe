# CLI launcher (repo root). Usage: .\scripts\nexus.ps1  |  .\scripts\nexus.ps1 run "task"
$Root = Split-Path -Parent $PSScriptRoot
$Nexus = Join-Path $Root "target\debug\nexus.exe"
if (-not (Test-Path $Nexus)) {
    Write-Host "Building nexus-cli..." -ForegroundColor Yellow
    Push-Location $Root
    cargo build -p nexus-cli -q
    Pop-Location
}
if ($args.Count -eq 0) {
    & $Nexus
} else {
    & $Nexus @args
}
