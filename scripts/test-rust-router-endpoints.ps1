<#
.SYNOPSIS
  Test all Rust router HTTP endpoints.

.DESCRIPTION
  Starts the Rust router, tests all endpoints, and reports pass/fail.
  Router is started and stopped within the script.

.EXAMPLE
  .\scripts\test-rust-router-endpoints.ps1
  .\scripts\test-rust-router-endpoints.ps1 -Port 9131
#>

param(
  [int]$Port = 9130,
  [string]$RouterDir = "G:\OpenWork\librarian-runtime-node\rust-router",
  [string]$BinaryPath = "G:\OpenWork\librarian-runtime-node\rust-router\target\release\rust-router.exe"
)

$BaseUrl = "http://127.0.0.1:$Port"
$Passed = 0
$Failed = 0

function Test-Step {
  param([string]$Name, [scriptblock]$Block)
  try {
    $result = & $Block
    if ($result) {
      Write-Host "  PASS: $Name" -ForegroundColor Green
      return $true
    } else {
      Write-Host "  FAIL: $Name" -ForegroundColor Red
      return $false
    }
  } catch {
    Write-Host "  FAIL: $Name ($($_.Exception.Message))" -ForegroundColor Red
    return $false
  }
}

function Invoke-CurlGet {
  param([string]$Path)
  $result = curl.exe -s --connect-timeout 5 "$BaseUrl$Path" 2>$null
  if ($LASTEXITCODE -ne 0) { throw "curl failed with exit code $LASTEXITCODE" }
  return $result | ConvertFrom-Json
}

function Invoke-CurlPost {
  param([string]$Path, [object]$Body)
  $json = $Body | ConvertTo-Json -Compress
  $tempFile = [System.IO.Path]::GetTempFileName()
  $json | Set-Content -Path $tempFile -Encoding Ascii -NoNewline
  try {
    $result = curl.exe -s --connect-timeout 5 -X POST "$BaseUrl$Path" -H "Content-Type: application/json" -d "@$tempFile" 2>$null
    $statusCode = $LASTEXITCODE
    # We can't easily get status code from curl, so we try to parse result
    if ($result) {
      $parsed = $result | ConvertFrom-Json
      return @{ Body = $parsed }
    }
    return @{ Body = $null }
  } finally {
    Remove-Item -Path $tempFile -Force -ErrorAction SilentlyContinue
  }
}

function Get-CurlStatus {
  param([string]$Path, [string]$Method = "GET", [object]$Body = $null)
  $tempFile = [System.IO.Path]::GetTempFileName()
  try {
    if ($Body) {
      $json = $Body | ConvertTo-Json -Compress
      $json | Set-Content -Path $tempFile -Encoding Ascii -NoNewline
      $result = curl.exe -s --connect-timeout 5 -w "%{http_code}" -X POST "$BaseUrl$Path" -H "Content-Type: application/json" -d "@$tempFile" 2>$null
    } else {
      $result = curl.exe -s --connect-timeout 5 -w "%{http_code}" "$BaseUrl$Path" 2>$null
    }
    if ($result -match '^(.+)(\d{3})$') {
      return @{ Body = $Matches[1]; StatusCode = [int]$Matches[2] }
    }
    return @{ Body = $null; StatusCode = 0 }
  } finally {
    Remove-Item -Path $tempFile -Force -ErrorAction SilentlyContinue
  }
}

# --- Start the router ---
Write-Host "=== Rust Router Endpoint Tests ===" -ForegroundColor Cyan
Write-Host ""

Write-Host "Starting rust-router on port $Port..." -ForegroundColor Yellow
$proc = Start-Process -FilePath $BinaryPath -ArgumentList "--port", $Port -NoNewWindow -PassThru

# Wait for startup
$ready = $false
for ($i = 0; $i -lt 20; $i++) {
  Start-Sleep -Milliseconds 500
  try {
    $result = curl.exe -s --connect-timeout 2 "$BaseUrl/health" 2>$null
    if ($LASTEXITCODE -eq 0 -and $result) { $ready = $true; break }
  } catch {}
}
if (-not $ready) {
  Write-Host "FAIL: Router did not start within 10 seconds" -ForegroundColor Red
  if ($proc -and !$proc.HasExited) { $proc.Kill() }
  exit 1
}
Write-Host "Router started successfully." -ForegroundColor Green
Write-Host ""

# --- Tests ---
Write-Host "--- GET /health ---" -ForegroundColor Cyan
$ok = Test-Step -Name "Returns 200 OK" -Block { ($null -ne (Invoke-CurlGet -Path "/health").status) }
if ($ok) { $Passed++ } else { $Failed++ }
$ok = Test-Step -Name "Contains authority field" -Block { ("advisory_only" -eq (Invoke-CurlGet -Path "/health").authority) }
if ($ok) { $Passed++ } else { $Failed++ }

Write-Host ""
Write-Host "--- GET /backend/profiles ---" -ForegroundColor Cyan
$ok = Test-Step -Name "Returns profiles list" -Block { ($null -ne (Invoke-CurlGet -Path "/backend/profiles").profiles) }
if ($ok) { $Passed++ } else { $Failed++ }
$ok = Test-Step -Name "Contains authority field" -Block { ("advisory_only" -eq (Invoke-CurlGet -Path "/backend/profiles").authority) }
if ($ok) { $Passed++ } else { $Failed++ }

Write-Host ""
Write-Host "--- GET /backend/status ---" -ForegroundColor Cyan
$ok = Test-Step -Name "Returns status object" -Block { ($null -ne (Invoke-CurlGet -Path "/backend/status").status) }
if ($ok) { $Passed++ } else { $Failed++ }
$ok = Test-Step -Name "Shows 0 runtimes alive" -Block { (0 -eq (Invoke-CurlGet -Path "/backend/status").runtimes_alive) }
if ($ok) { $Passed++ } else { $Failed++ }

Write-Host ""
Write-Host "--- GET /backend/health ---" -ForegroundColor Cyan
$ok = Test-Step -Name "Returns health object" -Block { ($null -ne (Invoke-CurlGet -Path "/backend/health").status) }
if ($ok) { $Passed++ } else { $Failed++ }

Write-Host ""
Write-Host "--- GET /v1/models ---" -ForegroundColor Cyan
$ok = Test-Step -Name "Returns models list" -Block { ($null -ne (Invoke-CurlGet -Path "/v1/models").data) }
if ($ok) { $Passed++ } else { $Failed++ }
$ok = Test-Step -Name "Contains object=list" -Block { ("list" -eq (Invoke-CurlGet -Path "/v1/models").object) }
if ($ok) { $Passed++ } else { $Failed++ }

Write-Host ""
Write-Host "--- POST /backend/select with invalid profile ---" -ForegroundColor Cyan
$result = Get-CurlStatus -Path "/backend/select" -Method POST -Body @{ profile = "__nonexistent__" }
$ok = Test-Step -Name "Returns 403 for unknown profile" -Block { $result.StatusCode -eq 403 }
if ($ok) { $Passed++ } else { $Failed++ }

Write-Host ""
Write-Host "--- POST /backend/stop (no backends) ---" -ForegroundColor Cyan
$result = Get-CurlStatus -Path "/backend/stop" -Method POST -Body @{ }
$ok = Test-Step -Name "Accepts stop with no backends" -Block { $result.StatusCode -in @(200, 400) }
if ($ok) { $Passed++ } else { $Failed++ }

Write-Host ""
Write-Host "--- POST /backend/restart with invalid profile ---" -ForegroundColor Cyan
$result = Get-CurlStatus -Path "/backend/restart" -Method POST -Body @{ profile = "__nonexistent__" }
$ok = Test-Step -Name "Returns 403 for unknown profile" -Block { $result.StatusCode -eq 403 }
if ($ok) { $Passed++ } else { $Failed++ }

Write-Host ""
Write-Host "--- POST /backend/restart without prior select ---" -ForegroundColor Cyan
$result = Get-CurlStatus -Path "/backend/restart" -Method POST -Body @{ profile = "phi-4" }
$ok = Test-Step -Name "Returns 503 for unselected profile" -Block { $result.StatusCode -eq 503 }
if ($ok) { $Passed++ } else { $Failed++ }

Write-Host ""
Write-Host "--- POST /backend/chat without backend ---" -ForegroundColor Cyan
$result = Get-CurlStatus -Path "/backend/chat" -Method POST -Body @{ profile = "phi-4"; messages = @(@{role="user";content="hello"}) }
$ok = Test-Step -Name "Returns 403 (refusal: runtime_unhealthy)" -Block { $result.StatusCode -eq 403 }
if ($ok) { $Passed++ } else { $Failed++ }

Write-Host ""
Write-Host "--- POST /v1/chat/completions without backend ---" -ForegroundColor Cyan
$result = Get-CurlStatus -Path "/v1/chat/completions" -Method POST -Body @{ model="phi-4"; messages = @(@{role="user";content="hello"}) }
$ok = Test-Step -Name "Returns 503 for no active backend" -Block { $result.StatusCode -eq 503 }
if ($ok) { $Passed++ } else { $Failed++ }

# --- Summary ---
Write-Host ""
Write-Host "=== Results ===" -ForegroundColor Cyan
Write-Host "Passed: $Passed" -ForegroundColor $(if ($Failed -eq 0) { "Green" } else { "Yellow" })
Write-Host "Failed: $Failed" -ForegroundColor $(if ($Failed -eq 0) { "Green" } else { "Red" })

# --- Cleanup ---
if ($proc -and !$proc.HasExited) {
  Write-Host "Stopping router..." -ForegroundColor Yellow
  $proc.Kill()
  Write-Host "Router stopped." -ForegroundColor Green
}

if ($Failed -gt 0) { exit 1 } else { exit 0 }
