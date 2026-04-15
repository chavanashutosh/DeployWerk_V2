param(
  [string]$BaseUrl = "http://127.0.0.1:8080"
)

$ErrorActionPreference = "Stop"

function Invoke-Api($method, $path, $token, $body = $null) {
  $uri = "$BaseUrl$path"
  $headers = @{}
  if ($token) { $headers["Authorization"] = "Bearer $token" }
  if ($null -eq $body) {
    return Invoke-RestMethod -Method $method -Uri $uri -Headers $headers
  }
  return Invoke-RestMethod -Method $method -Uri $uri -Headers $headers -ContentType "application/json" -Body ($body | ConvertTo-Json -Depth 20)
}

Write-Host "Bringing up stack..."
docker compose up -d --build | Out-Null

Write-Host "Logging in as demo owner..."
$login = Invoke-Api "Post" "/api/v1/auth/login" $null @{ email = "owner@demo.deploywerk.local"; password = "DemoOwner1!" }
$token = $login.token

Write-Host "Fetching demo team/project/environment/app..."
$teams = Invoke-Api "Get" "/api/v1/teams" $token
$team = $teams | Where-Object { $_.slug -eq "demo" } | Select-Object -First 1
if (-not $team) { throw "Demo team not found" }
$teamId = $team.id

$projects = Invoke-Api "Get" "/api/v1/teams/$teamId/projects" $token
$proj = $projects | Where-Object { $_.slug -eq "sample" } | Select-Object -First 1
if (-not $proj) { throw "Demo project not found" }
$projectId = $proj.id

$envs = Invoke-Api "Get" "/api/v1/teams/$teamId/projects/$projectId/environments" $token
$env = $envs | Where-Object { $_.slug -eq "production" } | Select-Object -First 1
if (-not $env) { throw "Demo environment not found" }
$environmentId = $env.id

$apps = Invoke-Api "Get" "/api/v1/teams/$teamId/projects/$projectId/environments/$environmentId/applications" $token
$app = $apps | Where-Object { $_.slug -eq "hello" } | Select-Object -First 1
if (-not $app) { throw "Demo app not found" }
$appId = $app.id

Write-Host "Ensuring platform destination exists..."
$dests = Invoke-Api "Get" "/api/v1/teams/$teamId/destinations" $token
$platform = $dests | Where-Object { $_.slug -eq "platform" } | Select-Object -First 1
if (-not $platform) { throw "Platform destination not found (DEPLOYWERK_PLATFORM_DOCKER_ENABLED should be true)" }

Write-Host "Updating demo app to use platform destination + a runtime volume mount..."
$patch = @{
  destination_id = $platform.id
  runtime_volumes = @(
    @{ name = "data"; container_path = "/data" }
  )
}
Invoke-Api "Patch" "/api/v1/teams/$teamId/projects/$projectId/environments/$environmentId/applications/$appId" $token $patch | Out-Null

Write-Host "Triggering deploy..."
$deploy = Invoke-Api "Post" "/api/v1/teams/$teamId/projects/$projectId/environments/$environmentId/applications/$appId/deploy" $token @{}
$jobId = $deploy.job_id

Write-Host "Waiting for deploy job to finish..."
for ($i=0; $i -lt 60; $i++) {
  Start-Sleep -Seconds 1
  $job = Invoke-Api "Get" "/api/v1/teams/$teamId/deploy-jobs/$jobId" $token
  if ($job.status -ne "queued" -and $job.status -ne "running") { break }
}
$job = Invoke-Api "Get" "/api/v1/teams/$teamId/deploy-jobs/$jobId" $token
Write-Host ("Job status: {0}" -f $job.status)

if (-not $job.log_object_key -or -not $job.artifact_manifest_key) {
  throw "Expected job.log_object_key and job.artifact_manifest_key to be set"
}

Write-Host "Verifying objects exist in MinIO..."
docker run --rm --network deploywerkv2_default --entrypoint /bin/sh minio/mc:latest -lc "mc alias set local http://minio:9000 deploywerk deploywerk-dev-only-change-me >/dev/null && mc stat `"local/deploywerk/$($job.log_object_key)`" >/dev/null && mc stat `"local/deploywerk/$($job.artifact_manifest_key)`" >/dev/null" | Out-Null

Write-Host "E2E finished. Inspect job log in UI if status != succeeded."

