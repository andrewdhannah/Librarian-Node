<#
.SYNOPSIS
  Stop the local llama.cpp runtime.

.DESCRIPTION
  Stops the llama.cpp background job started by start-runtime.ps1.
  Also attempts graceful shutdown via the health endpoint.

.EXAMPLE
  .\scripts\stop-runtime.ps1
#>

$job = Get-Job -Name "llama.cpp" -ErrorAction SilentlyContinue

if (-not $job) {
  Write-Host "No running llama.cpp job found." -ForegroundColor Yellow
  exit 0
}

Write-Host "Stopping llama.cpp (Job Id=$($job.Id))..." -ForegroundColor Cyan

# Try graceful shutdown via API
$configPath = "G:\openwork\librarian-runtime-node\config\runtime-node.local.json"
if (Test-Path $configPath) {
  $config = Get-Content $configPath | ConvertFrom-Json
  $shutdownUri = "http://$($config.llama.host):$($config.llama.port)/shutdown"
  try {
    Invoke-WebRequest -Uri $shutdownUri -Method POST -TimeoutSec 5 -ErrorAction Stop | Out-Null
    Write-Host "Graceful shutdown requested." -ForegroundColor Green
  } catch {
    Write-Host "Graceful shutdown failed (OK if server already stopped): $_" -ForegroundColor DarkGray
  }
}

# Force stop the job
Stop-Job -Job $job
Remove-Job -Job $job

Write-Host "llama.cpp stopped." -ForegroundColor Green
