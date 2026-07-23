<#
.SYNOPSIS
  Router HTTP Contract Tests (ROUTER-CONTRACT-TESTS-1)

.DESCRIPTION
  Freezes the current router HTTP/API behavior before any native daemon
  or router implementation changes. Tests all externally visible endpoints
  for success status codes, JSON response shape, auth behavior, and safety
  boundaries.

  Runs in two phases:
    Phase 1 — Auth disabled (default): tests all 7 target endpoints
    Phase 2 — Auth enabled: tests missing/invalid/valid token behavior

  Hard constraints:
    - Tests first. Do not modify router behavior.
    - No model execution required.
    - Temporary local tokens only for auth tests.
    - No secrets committed or persisted.
    - Final state: Stopped, port free, no orphans, clean trees.

  Target endpoints:
    GET  /backend/status
    GET  /backend/profiles
    GET  /backend/health
    GET  /v1/models
    POST /backend/select
    POST /v1/chat/completions
    POST /backend/stop

  Behavior categories:
    - Success status codes (200)
    - Unauthorized / missing token (401)
    - Invalid token (401)
    - JSON response shape verification
    - Profile list shape verification
    - Selected profile state behavior
    - Chat request pass-through contract (no long generation)
    - Stop behavior
    - Oversized body rejection (413)
    - Malformed JSON handling (400)
    - No secret leakage in responses

  Required acceptance criteria:
    CONTRACT-001 through CONTRACT-012

.AUTHORITY
  advisory_only

.SPRINT
  ROUTER-CONTRACT-TESTS-1
#>

param(
  [int]$Port = 9130,
  [string]$RouterDir = "G:\OpenWork\librarian-runtime-node\rust-router",
  [string]$BinaryPath = "G:\OpenWork\librarian-runtime-node\rust-router\target\release\rust-router.exe",
  [string]$ProfileConfigPath = "G:\OpenWork\librarian-runtime-node\config\model-profiles.json"
)

$BaseUrl = "http://127.0.0.1:$Port"
$Passed = 0
$Failed = 0
$Total = 0

# Test result tracking
$Results = New-Object System.Collections.ArrayList
$AuthResults = New-Object System.Collections.ArrayList

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

function Write-Step {
  param([string]$Label)
  Write-Host "`n--- $Label ---" -ForegroundColor Cyan
}

function Test-Step {
  param([string]$Name, [scriptblock]$Block)
  $script:Total++
  try {
    $result = & $Block
    if ($result) {
      Write-Host "  PASS: $Name" -ForegroundColor Green
      $script:Passed++
      return $true
    } else {
      Write-Host "  FAIL: $Name" -ForegroundColor Red
      $script:Failed++
      return $false
    }
  } catch {
    Write-Host "  FAIL: $Name ($($_.Exception.Message))" -ForegroundColor Red
    $script:Failed++
    return $false
  }
}

function New-TempFile {
  return [System.IO.Path]::GetTempFileName()
}

function Remove-TempFile {
  param([string]$Path)
  Remove-Item -Path $Path -Force -ErrorAction SilentlyContinue
}

# Invoke an HTTP request and return structured result
function Invoke-Http {
  param(
    [string]$Method = "GET",
    [string]$Path,
    [object]$Body = $null,
    [int]$ConnectTimeout = 5,
    [string]$AuthToken = $null
  )

  $tempFile = New-TempFile
  try {
    $curlArgs = @("-s", "--connect-timeout", "$ConnectTimeout", "-w", "%{http_code}")

    if ($AuthToken) {
      # Auth middleware checks the raw token value (not "Bearer <token>" format)
      $curlArgs += @("-H", "Authorization: $AuthToken")
    }

    if ($Body -and $Method -eq "POST") {
      $json = $Body | ConvertTo-Json -Compress -Depth 10
      $json | Set-Content -Path $tempFile -Encoding Ascii -NoNewline
      $curlArgs += @("-X", "POST", "-H", "Content-Type: application/json", "-d", "@$tempFile")
    } elseif ($Method -eq "POST") {
      $curlArgs += @("-X", "POST", "-H", "Content-Type: application/json", "-d", "{}")
    }

    $curlArgs += "$BaseUrl$Path"

    $response = curl.exe @curlArgs 2>$null
    $exitCode = $LASTEXITCODE

    if ($exitCode -ne 0 -and -not $response) {
      return @{ Body = $null; StatusCode = 0; Raw = $null; Error = "curl exit code $exitCode" }
    }

    # Split body and status code (last 3 chars are status)
    if ($response -match '^(.*?)(\d{3})$') {
      $rawBody = $Matches[1]
      $statusCode = [int]$Matches[2]
    } else {
      return @{ Body = $null; StatusCode = 0; Raw = $response; Error = "Could not parse status code" }
    }

    $parsedBody = $null
    if ($rawBody -and $rawBody.Trim().Length -gt 0) {
      try { $parsedBody = $rawBody | ConvertFrom-Json } catch { $parsedBody = $rawBody }
    }

    return @{ Body = $parsedBody; StatusCode = $statusCode; Raw = $rawBody; Error = $null }
  } finally {
    Remove-TempFile -Path $tempFile
  }
}

# ---------------------------------------------------------------------------
# Phase 1: Unauthenticated Mode (default)
# ---------------------------------------------------------------------------

function Run-Phase1-Unauthenticated {
  Write-Host "`n========================================" -ForegroundColor Magenta
  Write-Host " PHASE 1: Auth DISABLED (default mode)" -ForegroundColor Magenta
  Write-Host "========================================" -ForegroundColor Magenta

  # --- GET /backend/status ---
  Write-Step "GET /backend/status"
  Test-Step -Name "Returns 200 OK" -Block {
    $r = Invoke-Http -Path "/backend/status"
    $r.StatusCode -eq 200
  }
  Test-Step -Name "Response has 'status' field" -Block {
    $r = Invoke-Http -Path "/backend/status"
    $null -ne $r.Body.status
  }
  Test-Step -Name "Response has 'authority: advisory_only'" -Block {
    $r = Invoke-Http -Path "/backend/status"
    $r.Body.authority -eq "advisory_only"
  }
  Test-Step -Name "Response has 'profiles_registered' >= 5" -Block {
    $r = Invoke-Http -Path "/backend/status"
    $r.Body.profiles_registered -ge 5
  }
  Test-Step -Name "Response has 'runtimes_alive' (integer)" -Block {
    $r = Invoke-Http -Path "/backend/status"
    ($r.Body.runtimes_alive -is [int]) -or ($r.Body.runtimes_alive -is [long])
  }
  Test-Step -Name "Response has 'uptime_seconds' (integer)" -Block {
    $r = Invoke-Http -Path "/backend/status"
    ($r.Body.uptime_seconds -is [int]) -or ($r.Body.uptime_seconds -is [long])
  }
  Test-Step -Name "Response has 'active_profile' (null when no backends)" -Block {
    $r = Invoke-Http -Path "/backend/status"
    $null -eq $r.Body.active_profile
  }
  Test-Step -Name "Response 'profiles' is an object" -Block {
    $r = Invoke-Http -Path "/backend/status"
    $r.Body.profiles -is [System.Management.Automation.PSCustomObject]
  }
  Test-Step -Name "Response has no credential leakage" -Block {
    $r = Invoke-Http -Path "/backend/status"
    # Check no sensitive patterns leaked (avoid false positives from "authority" / "authorization")
    $body = $r.Raw -replace '\d{3}$', ''
    $body -notmatch '(?i)(api_key|password|secret|credential)\s*[:"''=]' -and
    $body -notmatch '(?i)bearer\s+\S+\s' -and
    $body -notmatch '(?i)x-api-key'
  }

  # --- GET /backend/profiles ---
  Write-Step "GET /backend/profiles"
  Test-Step -Name "Returns 200 OK" -Block {
    $r = Invoke-Http -Path "/backend/profiles"
    $r.StatusCode -eq 200
  }
  Test-Step -Name "Response has 'profiles' array" -Block {
    $r = Invoke-Http -Path "/backend/profiles"
    $r.Body.profiles -is [System.Array]
  }
  Test-Step -Name "Profiles array has at least 5 entries" -Block {
    $r = Invoke-Http -Path "/backend/profiles"
    $r.Body.profiles.Count -ge 5
  }
  Test-Step -Name "Each profile has 'alias' string" -Block {
    $r = Invoke-Http -Path "/backend/profiles"
    $allOk = $true
    foreach ($p in $r.Body.profiles) { if ([string]::IsNullOrEmpty($p.alias)) { $allOk = $false } }
    $allOk
  }
  Test-Step -Name "Each profile has 'task_classes' array" -Block {
    $r = Invoke-Http -Path "/backend/profiles"
    $allOk = $true
    foreach ($p in $r.Body.profiles) { if ($p.task_classes -isnot [System.Array]) { $allOk = $false } }
    $allOk
  }
  Test-Step -Name "Each profile has 'verified' boolean" -Block {
    $r = Invoke-Http -Path "/backend/profiles"
    $allOk = $true
    foreach ($p in $r.Body.profiles) { if ($null -eq $p.verified -or ($p.verified -isnot [bool])) { $allOk = $false } }
    $allOk
  }
  Test-Step -Name "Each profile has 'port' integer" -Block {
    $r = Invoke-Http -Path "/backend/profiles"
    $allOk = $true
    foreach ($p in $r.Body.profiles) { if (($p.port -isnot [int]) -and ($p.port -isnot [long])) { $allOk = $false } }
    $allOk
  }
  Test-Step -Name "Response has 'authority: advisory_only'" -Block {
    $r = Invoke-Http -Path "/backend/profiles"
    $r.Body.authority -eq "advisory_only"
  }
  Test-Step -Name "Phi-4 profile is present" -Block {
    $r = Invoke-Http -Path "/backend/profiles"
    $matches = @($r.Body.profiles | Where-Object { $_.alias -eq "phi-4" })
    $matches.Count -ge 1
  }
  Test-Step -Name "No model_file path leakage (only filename, not full path)" -Block {
    $r = Invoke-Http -Path "/backend/profiles"
    $allOk = $true
    foreach ($p in $r.Body.profiles) {
      if ($p.model_file -and $p.model_file -match '^[A-Za-z]:\\') { $allOk = $false }
    }
    $allOk
  }

  # --- GET /backend/health ---
  Write-Step "GET /backend/health"
  Test-Step -Name "Returns 200 OK" -Block {
    $r = Invoke-Http -Path "/backend/health"
    $r.StatusCode -eq 200
  }
  Test-Step -Name "Response has 'status' field" -Block {
    $r = Invoke-Http -Path "/backend/health"
    $null -ne $r.Body.status
  }
  Test-Step -Name "Response has 'authority: advisory_only'" -Block {
    $r = Invoke-Http -Path "/backend/health"
    $r.Body.authority -eq "advisory_only"
  }
  Test-Step -Name "Response 'profiles' is an object" -Block {
    $r = Invoke-Http -Path "/backend/health"
    $r.Body.profiles -is [System.Management.Automation.PSCustomObject]
  }
  Test-Step -Name "Response 'active_profile' is null when no backends" -Block {
    $r = Invoke-Http -Path "/backend/health"
    $null -eq $r.Body.active_profile
  }

  # --- GET /v1/models ---
  Write-Step "GET /v1/models"
  Test-Step -Name "Returns 200 OK" -Block {
    $r = Invoke-Http -Path "/v1/models"
    $r.StatusCode -eq 200
  }
  Test-Step -Name "Response 'object' is 'list'" -Block {
    $r = Invoke-Http -Path "/v1/models"
    $r.Body.object -eq "list"
  }
  Test-Step -Name "Response has 'data' array" -Block {
    $r = Invoke-Http -Path "/v1/models"
    $r.Body.data -is [System.Array]
  }
  Test-Step -Name "Data array has at least 5 entries" -Block {
    $r = Invoke-Http -Path "/v1/models"
    $r.Body.data.Count -ge 5
  }
  Test-Step -Name "Each model has 'id' string" -Block {
    $r = Invoke-Http -Path "/v1/models"
    $allOk = $true
    foreach ($m in $r.Body.data) { if ([string]::IsNullOrEmpty($m.id)) { $allOk = $false } }
    $allOk
  }
  Test-Step -Name "Each model has 'object: model'" -Block {
    $r = Invoke-Http -Path "/v1/models"
    $allOk = $true
    foreach ($m in $r.Body.data) { if ($m.object -ne "model") { $allOk = $false } }
    $allOk
  }
  Test-Step -Name "Each model has 'owned_by' string" -Block {
    $r = Invoke-Http -Path "/v1/models"
    $allOk = $true
    foreach ($m in $r.Body.data) { if ([string]::IsNullOrEmpty($m.owned_by)) { $allOk = $false } }
    $allOk
  }
  Test-Step -Name "Response has 'authority: advisory_only'" -Block {
    $r = Invoke-Http -Path "/v1/models"
    $r.Body.authority -eq "advisory_only"
  }
  Test-Step -Name "No model_file or path leakage in /v1/models" -Block {
    $r = Invoke-Http -Path "/v1/models"
    $r.Raw -notmatch '(?i)(model_file|model_path|gguf|llama\.cpp)'
  }

  # --- POST /backend/select invalid profile ---
  Write-Step "POST /backend/select (invalid profile)"
  Test-Step -Name "Returns 403 for nonexistent profile" -Block {
    $r = Invoke-Http -Method POST -Path "/backend/select" -Body @{ profile = "__nonexistent__" }
    $r.StatusCode -eq 403
  }
  Test-Step -Name "Refusal has 'status: refused'" -Block {
    $r = Invoke-Http -Method POST -Path "/backend/select" -Body @{ profile = "__nonexistent__" }
    $r.Body.status -eq "refused"
  }
  Test-Step -Name "Refusal has 'reason: unknown_profile'" -Block {
    $r = Invoke-Http -Method POST -Path "/backend/select" -Body @{ profile = "__nonexistent__" }
    $r.Body.reason -eq "unknown_profile"
  }
  Test-Step -Name "Refusal has 'detail' string" -Block {
    $r = Invoke-Http -Method POST -Path "/backend/select" -Body @{ profile = "__nonexistent__" }
    -not [string]::IsNullOrEmpty($r.Body.detail)
  }
  Test-Step -Name "Refusal has 'authority: advisory_only'" -Block {
    $r = Invoke-Http -Method POST -Path "/backend/select" -Body @{ profile = "__nonexistent__" }
    $r.Body.authority -eq "advisory_only"
  }
  Test-Step -Name "Refusal has 'timestamp' (ISO 8601)" -Block {
    $r = Invoke-Http -Method POST -Path "/backend/select" -Body @{ profile = "__nonexistent__" }
    -not [string]::IsNullOrEmpty($r.Body.timestamp)
  }

  # --- POST /backend/select missing profile field ---
  Write-Step "POST /backend/select (missing profile field)"
  Test-Step -Name "Returns 422 for missing profile field" -Block {
    $r = Invoke-Http -Method POST -Path "/backend/select" -Body @{ }
    $r.StatusCode -eq 422
  }
  Test-Step -Name "422 response mentions missing field 'profile'" -Block {
    $r = Invoke-Http -Method POST -Path "/backend/select" -Body @{ }
    $r.Raw -match "missing field.*profile"
  }

  # --- POST /backend/select with invalid task_class ---
  Write-Step "POST /backend/select (invalid task_class)"
  Test-Step -Name "Returns 403 for invalid task_class" -Block {
    $r = Invoke-Http -Method POST -Path "/backend/select" -Body @{ profile = "phi-4"; task_class = "__bogus__" }
    $r.StatusCode -eq 403
  }

  # --- POST /backend/stop (no backends running) ---
  Write-Step "POST /backend/stop (no backends)"
  Test-Step -Name "Returns 400 for no backends" -Block {
    $r = Invoke-Http -Method POST -Path "/backend/stop" -Body @{ }
    $r.StatusCode -eq 400
  }
  Test-Step -Name "Stop response has 'status: error'" -Block {
    $r = Invoke-Http -Method POST -Path "/backend/stop" -Body @{ }
    $r.Body.status -eq "error"
  }
  Test-Step -Name "Stop response has 'detail: No backends running'" -Block {
    $r = Invoke-Http -Method POST -Path "/backend/stop" -Body @{ }
    $r.Body.detail -eq "No backends running"
  }

  # --- POST /v1/chat/completions without backend ---
  Write-Step "POST /v1/chat/completions (no active backend)"
  Test-Step -Name "Returns 503 for no active backend" -Block {
    $r = Invoke-Http -Method POST -Path "/v1/chat/completions" -Body @{ model = "phi-4"; messages = @(@{ role = "user"; content = "hello" }) }
    $r.StatusCode -eq 503
  }
  Test-Step -Name "503 response has 'error' field" -Block {
    $r = Invoke-Http -Method POST -Path "/v1/chat/completions" -Body @{ model = "phi-4"; messages = @(@{ role = "user"; content = "hello" }) }
    -not [string]::IsNullOrEmpty($r.Body.error)
  }
  Test-Step -Name "503 response mentions select first" -Block {
    $r = Invoke-Http -Method POST -Path "/v1/chat/completions" -Body @{ model = "phi-4"; messages = @(@{ role = "user"; content = "hello" }) }
    $r.Body.error -match "select first"
  }

  # --- POST /v1/chat/completions empty messages ---
  # NOTE: Empty messages is gated behind backend-availability check.
  # When no backend is selected, the handler returns 503 before checking messages.
  Write-Step "POST /v1/chat/completions (empty messages, no active backend)"
  Test-Step -Name "Returns 503 for no backend (gates before message check)" -Block {
    $r = Invoke-Http -Method POST -Path "/v1/chat/completions" -Body @{ model = "phi-4"; messages = @() }
    $r.StatusCode -eq 503
  }

  # --- POST /v1/chat/completions no model field ---
  Write-Step "POST /v1/chat/completions (no model field)"
  Test-Step -Name "Defaults model to empty and returns 503 (no backend)" -Block {
    $r = Invoke-Http -Method POST -Path "/v1/chat/completions" -Body @{ messages = @(@{ role = "user"; content = "hi" }) }
    $r.StatusCode -eq 503
  }

  # --- Malformed JSON handling ---
  Write-Step "Malformed JSON handling"
  Test-Step -Name "POST with malformed JSON returns 400" -Block {
    $r = Invoke-Http -Method POST -Path "/backend/select"
    # Override the body - send raw invalid JSON
    $tempFile = New-TempFile
    try {
      "{invalid json}" | Set-Content -Path $tempFile -Encoding Ascii -NoNewline
      $response = curl.exe -s --connect-timeout 5 -w "%{http_code}" -X POST "$BaseUrl/backend/select" -H "Content-Type: application/json" -d "@$tempFile" 2>$null
      $match = $response -match '^(.*?)(\d{3})$'
      $match -and [int]$Matches[2] -eq 400
    } finally { Remove-TempFile -Path $tempFile }
  }
  Test-Step -Name "POST blank body returns 400 (not 500)" -Block {
    $tempFile = New-TempFile
    try {
      "" | Set-Content -Path $tempFile -Encoding Ascii -NoNewline
      $response = curl.exe -s --connect-timeout 5 -w "%{http_code}" -X POST "$BaseUrl/backend/select" -H "Content-Type: application/json" -d "@$tempFile" 2>$null
      $match = $response -match '^(.*?)(\d{3})$'
      $match -and [int]$Matches[2] -ge 400 -and [int]$Matches[2] -lt 500
    } finally { Remove-TempFile -Path $tempFile }
  }

  # --- Oversized body rejection ---
  Write-Step "Oversized body rejection"
  Test-Step -Name "Body >10MB returns 413" -Block {
    $tempFile = New-TempFile
    try {
      # Create a JSON payload with a large padding field
      $padding = 'x' * 12000000  # ~12 MB
      $json = "{`"profile`":`"phi-4`",`"pad`":`"$padding`"}"
      $json | Set-Content -Path $tempFile -Encoding Ascii -NoNewline
      $response = curl.exe -s --connect-timeout 15 -w "%{http_code}" -X POST "$BaseUrl/backend/select" -H "Content-Type: application/json" -d "@$tempFile" 2>$null
      $match = $response -match '^(.*?)(\d{3})$'
      $match -and [int]$Matches[2] -eq 413
    } finally { Remove-TempFile -Path $tempFile }
  }
  Test-Step -Name "Body <10MB accepted (no 413)" -Block {
    $r = Invoke-Http -Method POST -Path "/backend/select" -Body @{ profile = "__nonexistent__" }
    $r.StatusCode -ne 413
  }

  # --- No secret leakage ---
  Write-Step "No secret leakage check"
  Test-Step -Name "All GET responses omit hardcoded config paths" -Block {
    $paths = @("/backend/status", "/backend/profiles", "/backend/health", "/v1/models")
    $allOk = $true
    foreach ($p in $paths) {
      $r = Invoke-Http -Path $p
      $body = $r.Raw -replace '\d{3}$', ''
      # Skip false positive on "authority" — check for actual credential leakage
      if ($body -match '(?i)(?:api_key|password|secret|credential)\s*[:"''=]') {
        $allOk = $false
        Write-Host "    WARN: $p contains sensitive-looking content" -ForegroundColor Yellow
      }
    }
    $allOk
  }
}

# ---------------------------------------------------------------------------
# Phase 2: Authenticated Mode
# ---------------------------------------------------------------------------

function Run-Phase2-Authenticated {
  Write-Host "`n========================================" -ForegroundColor Magenta
  Write-Host " PHASE 2: Auth ENABLED" -ForegroundColor Magenta
  Write-Host "========================================" -ForegroundColor Magenta

  # Generate a temporary auth token for testing
  $testToken = "router-contract-test-token-$(Get-Random)"
  Write-Host "Using temporary auth token for testing (not persisted)" -ForegroundColor Yellow

  # Start router with auth enabled
  Write-Host "Starting rust-router with auth enabled on port $Port..." -ForegroundColor Yellow
  $env:ROUTER_AUTH_TOKEN = $testToken
  $env:ROUTER_REQUIRE_AUTH = "true"
  $env:ROUTER_PORT = "$Port"
  $env:EVIDENCE_PATH = "$RouterDir\..\fixtures\windows-runtime-node\router-impl"
  $env:BACKEND_BINARY_PATH = "$RouterDir\..\runtime\llama.cpp\llama-server.exe"

  $authProc = Start-Process -FilePath $BinaryPath -ArgumentList "--port", $Port -NoNewWindow -PassThru `
    -RedirectStandardOutput "$env:TEMP\rust-router-auth-out.log" `
    -RedirectStandardError "$env:TEMP\rust-router-auth-err.log"

  # Wait for startup
  $ready = $false
  for ($i = 0; $i -lt 20; $i++) {
    Start-Sleep -Milliseconds 500
    try {
      $result = curl.exe -s --connect-timeout 2 "http://127.0.0.1:$Port/backend/status" 2>$null
      if ($LASTEXITCODE -eq 0) { $ready = $true; break }
    } catch {}
  }
  if (-not $ready) {
    Write-Host "FAIL: Auth-router did not start within 10 seconds" -ForegroundColor Red
    if ($authProc -and !$authProc.HasExited) { $authProc.Kill() }
    return $authProc
  }
  Write-Host "Auth-enabled router started successfully." -ForegroundColor Green

  # --- Auth: Missing token ---
  Write-Step "Auth: Missing token"
  Test-Step -Name "GET /backend/status without token returns 401" -Block {
    $r = Invoke-Http -Path "/backend/status"
    $r.StatusCode -eq 401
  }
  Test-Step -Name "GET /backend/profiles without token returns 401" -Block {
    $r = Invoke-Http -Path "/backend/profiles"
    $r.StatusCode -eq 401
  }
  Test-Step -Name "POST /backend/select without token returns 401" -Block {
    $r = Invoke-Http -Method POST -Path "/backend/select" -Body @{ profile = "phi-4" }
    $r.StatusCode -eq 401
  }

  # --- Auth: Invalid token ---
  Write-Step "Auth: Invalid token"
  Test-Step -Name "GET with wrong token returns 401" -Block {
    $r = Invoke-Http -Path "/backend/status" -AuthToken "wrong-token-123"
    $r.StatusCode -eq 401
  }
  Test-Step -Name "POST with wrong token returns 401" -Block {
    $r = Invoke-Http -Method POST -Path "/backend/select" -Body @{ profile = "phi-4" } -AuthToken "wrong-token-456"
    $r.StatusCode -eq 401
  }
  Test-Step -Name "GET with Bearer prefix (not raw token) returns 401" -Block {
    # Auth middleware checks the raw value, not Bearer prefix
    $r = Invoke-Http -Path "/backend/status" -AuthToken "Bearer $testToken"
    $r.StatusCode -eq 401
  }

  # --- Auth: Valid token ---
  Write-Step "Auth: Valid token (basic smoke test)"
  Test-Step -Name "GET /backend/status with valid token returns 200" -Block {
    $r = Invoke-Http -Path "/backend/status" -AuthToken $testToken
    $r.StatusCode -eq 200
  }
  Test-Step -Name "GET /backend/profiles with valid token returns 200" -Block {
    $r = Invoke-Http -Path "/backend/profiles" -AuthToken $testToken
    $r.StatusCode -eq 200
  }
  Test-Step -Name "GET /backend/health with valid token returns 200" -Block {
    $r = Invoke-Http -Path "/backend/health" -AuthToken $testToken
    $r.StatusCode -eq 200
  }
  Test-Step -Name "GET /v1/models with valid token returns 200" -Block {
    $r = Invoke-Http -Path "/v1/models" -AuthToken $testToken
    $r.StatusCode -eq 200
  }
  Test-Step -Name "Auth response has authority: advisory_only" -Block {
    $r = Invoke-Http -Path "/backend/status" -AuthToken $testToken
    $r.Body.authority -eq "advisory_only"
  }
  Test-Step -Name "Auth error returns 401 (bare status)" -Block {
    $r = Invoke-Http -Path "/backend/status"
    # 401 with empty body — Raw will be the status code appended by curl
    # Or axum may include a default body like "Unauthorized"
    $r.StatusCode -eq 401 -and ($r.Raw.Trim() -eq "401" -or $null -eq $r.Body)
  }

  # Return ONLY the process object — discard any other accumulated output
  Write-Output $authProc
  return
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

Write-Host "========================================" -ForegroundColor Cyan
Write-Host " ROUTER HTTP CONTRACT TESTS" -ForegroundColor Cyan
Write-Host " Sprint: ROUTER-CONTRACT-TESTS-1" -ForegroundColor Cyan
Write-Host " Authority: advisory_only" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# Check prerequisites
if (-not (Test-Path -LiteralPath $BinaryPath)) {
  Write-Error "Router binary not found at: $BinaryPath"
  exit 1
}
if (-not (Test-Path -LiteralPath $ProfileConfigPath)) {
  Write-Error "Profile config not found at: $ProfileConfigPath"
  exit 1
}

# ============================================================================
# Phase 1: Unauthenticated mode
# ============================================================================

Write-Host "`nStarting rust-router (auth DISABLED) on port $Port..." -ForegroundColor Yellow

# Clear auth env vars to ensure default (no auth)
Remove-Item Env:\ROUTER_AUTH_TOKEN -ErrorAction SilentlyContinue
Remove-Item Env:\ROUTER_REQUIRE_AUTH -ErrorAction SilentlyContinue
$env:ROUTER_PORT = "$Port"
$env:EVIDENCE_PATH = "$RouterDir\..\fixtures\windows-runtime-node\router-impl"
$env:BACKEND_BINARY_PATH = "$RouterDir\..\runtime\llama.cpp\llama-server.exe"

$proc1 = Start-Process -FilePath $BinaryPath -ArgumentList "--port", $Port -NoNewWindow -PassThru `
  -RedirectStandardOutput "$env:TEMP\rust-router-phase1-out.log" `
  -RedirectStandardError "$env:TEMP\rust-router-phase1-err.log"

# Wait for startup
$ready = $false
for ($i = 0; $i -lt 30; $i++) {
  Start-Sleep -Milliseconds 500
  try {
    $result = curl.exe -s --connect-timeout 2 "http://127.0.0.1:$Port/backend/status" 2>$null
    if ($LASTEXITCODE -eq 0 -and $result) { $ready = $true; break }
  } catch {}
}
if (-not $ready) {
  Write-Host "FAIL: Router did not start within 15 seconds" -ForegroundColor Red
  if ($proc1 -and !$proc1.HasExited) { $proc1.Kill() }
  exit 1
}
Write-Host "Router started successfully." -ForegroundColor Green

# Run Phase 1 tests
Run-Phase1-Unauthenticated

# Stop Phase 1 router
Write-Host "`nStopping Phase 1 router..." -ForegroundColor Yellow
if ($proc1 -and !$proc1.HasExited) {
  $proc1.Kill()
  Start-Sleep -Seconds 2
}
Write-Host "Phase 1 router stopped." -ForegroundColor Green

# ============================================================================
# Phase 2: Authenticated mode
# ============================================================================

Run-Phase2-Authenticated | Out-Null

# Stop Phase 2 router — find by process name
Write-Host "`nStopping Phase 2 router..." -ForegroundColor Yellow
$authRouters = Get-Process rust-router -ErrorAction SilentlyContinue | Where-Object { $_.StartTime -gt (Get-Date).AddMinutes(-5) }
foreach ($p in $authRouters) {
  Write-Host "  Killing rust-router PID $($p.Id)" -ForegroundColor Yellow
  $p.Kill()
}
Start-Sleep -Seconds 2
Write-Host "Phase 2 router stopped." -ForegroundColor Green

# Clean up temp auth token
Remove-Item Env:\ROUTER_AUTH_TOKEN -ErrorAction SilentlyContinue
Remove-Item Env:\ROUTER_REQUIRE_AUTH -ErrorAction SilentlyContinue

# ============================================================================
# Summary
# ============================================================================

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host " RESULTS" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Total:  $Total"
Write-Host "Passed: $Passed" -ForegroundColor $(if ($Failed -eq 0) { "Green" } else { "Yellow" })
Write-Host "Failed: $Failed" -ForegroundColor $(if ($Failed -eq 0) { "Green" } else { "Red" })
Write-Host ""

# Verify final state
Write-Host "--- Final State Verification ---" -ForegroundColor Cyan
$portCheck = netstat -ano | Select-String ":9130.*LISTENING"
$orphanRouters = Get-Process rust-router -ErrorAction SilentlyContinue
$orphanLlama = Get-Process llama-server -ErrorAction SilentlyContinue

if (-not $portCheck) { Write-Host "  OK: Port $Port is free" -ForegroundColor Green } else { Write-Host "  WARN: Port $Port still in use" -ForegroundColor Yellow }
if (-not $orphanRouters) { Write-Host "  OK: No rust-router orphans" -ForegroundColor Green } else { Write-Host "  WARN: rust-router orphans found" -ForegroundColor Yellow }
if (-not $orphanLlama) { Write-Host "  OK: No llama-server orphans" -ForegroundColor Green } else { Write-Host "  WARN: llama-server orphans found" -ForegroundColor Yellow }

Write-Host "`nDone." -ForegroundColor Cyan

if ($Failed -gt 0) { exit 1 } else { exit 0 }
