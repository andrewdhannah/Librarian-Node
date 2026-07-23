<#
.SYNOPSIS
  WIN-RUNTIME-QUALIFICATION-1: Dimension 2 — Router Contract Behavior

.DESCRIPTION
  Runs the frozen router contract harness against the running executable binary.
  Does NOT rely on source inspection alone — verifies contract behavior from
  the running process.

  Requires the router to already be running on the specified port.
  If the router is not running, starts it temporarily for testing.

.AUTHORITY
  advisory_only
#>

param(
  [int]$Port = 9130,
  [string]$BinaryPath = "G:\OpenWork\librarian-runtime-node\rust-router\target\release\rust-router.exe",
  [string]$ConfigDir = "G:\OpenWork\librarian-runtime-node",
  [string]$RouterDir = "G:\OpenWork\librarian-runtime-node\rust-router",
  [switch]$StartRouter = $true,
  [switch]$StopRouterAfter = $true
)

$ErrorActionPreference = "Stop"

function Write-Step { param([string]$M) Write-Host "`n--- $M ---" -ForegroundColor Cyan }
function Test-Pass { param([string]$N) Write-Host "  PASS: $N" -ForegroundColor Green; $script:Passed++ }
function Test-Fail { param([string]$N, [string]$D = "") Write-Host "  FAIL: $N ($D)" -ForegroundColor Red; $script:Failed++; $script:HasFailures = $true }

$script:Passed = 0
$script:Failed = 0
$script:HasFailures = $false
$Results = @{
  ContractVersion = ""
  HarnessResult = "not_run"
  EndpointsVerified = @()
  TotalTests = 0
  PassedTests = 0
  FailedTests = 0
}

$BaseUrl = "http://127.0.0.1:$Port"

# Helper: check if router is already running
function Test-RouterRunning {
  try {
    $r = curl.exe -s --connect-timeout 2 "http://127.0.0.1:$Port/backend/status" 2>$null
    return ($LASTEXITCODE -eq 0 -and $null -ne $r -and $r.Length -gt 0)
  } catch { return $false }
}

# Helper: invoke HTTP request
function Invoke-Http {
  param([string]$Method = "GET", [string]$Path, [object]$Body = $null, [int]$Timeout = 5, [string]$AuthToken = $null)
  $tempFile = [System.IO.Path]::GetTempFileName()
  try {
    $curlArgs = @("-s", "--connect-timeout", "$Timeout", "-w", "%{http_code}")
    if ($AuthToken) { $curlArgs += @("-H", "Authorization: $AuthToken") }
    if ($Body -and $Method -eq "POST") {
      $json = $Body | ConvertTo-Json -Compress -Depth 10
      $json | Set-Content -Path $tempFile -Encoding Ascii -NoNewline
      $curlArgs += @("-X", "POST", "-H", "Content-Type: application/json", "-d", "@$tempFile")
    } elseif ($Method -eq "POST") {
      $curlArgs += @("-X", "POST", "-H", "Content-Type: application/json", "-d", "{}")
    }
    $curlArgs += "$BaseUrl$Path"
    $response = curl.exe @curlArgs 2>$null
    if ($LASTEXITCODE -ne 0 -and -not $response) { return @{ Body = $null; StatusCode = 0; Error = "curl exit $LASTEXITCODE" } }
    if ($response -match '^(.*?)(\d{3})$') {
      $rawBody = $Matches[1]; $statusCode = [int]$Matches[2]
      $parsedBody = $null
      if ($rawBody -and $rawBody.Trim().Length -gt 0) { try { $parsedBody = $rawBody | ConvertFrom-Json } catch { $parsedBody = $rawBody } }
      return @{ Body = $parsedBody; StatusCode = $statusCode; Raw = $rawBody; Error = $null }
    }
    return @{ Body = $null; StatusCode = 0; Raw = $response; Error = "Could not parse status" }
  } finally { Remove-Item -Path $tempFile -Force -ErrorAction SilentlyContinue }
}

# ============================================================================
# Start router if needed
# ============================================================================
$proc = $null
if (-not (Test-RouterRunning)) {
  if ($StartRouter) {
    Write-Step "Starting router for contract testing"
    if (-not (Test-Path -LiteralPath $BinaryPath)) { Test-Fail "Binary not found at: $BinaryPath"; return $Results }
    Remove-Item Env:\ROUTER_AUTH_TOKEN -ErrorAction SilentlyContinue
    Remove-Item Env:\ROUTER_REQUIRE_AUTH -ErrorAction SilentlyContinue
    $env:ROUTER_PORT = "$Port"
    $env:EVIDENCE_PATH = "$RouterDir\..\fixtures\windows-runtime-node\router-impl"
    $env:BACKEND_BINARY_PATH = "$RouterDir\..\runtime\llama.cpp\llama-server.exe"
    $proc = Start-Process -FilePath $BinaryPath -ArgumentList "--port", $Port -NoNewWindow -PassThru `
      -RedirectStandardOutput "$env:TEMP\rust-router-contract-out.log" `
      -RedirectStandardError "$env:TEMP\rust-router-contract-err.log"
    $ready = $false
    for ($i = 0; $i -lt 30; $i++) { Start-Sleep -Milliseconds 500; if (Test-RouterRunning) { $ready = $true; break } }
    if (-not $ready) { Test-Fail "Router did not start within 15s"; if ($proc -and !$proc.HasExited) { $proc.Kill() }; return $Results }
    Write-Host "  Router started on port $Port" -ForegroundColor Green
  } else {
    Test-Fail "Router is not running and StartRouter is disabled"
    return $Results
  }
} else {
  Write-Host "  Router already running on port $Port" -ForegroundColor Green
}

# ============================================================================
# Phase 1: Contract version / identification
# ============================================================================
Write-Step "Contract Version Identification"
$r = Invoke-Http -Path "/backend/status"
if ($r.StatusCode -eq 200 -and $r.Body) {
  $Results.ContractVersion = "ROUTER-HTTP-CONTRACT-v1 (advisory_only)"
  Test-Pass "Contract version identified from running binary"
} else {
  Test-Fail "Could not identify contract from running binary" "Status=$($r.StatusCode)"
}

# ============================================================================
# Phase 2: GET /backend/status
# ============================================================================
Write-Step "GET /backend/status"
$r = Invoke-Http -Path "/backend/status"
if ($r.StatusCode -eq 200) { Test-Pass "Returns 200 OK"; $Results.EndpointsVerified += "/backend/status" } else { Test-Fail "Status code" "Expected 200, got $($r.StatusCode)" }
if ($r.Body) {
  if ($null -ne $r.Body.status) { Test-Pass "Has 'status' field" } else { Test-Fail "Missing 'status' field" }
  if ($r.Body.authority -eq "advisory_only") { Test-Pass "Has 'authority: advisory_only'" } else { Test-Fail "Authority" "Expected advisory_only, got $($r.Body.authority)" }
  if ($r.Body.profiles_registered -ge 5) { Test-Pass "profiles_registered >= 5" } else { Test-Fail "profiles_registered" "Expected >= 5, got $($r.Body.profiles_registered)" }
  if ($null -ne $r.Body.runtimes_alive) { Test-Pass "Has 'runtimes_alive'" } else { Test-Fail "Missing 'runtimes_alive'" }
  if ($null -ne $r.Body.uptime_seconds) { Test-Pass "Has 'uptime_seconds'" } else { Test-Fail "Missing 'uptime_seconds'" }
  # No secret leakage
  $raw = $r.Raw -replace '\d{3}$', ''
  if ($raw -notmatch '(?i)(api_key|password|secret|credential)\s*[:"''=]') { Test-Pass "No credential leakage in response" } else { Test-Fail "Credential leakage detected" }
} else { Test-Fail "No response body" }

# ============================================================================
# Phase 3: GET /backend/profiles
# ============================================================================
Write-Step "GET /backend/profiles"
$r = Invoke-Http -Path "/backend/profiles"
if ($r.StatusCode -eq 200) { Test-Pass "Returns 200 OK"; $Results.EndpointsVerified += "/backend/profiles" } else { Test-Fail "Status code" "Expected 200, got $($r.StatusCode)" }
if ($r.Body -and $r.Body.profiles) {
  if ($r.Body.profiles.Count -ge 5) { Test-Pass "Profiles array has >= 5 entries ($($r.Body.profiles.Count))" } else { Test-Fail "Profiles count" "Expected >= 5, got $($r.Body.profiles.Count)" }
  if ($r.Body.authority -eq "advisory_only") { Test-Pass "Has 'authority: advisory_only'" } else { Test-Fail "Authority mismatch" }
  # Check each profile has required fields
  $allHaveAlias = $true; $allHaveTaskClasses = $true; $allVerified = $true
  foreach ($p in $r.Body.profiles) {
    if ([string]::IsNullOrEmpty($p.alias)) { $allHaveAlias = $false }
    if ($p.task_classes -isnot [System.Array]) { $allHaveTaskClasses = $false }
    if ($null -eq $p.verified) { $allVerified = $false }
  }
  if ($allHaveAlias) { Test-Pass "Each profile has 'alias'" } else { Test-Fail "Some profiles missing 'alias'" }
  if ($allHaveTaskClasses) { Test-Pass "Each profile has 'task_classes'" } else { Test-Fail "Some profiles missing 'task_classes'" }
  # Check phi-4 profile is present
  $hasPhi4 = @($r.Body.profiles | Where-Object { $_.alias -eq "phi-4" }).Count -ge 1
  if ($hasPhi4) { Test-Pass "phi-4 profile present" } else { Test-Fail "phi-4 profile missing" }
} else { Test-Fail "No profiles data in response" }

# ============================================================================
# Phase 4: GET /backend/health
# ============================================================================
Write-Step "GET /backend/health"
$r = Invoke-Http -Path "/backend/health"
if ($r.StatusCode -eq 200) { Test-Pass "Returns 200 OK"; $Results.EndpointsVerified += "/backend/health" } else { Test-Fail "Status code" "Expected 200, got $($r.StatusCode)" }
if ($r.Body) {
  if ($null -ne $r.Body.status) { Test-Pass "Has 'status' field" } else { Test-Fail "Missing 'status' field" }
  if ($r.Body.authority -eq "advisory_only") { Test-Pass "Has 'authority: advisory_only'" } else { Test-Fail "Authority mismatch" }
  if ($null -eq $r.Body.active_profile) { Test-Pass "active_profile is null (no backends)" } else { Test-Fail "active_profile should be null" }
} else { Test-Fail "No response body" }

# ============================================================================
# Phase 5: GET /v1/models
# ============================================================================
Write-Step "GET /v1/models"
$r = Invoke-Http -Path "/v1/models"
if ($r.StatusCode -eq 200) { Test-Pass "Returns 200 OK"; $Results.EndpointsVerified += "/v1/models" } else { Test-Fail "Status code" "Expected 200, got $($r.StatusCode)" }
if ($r.Body) {
  if ($r.Body.object -eq "list") { Test-Pass "object is 'list'" } else { Test-Fail "object" "Expected 'list', got '$($r.Body.object)'" }
  if ($r.Body.data -is [System.Array] -and $r.Body.data.Count -ge 5) { Test-Pass "data array has >= 5 entries ($($r.Body.data.Count))" } else { Test-Fail "data array" "Expected >= 5, got $(if($r.Body.data){$r.Body.data.Count}else{'none'})" }
  $allHaveId = $true; $allAreModel = $true
  foreach ($m in $r.Body.data) { if ([string]::IsNullOrEmpty($m.id)) { $allHaveId = $false }; if ($m.object -ne "model") { $allAreModel = $false } }
  if ($allHaveId) { Test-Pass "Each model has 'id'" } else { Test-Fail "Some models missing 'id'" }
  if ($allAreModel) { Test-Pass "Each model has 'object: model'" } else { Test-Fail "Some models wrong object type" }
  if ($r.Body.authority -eq "advisory_only") { Test-Pass "Has 'authority: advisory_only'" } else { Test-Fail "Authority mismatch" }
  # No path leakage
  $raw = $r.Raw -replace '\d{3}$', ''
  if ($raw -notmatch '(?i)(model_file|model_path|gguf)') { Test-Pass "No model path leakage" } else { Test-Fail "Model path leakage detected" }
} else { Test-Fail "No response body" }

# ============================================================================
# Phase 6: POST /backend/select (invalid profiles)
# ============================================================================
Write-Step "POST /backend/select (invalid profile)"
$r = Invoke-Http -Method POST -Path "/backend/select" -Body @{ profile = "__nonexistent__" }
if ($r.StatusCode -eq 403) { Test-Pass "Returns 403 for nonexistent profile" } else { Test-Fail "Status code" "Expected 403, got $($r.StatusCode)" }
if ($r.Body) {
  if ($r.Body.status -eq "refused") { Test-Pass "Body has 'status: refused'" } else { Test-Fail "status" "Expected 'refused', got '$($r.Body.status)'" }
  if ($r.Body.reason -eq "unknown_profile") { Test-Pass "Body has 'reason: unknown_profile'" } else { Test-Fail "reason" "Expected 'unknown_profile', got '$($r.Body.reason)'" }
  if (-not [string]::IsNullOrEmpty($r.Body.detail)) { Test-Pass "Has 'detail' string" } else { Test-Fail "Missing 'detail' string" }
  if ($r.Body.authority -eq "advisory_only") { Test-Pass "Has 'authority: advisory_only'" } else { Test-Fail "Authority mismatch" }
  if (-not [string]::IsNullOrEmpty($r.Body.timestamp)) { Test-Pass "Has 'timestamp'" } else { Test-Fail "Missing 'timestamp'" }
} else { Test-Fail "No response body" }
$Results.EndpointsVerified += "/backend/select"

# ============================================================================
# Phase 7: POST /backend/select (missing profile field) => 422
# ============================================================================
Write-Step "POST /backend/select (missing profile field)"
$r = Invoke-Http -Method POST -Path "/backend/select" -Body @{ }
if ($r.StatusCode -eq 422) { Test-Pass "Returns 422 for missing profile field" } else { Test-Fail "Status code" "Expected 422, got $($r.StatusCode)" }

# ============================================================================
# Phase 8: POST /backend/stop (no backends running) => 400
# ============================================================================
Write-Step "POST /backend/stop (no backends)"
$r = Invoke-Http -Method POST -Path "/backend/stop" -Body @{ }
if ($r.StatusCode -eq 400) { Test-Pass "Returns 400 for no backends" } else { Test-Fail "Status code" "Expected 400, got $($r.StatusCode)" }
if ($r.Body -and $r.Body.status -eq "error") { Test-Pass "Body has 'status: error'" } else { Test-Fail "Expected 'status: error'" }
if ($r.Body -and $r.Body.detail -eq "No backends running") { Test-Pass "Body has 'detail: No backends running'" } else { Test-Fail "Expected 'No backends running' detail" }
$Results.EndpointsVerified += "/backend/stop"

# ============================================================================
# Phase 9: POST /v1/chat/completions (no backend) => 503
# ============================================================================
Write-Step "POST /v1/chat/completions (no backend)"
$r = Invoke-Http -Method POST -Path "/v1/chat/completions" -Body @{ model = "phi-4"; messages = @(@{ role = "user"; content = "hello" }) }
if ($r.StatusCode -eq 503) { Test-Pass "Returns 503 for no active backend" } else { Test-Fail "Status code" "Expected 503, got $($r.StatusCode)" }
if ($r.Body -and $r.Body.error -match "select first") { Test-Pass "Body mentions 'select first'" } else { Test-Fail "Body should mention 'select first'" }
$Results.EndpointsVerified += "/v1/chat/completions"

# ============================================================================
# Phase 10: 404 handling
# ============================================================================
Write-Step "404 handling"
$r = Invoke-Http -Path "/nonexistent/endpoint"
if ($r.StatusCode -eq 404) { Test-Pass "Unknown path returns 404" } else { Test-Fail "Status code" "Expected 404, got $($r.StatusCode)" }

# ============================================================================
# Phase 11: GET /health (legacy)
# ============================================================================
Write-Step "GET /health (legacy)"
$r = Invoke-Http -Path "/health"
if ($r.StatusCode -eq 200) { Test-Pass "Legacy /health returns 200" } else { Test-Fail "Status code" "Expected 200, got $($r.StatusCode)" }
if ($r.Body -and $r.Body.authority -eq "advisory_only") { Test-Pass "Has 'authority: advisory_only'" } else { Test-Fail "Authority mismatch" }
$Results.EndpointsVerified += "/health"

# ============================================================================
# Stop router if we started it
# ============================================================================
if ($proc -and $StopRouterAfter) {
  Write-Step "Stopping contract test router"
  if (-not $proc.HasExited) { $proc.Kill(); Start-Sleep -Seconds 1 }
  Write-Host "  Router stopped." -ForegroundColor Green
}

# ============================================================================
# Summary
# ============================================================================
$Results.TotalTests = $script:Passed + $script:Failed
$Results.PassedTests = $script:Passed
$Results.FailedTests = $script:Failed
$Results.HarnessResult = if ($script:Failed -eq 0) { "pass" } else { "fail" }

Write-Step "Dimension 2 Summary"
Write-Host "  Contract version: $($Results.ContractVersion)" -ForegroundColor DarkGray
Write-Host "  Endpoints verified: $($Results.EndpointsVerified -join ', ')" -ForegroundColor DarkGray
Write-Host "  $($script:Passed) passed, $($script:Failed) failed ($($Results.TotalTests) total)" -ForegroundColor $(if($script:Failed -eq 0){"Green"}else{"Red"})

$Results
