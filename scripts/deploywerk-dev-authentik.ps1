# Start Docker Compose with the Authentik profile (Postgres for DeployWerk + Authentik stack).
# Run from repo root:  .\scripts\deploywerk-dev-authentik.ps1
# Then start the API and web separately (e.g. cargo run -p deploywerk-api, npm run dev in web/).

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

Write-Host "Starting docker compose --profile authentik up -d ..."
docker compose --profile authentik up -d

Write-Host "Waiting for DeployWerk Postgres..."
$deadline = (Get-Date).AddSeconds(60)
while ((Get-Date) -lt $deadline) {
    $r = docker compose exec -T postgres pg_isready -U deploywerk -d deploywerk 2>$null
    if ($LASTEXITCODE -eq 0) { break }
    Start-Sleep -Seconds 1
}

Write-Host "Waiting for Authentik live health (http://127.0.0.1:9000/-/health/live/)..."
$deadline = (Get-Date).AddSeconds(120)
$ok = $false
while ((Get-Date) -lt $deadline) {
    try {
        $resp = Invoke-WebRequest -Uri "http://127.0.0.1:9000/-/health/live/" -UseBasicParsing -TimeoutSec 3
        if ($resp.StatusCode -eq 200) { $ok = $true; break }
    } catch {
        Start-Sleep -Seconds 2
    }
}

if (-not $ok) {
    Write-Warning "Authentik did not respond with HTTP 200 yet. Check: docker compose --profile authentik logs authentik-server"
}

Write-Host ""
Write-Host "Up:"
Write-Host "  DeployWerk DB: postgresql://deploywerk:deploywerk@127.0.0.1:5432/deploywerk"
Write-Host "  Authentik:     http://127.0.0.1:9000/if/admin/"
Write-Host "Configure AUTHENTIK_* in .env and restart deploywerk-api. See README.md."
