<#
.SYNOPSIS
  Check the health of the local llama.cpp runtime.

.DESCRIPTION
  Queries the configured health endpoint and returns status as JSON.
  Also checks that the background job is alive.

.EXAMPLE
  .\scripts\health-check.ps1
  .\scripts\health-check.ps1 | ConvertFrom-Json
#>

$configPath = "G:\openwork\librarian-runtime-node\config\runtime-node.local.json"
$result = @{
  timestamp = (Get-Date).ToString("o")
  job_alive = $false
  endpoint_reachable = $false
  status_code = $null
  response = $null
  error = $null
}

# Check job
$job = Get-Job -Name "llama.cpp" -ErrorAction SilentlyContinue
if ($job -and $job.State -eq "Running") {
  $result.job_alive = $true
} else {
  $result.error = "llama.cpp job not running"
}

# Check endpoint
if (Test-Path $configPath) {
  $config = Get-Content $configPath | ConvertFrom-Json
  $uri = $config.health.endpoint
  try {
    $resp = Invoke-WebRequest -Uri $uri -TimeoutSec $config.health.timeout_seconds -ErrorAction Stop
    $result.endpoint_reachable = $true
    $result.status_code = [int]$resp.StatusCode
    $result.response = $resp.Content
  } catch {
    $result.status_code = if ($_.Exception.Response) { [int]$_.Exception.Response.StatusCode } else { $null }
    $result.error = $_.Exception.Message
  }
} else {
  $result.error = "Config not found at $configPath"
}

$result | ConvertTo-Json -Depth 3
