# NexusIDE CLI smoke test (engine must be running for chat steps)
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot\..

Write-Host "== nexus help ==" -ForegroundColor Cyan
cargo run -p nexus-cli -- --help | Out-Null

Write-Host "== engine status ==" -ForegroundColor Cyan
cargo run -p nexus-cli -- engine status

Write-Host "== core tests ==" -ForegroundColor Cyan
cargo test -p nexus-core -q

Write-Host "== cli unit tests ==" -ForegroundColor Cyan
cargo test -p nexus-cli -q

$engineOk = cargo run -p nexus-cli -- engine status 2>&1 | Select-String "online"
if (-not $engineOk) {
    Write-Host "Engine offline — start with: nexus engine start" -ForegroundColor Yellow
    Write-Host "Skipping chat smoke." -ForegroundColor Yellow
    exit 0
}

Write-Host "== chat -p smoke ==" -ForegroundColor Cyan
cargo run -p nexus-cli -- chat -p "Reply with exactly: NexusIDE OK" --yes

Write-Host "Smoke finished." -ForegroundColor Green
