<#
.SYNOPSIS
  WIN-RUNTIME-QUALIFICATION-1: Dimension 6 — Request/Body Limits

.DESCRIPTION
  Verifies configured request body size limits:
  1. Read configured max_body_bytes from environment or source defaults
  2. Send oversized request > limit and verify 413 response
  3. Send request within limit and verify it is accepted (non-413)
  4. Record limit in capability estimate

.AUTHORITY
  advisory_only
#>

param(
  [int]$Port = 9132,
  [string]$BinaryPath = "G:\OpenWork\librarian-runtime-node\rust-router\target\release\rust-router.exe",
  [string]$RouterDir = "G:\OpenWork\librarian-runtime-node\rust-router"
)

$ErrorActionPreference = "Stop"

function Write-Step { param([string]$M) Write-Host "`n--- $M ---" -ForegroundColor Cyan }
function Test-Pass { param([string]$N) Write-Host "  PASS: $N" -ForegroundColor Green; $script:Passed++ }
function Test-Fail { param([string]$N, [string]$D = "") Write-Host "  FAIL: $N ($D)" -ForegroundColor Red; $script:Failed++; $script:HasFailures = $true }

$script:Passed = 0
$script:Failed = 0
$script:HasFailures = $false
$Results = @{
  MaxBodyBytes = 0
  OversizedRefused = $false
  NormalSizedAccepted = $false
  LimitSource = ""
}

# ============================================================================
# Phase 1: Determine max body bytes
# ============================================================================
Write-Step "Determine configured max_body_bytes"

# Check source code default
$configSource = Get-Content "$RouterDir\src\config.rs" -Raw
$maxBodyMatch = [regex]::Match($configSource, 'max_body_bytes.*unwrap_or\((\d+)\s*\*\s*(\d+)\s*\*\s*(\d+)\)')
$maxBodyDirect = [regex]::Match($configSource, 'max_body_bytes.*unwrap_or\((\d+)\)')

if ($maxBodyMatch.Success) {
  $a = [int]$maxBodyMatch.Groups[1].Value
  $b = [int]$maxBodyMatch.Groups[2].Value
  $c = [int]$maxBodyMatch.Groups[3].Value
  $defaultMaxBody = $a * $b * $c
  Test-Pass "Source default max_body_bytes = $a * $b * $c = $defaultMaxBody (10 MB)"
  $Results.MaxBodyBytes = $defaultMaxBody
  $Results.LimitSource = "source default (10 MB)"
} elseif ($maxBodyDirect.Success) {
  $defaultMaxBody = [int]$maxBodyDirect.Groups[1].Value
  Test-Pass "Source default max_body_bytes = $defaultMaxBody"
  $Results.MaxBodyBytes = $defaultMaxBody
  $Results.LimitSource = "source default"
} else {
  $defaultMaxBody = 10485760  # 10 MB default
  Test-Pass "Using default max_body_bytes = $defaultMaxBody (10 MB)"
  $Results.MaxBodyBytes = $defaultMaxBody
  $Results.LimitSource = "assumed default (10 MB)"
}

# Check env override
$envMax = $env:ROUTER_MAX_BODY_BYTES
if ($envMax) {
  Write-Host "  Env override ROUTER_MAX_BODY_BYTES=$envMax" -ForegroundColor DarkGray
}

# ============================================================================
# Phase 2: Start router
# ============================================================================
Write-Step "Start router for limit testing"
Remove-Item Env:\ROUTER_AUTH_TOKEN -ErrorAction SilentlyContinue
Remove-Item Env:\ROUTER_REQUIRE_AUTH -ErrorAction SilentlyContinue
$env:ROUTER_PORT = "$Port"
$env:EVIDENCE_PATH = "$RouterDir\..\fixtures\windows-runtime-node\router-impl"
$env:BACKEND_BINARY_PATH = "$RouterDir\..\runtime\llama.cpp\llama-server.exe"
# Explicitly set max body bytes to 10 MB for test
$env:ROUTER_MAX_BODY_BYTES = "10485760"

$proc = Start-Process -FilePath $BinaryPath -ArgumentList "--port", $Port -NoNewWindow -PassThru `
  -RedirectStandardOutput "$env:TEMP\rust-router-limits-out.log" `
  -RedirectStandardError "$env:TEMP\rust-router-limits-err.log"

$ready = $false
for ($i = 0; $i -lt 30; $i++) { Start-Sleep -Milliseconds 500
  try { $r = curl.exe -s --connect-timeout 2 "http://127.0.0.1:$Port/backend/status" 2>$null; if ($LASTEXITCODE -eq 0 -and $r) { $ready = $true; break } } catch {}
}
if (-not $ready) { Test-Fail "Router did not start"; if ($proc -and !$proc.HasExited) { $proc.Kill() }; return $Results }

Test-Pass "Router started on port $Port"

# ============================================================================
# Phase 3: Oversized body test
# ============================================================================
Write-Step "Oversized body rejection test"
$maxBytes = $Results.MaxBodyBytes
$oversizedSize = $maxBytes + 1  # 1 byte over limit
Write-Host "  Sending body of size ~$($oversizedSize + 200) bytes (> limit $maxBytes)" -ForegroundColor DarkGray

$tempFile = [System.IO.Path]::GetTempFileName()
try {
  # Create a JSON payload with a large padding field
  $padding = 'x' * ($maxBytes + 100)  # Must exceed maxBodyBytes total including JSON overhead
  $json = "{`"profile`":`"phi-4`",`"pad`":`"$padding`"}"
  $json | Set-Content -Path $tempFile -Encoding Ascii -NoNewline
  $actualSize = (Get-Item $tempFile).Length
  Write-Host "  Actual payload size: $actualSize bytes" -ForegroundColor DarkGray

  $response = curl.exe -s --connect-timeout 15 -w "%{http_code}" -X POST "http://127.0.0.1:$Port/backend/select" -H "Content-Type: application/json" -d "@$tempFile" 2>$null

  if ($response -match '^(.*?)(\d{3})$') {
    $statusCode = [int]$Matches[2]
    if ($statusCode -eq 413) {
      Test-Pass "Oversized request (> $maxBytes bytes) returns 413"
      $Results.OversizedRefused = $true
    } else {
      Test-Fail "Oversized request" "Expected 413, got $statusCode"
      $Results.OversizedRefused = $false
    }
  } else {
    Test-Fail "Could not parse status from curl response"
  }
} finally { Remove-Item -Path $tempFile -Force -ErrorAction SilentlyContinue }

# ============================================================================
# Phase 4: Normal-sized body test
# ============================================================================
Write-Step "Normal-sized body acceptance"
$tempFile2 = [System.IO.Path]::GetTempFileName()
try {
  # Send a small body (< max) to a nonexistent profile (expect 403, not 413)
  $json = '{"profile":"__size_test__","test":"small_body"}'
  $json | Set-Content -Path $tempFile2 -Encoding Ascii -NoNewline
  $response = curl.exe -s --connect-timeout 5 -w "%{http_code}" -X POST "http://127.0.0.1:$Port/backend/select" -H "Content-Type: application/json" -d "@$tempFile2" 2>$null

  if ($response -match '^(.*?)(\d{3})$') {
    $statusCode = [int]$Matches[2]
    # Expect 403 (nonexistent profile) NOT 413 (oversized) or 4xx-other
    if ($statusCode -eq 403) {
      Test-Pass "Normal-sized body accepted (returns 403 as expected for nonexistent profile)"
      $Results.NormalSizedAccepted = $true
    } elseif ($statusCode -eq 413) {
      Test-Fail "Normal-sized body incorrectly rejected as 413"
    } else {
      Test-Pass "Normal-sized body not rejected as 413 (status: $statusCode)"
      $Results.NormalSizedAccepted = $true
    }
  } else {
    Test-Fail "Could not parse status from curl response"
  }
} finally { Remove-Item -Path $tempFile2 -Force -ErrorAction SilentlyContinue }

# ============================================================================
# Phase 5: Verify request limit in server.rs
# ============================================================================
Write-Step "Source limit verification"
$serverLine = Get-Content "$RouterDir\src\server.rs" | Select-String "DefaultBodyLimit::max"
if ($serverLine) {
  Test-Pass "DefaultBodyLimit configured in server.rs: $($serverLine.ToString().Trim())"
} else {
  Test-Fail "Could not find DefaultBodyLimit configuration"
}

# ============================================================================
# Cleanup
# ============================================================================
Write-Step "Cleanup"
if ($proc -and !$proc.HasExited) { $proc.Kill(); Start-Sleep -Seconds 2 }
Remove-Item Env:\ROUTER_MAX_BODY_BYTES -ErrorAction SilentlyContinue

$listenersFinal = netstat -ano | Select-String ":$Port\s" | Select-String "LISTENING"
if ($listenersFinal) { Write-Host ("  Port " + $Port + " still in use (cleaning up)") -ForegroundColor Yellow } else { Write-Host ("  Port " + $Port + ": free") -ForegroundColor Green }

# ============================================================================
# Summary
# ============================================================================
$total = $script:Passed + $script:Failed
Write-Step "Dimension 6 Summary"
Write-Host "  Max body bytes: $($Results.MaxBodyBytes) ($([math]::Round($Results.MaxBodyBytes / 1024 / 1024, 1)) MB)" -ForegroundColor DarkGray
Write-Host "  Oversized refused: $($Results.OversizedRefused)" -ForegroundColor DarkGray
Write-Host "  Normal-sized accepted: $($Results.NormalSizedAccepted)" -ForegroundColor DarkGray
Write-Host "  $($script:Passed) passed, $($script:Failed) failed ($total total)" -ForegroundColor $(if($script:Failed -eq 0){"Green"}else{"Red"})

$Results.Passed = $script:Passed
$Results.Failed = $script:Failed
$Results
