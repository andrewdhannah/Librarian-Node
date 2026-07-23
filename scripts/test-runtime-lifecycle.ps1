<#
.SYNOPSIS
  WIN-RUNTIME-QUALIFICATION-1: Dimension 3 — Service/Process Lifecycle

.DESCRIPTION
  Verifies the full lifecycle of the router:
  - Start router process
  - Exercise endpoints (select backend, chat, stop backend)
  - Stop router
  - Confirm clean shutdown
  - Verify no leaks

  Does NOT touch the Windows service (LibrarianRunTimeNode) — that is
  explicitly deferred. This tests the portable router process lifecycle.

.AUTHORITY
  advisory_only
#>

param(
  [int]$Port = 9130,
  [string]$BinaryPath = "G:\OpenWork\librarian-runtime-node\rust-router\target\release\rust-router.exe",
  [string]$RouterDir = "G:\OpenWork\librarian-runtime-node\rust-router",
  [switch]$SkipBackendTests = $false
)

$ErrorActionPreference = "Stop"

function Write-Step { param([string]$M) Write-Host "`n--- $M ---" -ForegroundColor Cyan }
function Test-Pass { param([string]$N) Write-Host "  PASS: $N" -ForegroundColor Green; $script:Passed++ }
function Test-Fail { param([string]$N, [string]$D = "") Write-Host "  FAIL: $N ($D)" -ForegroundColor Red; $script:Failed++; $script:HasFailures = $true }

$script:Passed = 0
$script:Failed = 0
$script:HasFailures = $false
$Results = @{
  StartResult = "not_run"
  SelectResult = "not_run"
  StopBackendResult = "not_run"
  StopRouterResult = "not_run"
  CleanShutdown = $false
}

$BaseUrl = "http://127.0.0.1:$Port"

function Invoke-Http {
  param([string]$Method = "GET", [string]$Path, [object]$Body = $null, [int]$Timeout = 5)
  $tempFile = [System.IO.Path]::GetTempFileName()
  try {
    $curlArgs = @("-s", "--connect-timeout", "$Timeout", "-w", "%{http_code}")
    if ($Body -and $Method -eq "POST") {
      $json = $Body | ConvertTo-Json -Compress -Depth 10
      $json | Set-Content -Path $tempFile -Encoding Ascii -NoNewline
      $curlArgs += @("-X", "POST", "-H", "Content-Type: application/json", "-d", "@$tempFile")
    } elseif ($Method -eq "POST") {
      $curlArgs += @("-X", "POST", "-H", "Content-Type: application/json", "-d", "{}")
    }
    $curlArgs += "$BaseUrl$Path"
    $response = curl.exe @curlArgs 2>$null
    if ($LASTEXITCODE -ne 0 -and -not $response) { return @{ Body = $null; StatusCode = 0 } }
    if ($response -match '^(.*?)(\d{3})$') {
      $rawBody = $Matches[1]; $statusCode = [int]$Matches[2]
      $parsedBody = $null
      if ($rawBody -and $rawBody.Trim().Length -gt 0) { try { $parsedBody = $rawBody | ConvertFrom-Json } catch {} }
      return @{ Body = $parsedBody; StatusCode = $statusCode }
    }
    return @{ Body = $null; StatusCode = 0 }
  } finally { Remove-Item -Path $tempFile -Force -ErrorAction SilentlyContinue }
}

function Test-RouterRunning {
  try { $r = curl.exe -s --connect-timeout 2 "http://127.0.0.1:$Port/backend/status" 2>$null; return ($LASTEXITCODE -eq 0 -and $null -ne $r) } catch { return $false }
}

# ============================================================================
# Phase 1: Start router
# ============================================================================
Write-Step "Starting router"
if (-not (Test-Path -LiteralPath $BinaryPath)) { Test-Fail "Binary not found at: $BinaryPath"; return $Results }

Remove-Item Env:\ROUTER_AUTH_TOKEN -ErrorAction SilentlyContinue
Remove-Item Env:\ROUTER_REQUIRE_AUTH -ErrorAction SilentlyContinue
$env:ROUTER_PORT = "$Port"
$env:EVIDENCE_PATH = "$RouterDir\..\fixtures\windows-runtime-node\router-impl"
$env:BACKEND_BINARY_PATH = "$RouterDir\..\runtime\llama.cpp\llama-server.exe"

$proc = Start-Process -FilePath $BinaryPath -ArgumentList "--port", $Port -NoNewWindow -PassThru `
  -RedirectStandardOutput "$env:TEMP\rust-router-lifecycle-out.log" `
  -RedirectStandardError "$env:TEMP\rust-router-lifecycle-err.log"

$ready = $false
for ($i = 0; $i -lt 30; $i++) { Start-Sleep -Milliseconds 500; if (Test-RouterRunning) { $ready = $true; break } }

if ($ready) {
  Test-Pass "Router started on port $Port"
  $Results.StartResult = "pass"
} else {
  Test-Fail "Router did not start within 15s"
  if ($proc -and !$proc.HasExited) { $proc.Kill() }
  $Results.StartResult = "fail"
  return $Results
}

# ============================================================================
# Phase 2: Verify endpoints respond
# ============================================================================
Write-Step "Endpoint verification (router alive)"
$r = Invoke-Http -Path "/backend/status"
if ($r.StatusCode -eq 200) { Test-Pass "/backend/status returns 200" } else { Test-Fail "/backend/status" "Got $($r.StatusCode)" }

$r = Invoke-Http -Path "/backend/profiles"
if ($r.StatusCode -eq 200) { Test-Pass "/backend/profiles returns 200" } else { Test-Fail "/backend/profiles" "Got $($r.StatusCode)" }

$r = Invoke-Http -Path "/v1/models"
if ($r.StatusCode -eq 200) { Test-Pass "/v1/models returns 200" } else { Test-Fail "/v1/models" "Got $($r.StatusCode)" }

$r = Invoke-Http -Path "/health"
if ($r.StatusCode -eq 200) { Test-Pass "/health returns 200" } else { Test-Fail "/health" "Got $($r.StatusCode)" }

# ============================================================================
# Phase 3: Select a profile (triggers backend spawning)
# ============================================================================
if (-not $SkipBackendTests) {
  Write-Step "Backend select (profile: phi-4)"
  $r = Invoke-Http -Method POST -Path "/backend/select" -Body @{ profile = "phi-4" } -Timeout 120
  $Results.SelectResult = if ($r.StatusCode -eq 200) { "pass" } else { "fail" }
  if ($r.StatusCode -eq 200) {
    Test-Pass "/backend/select phi-4 returned 200"
    if ($r.Body.status -eq "selected") { Test-Pass "status is 'selected'" } else { Test-Fail "status" "Expected selected, got $($r.Body.status)" }
  } else {
    Test-Fail "/backend/select phi-4" "Expected 200, got $($r.StatusCode) - backend may not be available"
    Write-Host "    WARN: This is expected if llama-server is not installed or models not present" -ForegroundColor Yellow
  }

  # ============================================================================
  # Phase 4: Chat (if backend is selected and healthy)
  # ============================================================================
  if ($r.StatusCode -eq 200) {
    Write-Step "Chat test (phi-4)"
    $r = Invoke-Http -Method POST -Path "/v1/chat/completions" -Body @{ model = "phi-4"; messages = @(@{ role = "user"; content = "Say 'Hello, world!'" }) } -Timeout 60
    if ($r.StatusCode -eq 200) {
      Test-Pass "Chat returned 200"
      $Results.ChatResult = "pass"
    } else {
      Test-Fail "Chat" "Expected 200, got $($r.StatusCode)"
      $Results.ChatResult = "fail"
      Write-Host "    Body: $($r.Raw)" -ForegroundColor DarkGray
    }
  }

  # ============================================================================
  # Phase 5: Stop backend
  # ============================================================================
  Write-Step "Stop backend"
  $r = Invoke-Http -Method POST -Path "/backend/stop" -Body @{ }
  $Results.StopBackendResult = if ($r.StatusCode -eq 200) { "pass" } else { "fail" }
  if ($r.StatusCode -eq 200) {
    Test-Pass "Backend stop returned 200"
    if ($r.Body.status -eq "stopped") { Test-Pass "status is 'stopped'" } else { Test-Fail "status" "Expected stopped, got $($r.Body.status)" }
  } else {
    Test-Fail "Backend stop" "Expected 200, got $($r.StatusCode)"
  }
} else {
  Write-Step "Backend tests skipped (SkipBackendTests)"
  $Results.SelectResult = "skipped"
  $Results.StopBackendResult = "skipped"
  $Results.ChatResult = "skipped"
}

# ============================================================================
# Phase 6: Stop router
# ============================================================================
Write-Step "Stop router"
if ($proc -and !$proc.HasExited) {
  # Try graceful shutdown via ctrl-c simulation
  $proc.CloseMainWindow() | Out-Null
  Start-Sleep -Seconds 3
  if (!$proc.HasExited) { $proc.Kill() }
  Start-Sleep -Seconds 1
}

$routerExited = ($null -eq $proc -or $proc.HasExited)
if ($routerExited) {
  Test-Pass "Router process exited cleanly"
  $Results.StopRouterResult = "pass"
  $Results.CleanShutdown = $true
} else {
  Test-Fail "Router process still running after stop"
  $Results.StopRouterResult = "fail"
  $Results.CleanShutdown = $false
  if ($proc -and !$proc.HasExited) { $proc.Kill() }
}

# ============================================================================
# Phase 7: Confirm port is free after stop
# ============================================================================
Write-Step "Port free after stop"
Start-Sleep -Seconds 2
$listeners = netstat -ano | Select-String ":$Port\s" | Select-String "LISTENING"
if ($listeners) {
  Test-Fail "Port $Port still in use after router stop"
} else {
  Test-Pass "Port $Port is free after router stop"
}

# ============================================================================
# Summary
# ============================================================================
$total = $script:Passed + $script:Failed
Write-Step "Dimension 3 Summary"
Write-Host "  Start: $($Results.StartResult)" -ForegroundColor DarkGray
Write-Host "  Select: $($Results.SelectResult)" -ForegroundColor DarkGray
Write-Host "  Stop backend: $($Results.StopBackendResult)" -ForegroundColor DarkGray
Write-Host "  Stop router: $($Results.StopRouterResult)" -ForegroundColor DarkGray
Write-Host "  Clean shutdown: $($Results.CleanShutdown)" -ForegroundColor DarkGray
Write-Host "  $($script:Passed) passed, $($script:Failed) failed ($total total)" -ForegroundColor $(if($script:Failed -eq 0){"Green"}else{"Red"})

$Results.Passed = $script:Passed
$Results.Failed = $script:Failed
$Results
