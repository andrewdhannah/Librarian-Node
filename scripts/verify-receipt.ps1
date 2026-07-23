<#
.SYNOPSIS
  Verifier for Windows Runtime Node Integration Receipts (v2 and v1 backward-compatible).

.DESCRIPTION
  Reads a receipt JSON file and validates:
    - Schema version compatibility
    - Required field presence
    - Field type correctness (v2 rules if win-runtime-receipt/v2)
    - Artifact SHA-256 format (v2)
    - listener_active + connectivity semantics (v2)
    - token_logged must be false
    - Derived overall matches evidence

  Exit code 0 = all checks passed
  Exit code 1 = one or more checks failed

.PARAMETER ReceiptPath
  Path to the receipt JSON file.

.EXAMPLE
  .\scripts\verify-receipt.ps1 -ReceiptPath "G:\OpenWork\receipts\runtime-integration\receipt.json"
#>

param(
  [Parameter(Mandatory = $true)]
  [string]$ReceiptPath
)

function Write-Result {
  param([string]$Name, [bool]$Passed, [string]$Detail = "")
  $mark = if ($Passed) { "PASS" } else { "FAIL" }
  $color = if ($Passed) { "Green" } else { "Red" }
  $detailStr = if ($Detail) { " ($Detail)" } else { "" }
  Write-Host ("  $mark $Name$detailStr") -ForegroundColor $color
}

$Global:Passed = 0
$Global:Failed = 0
$Global:CheckCount = 0

function Check {
  param([string]$Name, [scriptblock]$Block)
  $Global:CheckCount++
  try {
    $result = & $Block
    if ($result -eq $true) {
      $Global:Passed++
      Write-Result -Name $Name -Passed $true
    } else {
      $Global:Failed++
      Write-Result -Name $Name -Passed $false -Detail $result
    }
  } catch {
    $Global:Failed++
    Write-Result -Name $Name -Passed $false -Detail $_.Exception.Message
  }
}

# ============================================================================
# Load receipt
# ============================================================================
Write-Host "=== Verifying Receipt ===" -ForegroundColor Cyan
Write-Host "File: $ReceiptPath" -ForegroundColor DarkGray

if (-not (Test-Path $ReceiptPath)) {
  Write-Host "FATAL: Receipt file not found: $ReceiptPath" -ForegroundColor Red
  exit 1
}

$raw = Get-Content $ReceiptPath -Raw -Encoding UTF8
$receipt = $raw | ConvertFrom-Json

# ============================================================================
# Check 1: Schema version
# ============================================================================
Write-Host ""
Write-Host "--- Schema ---" -ForegroundColor Cyan

Check -Name "schema_version present" -Block {
  if (-not $receipt.schema_version) { return "missing schema_version" }
  $true
}

$isV2 = $receipt.schema_version -eq "win-runtime-receipt/v2"
$isV1 = $receipt.schema_version -eq "win-runtime-receipt/v1"

Check -Name "schema_version is valid" -Block {
  if (-not ($isV1 -or $isV2)) {
    return "unknown schema_version '$($receipt.schema_version)'. Expected win-runtime-receipt/v1 or /v2"
  }
  $true
}

Check -Name "receipt_type is runtime_integration_proof" -Block {
  if ($receipt.receipt_type -ne "runtime_integration_proof") {
    return "expected 'runtime_integration_proof', got '$($receipt.receipt_type)'"
  }
  $true
}

Check -Name "created_at_utc is valid ISO 8601" -Block {
  try {
    $d = [datetime]::ParseExact($receipt.created_at_utc, "yyyy-MM-ddTHH:mm:ssZ", [System.Globalization.CultureInfo]::InvariantCulture, [System.Globalization.DateTimeStyles]::AssumeUniversal)
    if ($d.Kind -eq [System.DateTimeKind]::Utc -or $receipt.created_at_utc.EndsWith('Z')) { $true } else { "not UTC" }
  } catch { return "invalid date format: $_" }
}

# ============================================================================
# Check 2: Machine
# ============================================================================
Write-Host ""
Write-Host "--- Machine ---" -ForegroundColor Cyan

Check -Name "machine.role is windows_runtime_node" -Block {
  if ($receipt.machine.role -ne "windows_runtime_node") { return "expected windows_runtime_node" }
  $true
}
Check -Name "machine.service_name is LibrarianRunTimeNode" -Block {
  if ($receipt.machine.service_name -ne "LibrarianRunTimeNode") { return "expected LibrarianRunTimeNode" }
  $true
}
Check -Name "machine.service_start_type valid" -Block {
  if ($receipt.machine.service_start_type -notin @("Manual","Automatic","Disabled")) { return "invalid start type" }
  $true
}
Check -Name "machine.service_final_state valid" -Block {
  if ($receipt.machine.service_final_state -notin @("Stopped","Running","Paused")) { return "invalid final state" }
  $true
}

# ============================================================================
# Check 3: Repos
# ============================================================================
Write-Host ""
Write-Host "--- Repos ---" -ForegroundColor Cyan

Check -Name "repos.thelibrarian_main_head valid SHA" -Block {
  if ($receipt.repos.thelibrarian_main_head -notmatch '^[0-9a-f]{7,40}$') { return "invalid SHA format" }
  $true
}
Check -Name "repos.runtime_node_head valid SHA" -Block {
  if ($receipt.repos.runtime_node_head -notmatch '^[0-9a-f]{7,40}$') { return "invalid SHA format" }
  $true
}

# ============================================================================
# Check 4: Auth
# ============================================================================
Write-Host ""
Write-Host "--- Auth ---" -ForegroundColor Cyan

Check -Name "auth.token_source is environment" -Block {
  if ($receipt.auth.token_source -ne "environment") { return "expected environment" }
  $true
}
Check -Name "auth.token_logged is false" -Block {
  if ($receipt.auth.token_logged -ne $false) { return "token_logged MUST be false" }
  $true
}
Check -Name "auth.missing_token_status is 200 or 401" -Block {
  $v = $receipt.auth.missing_token_status
  if ($v -notin @(200, 401)) { return "expected 200 (auth disabled) or 401 (auth enabled), got $v" }
  $true
}
Check -Name "auth.invalid_token_status is 200 or 401" -Block {
  $v = $receipt.auth.invalid_token_status
  if ($v -notin @(200, 401)) { return "expected 200 (auth disabled) or 401 (auth enabled), got $v" }
  $true
}

# Token safety: scan entire receipt for bearer/secret/token values
Check -Name "No token/bearer/secret in receipt body" -Block {
  # Remove known safe fields from scan
  $strip = $raw -replace '"token_logged":\s*(true|false)', ''
  $strip = $strip -replace '"token_source":\s*"[^"]*"', ''
  $strip = $strip -replace '"missing_token_status":\s*\d+', ''
  $strip = $strip -replace '"invalid_token_status":\s*\d+', ''
  if ($strip -match "(?i)(bearer\s+[a-z0-9_-]{20,}|secret[^`"=]*[:=]\s*[`"']?)") {
    return "potential secret or bearer token found in receipt body"
  }
  $true
}

# ============================================================================
# Check 5: Lifecycle
# ============================================================================
Write-Host ""
Write-Host "--- Lifecycle ---" -ForegroundColor Cyan

Check -Name "lifecycle.selected_profile present" -Block {
  if ([string]::IsNullOrEmpty($receipt.lifecycle.selected_profile)) { return "missing" }
  $true
}
Check -Name "lifecycle.chat_prompt non-empty" -Block {
  if ([string]::IsNullOrEmpty($receipt.lifecycle.chat_prompt)) { return "missing" }
  if ($receipt.lifecycle.chat_prompt.Length -gt 200) { return "prompt too long (>200 chars)" }
  $true
}
Check -Name "lifecycle.chat_expected present" -Block {
  if ([string]::IsNullOrEmpty($receipt.lifecycle.chat_expected)) { return "missing" }
  $true
}
Check -Name "lifecycle.chat_observed present" -Block {
  if ([string]::IsNullOrEmpty($receipt.lifecycle.chat_observed)) { return "missing" }
  $true
}
Check -Name "lifecycle.stop_called is boolean" -Block {
  if ($null -eq $receipt.lifecycle.stop_called -or $receipt.lifecycle.stop_called -isnot [bool]) {
    return "not boolean"
  }
  $true
}

# ============================================================================
# Check 6: Endpoints
# ============================================================================
Write-Host ""
Write-Host "--- Endpoints ---" -ForegroundColor Cyan

$endpoints = @("status","profiles","health","models","select","chat","stop")
foreach ($ep in $endpoints) {
  Check -Name "endpoints.$ep is pass/fail" -Block {
    if ($receipt.endpoints.$ep -notin @("pass","fail")) { return "expected pass or fail" }
    $true
  }
}

# ============================================================================
# Check 7: Cleanup (v2 extended)
# ============================================================================
Write-Host ""
Write-Host "--- Cleanup ---" -ForegroundColor Cyan

Check -Name "cleanup.backend_processes_observed_during_run >= 0" -Block {
  $v = $receipt.cleanup.backend_processes_observed_during_run
  if ($v -isnot [int] -or $v -lt 0) { return "invalid value: $v" }
  $true
}
Check -Name "cleanup.backend_orphans_after_stop >= 0" -Block {
  $v = $receipt.cleanup.backend_orphans_after_stop
  if ($v -isnot [int] -or $v -lt 0) { return "invalid value: $v" }
  $true
}
Check -Name "cleanup.cleanup_retry_seconds >= 0" -Block {
  $v = $receipt.cleanup.cleanup_retry_seconds
  if ($v -isnot [int] -or $v -lt 0) { return "invalid value: $v" }
  $true
}

# v2-specific cleanup checks
if ($isV2) {
  Check -Name "cleanup.listener_active is boolean" -Block {
    if ($null -eq $receipt.cleanup.listener_active -or $receipt.cleanup.listener_active -isnot [bool]) {
      return "not boolean"
    }
    $true
  }
  Check -Name "cleanup.connectivity is refused/listening/unknown" -Block {
    if ($receipt.cleanup.connectivity -notin @("refused","listening","unknown")) {
      return "invalid connectivity value"
    }
    $true
  }
  Check -Name "cleanup.port_check_method is listener_filter_and_tcp_connect" -Block {
    if ($receipt.cleanup.port_check_method -ne "listener_filter_and_tcp_connect") {
      return "expected listener_filter_and_tcp_connect"
    }
    $true
  }

  # Derived consistency checks
  Check -Name "cleanup.port_9130_free_after_stop consistent with listener+connectivity" -Block {
    $free = $receipt.cleanup.port_9130_free_after_stop
    $listener = $receipt.cleanup.listener_active
    $connect = $receipt.cleanup.connectivity
    $expectedFree = (-not $listener) -and ($connect -eq "refused")
    if ($free -ne $expectedFree) {
      return "port_9130_free_after_stop=$free but listener=$listener connectivity=$connect (expected port_free=$expectedFree)"
    }
    $true
  }

  # If cleanup passed, listener must be false and connectivity must be refused
  if ($receipt.cleanup_passed) {
    Check -Name "cleanup_passed => listener_active must be false" -Block {
      if ($receipt.cleanup.listener_active -ne $false) { return "listener_active should be false when cleanup passed" }
      $true
    }
    Check -Name "cleanup_passed => connectivity must be refused" -Block {
      if ($receipt.cleanup.connectivity -ne "refused") { return "connectivity should be refused when cleanup passed" }
      $true
    }
  }
} else {
  # v1: skip v2-specific cleanup checks but note it
  Write-Host "  (v1 receipt: skipping v2-specific cleanup checks)" -ForegroundColor DarkGray
}

# ============================================================================
# Check 8: Artifact (v2 only; not required for v1)
# ============================================================================
Write-Host ""
Write-Host "--- Artifact ---" -ForegroundColor Cyan

if ($isV2) {
  Check -Name "artifact present" -Block {
    if ($null -eq $receipt.artifact) { return "artifact section missing" }
    $true
  }
  Check -Name "artifact.router_binary_path non-empty" -Block {
    if ([string]::IsNullOrEmpty($receipt.artifact.router_binary_path)) { return "missing" }
    $true
  }
  Check -Name "artifact.router_binary_sha256 valid format" -Block {
    $h = $receipt.artifact.router_binary_sha256
    if ($h -cnotmatch '^[0-9A-F]{64}$') {
      return "SHA-256 must be exactly 64 uppercase hex characters, got '$h'"
    }
    $true
  }
  Check -Name "artifact.router_binary_modified_utc valid ISO 8601" -Block {
    try {
      $d = [datetime]::ParseExact($receipt.artifact.router_binary_modified_utc, "yyyy-MM-ddTHH:mm:ssZ", $null, [System.Globalization.DateTimeStyles]::AssumeUniversal)
      $true
    } catch { return "invalid date format: $_" }
  }
  Check -Name "artifact.router_implementation is rust or python" -Block {
    if ($receipt.artifact.router_implementation -notin @("rust","python")) {
      return "unexpected implementation: $($receipt.artifact.router_implementation)"
    }
    $true
  }
  Check -Name "artifact.governed_path_match is boolean" -Block {
    if ($null -eq $receipt.artifact.governed_path_match -or $receipt.artifact.governed_path_match -isnot [bool]) {
      return "not boolean"
    }
    $true
  }
} else {
  Write-Host "  (v1 receipt: artifact section not required)" -ForegroundColor DarkGray
}

# ============================================================================
# Check 9: Result
# ============================================================================
Write-Host ""
Write-Host "--- Result ---" -ForegroundColor Cyan

Check -Name "result.unauthorized_passed >= 0" -Block {
  $v = $receipt.result.unauthorized_passed; if ($v -isnot [int] -or $v -lt 0) { return "invalid" }; $true
}
Check -Name "result.unauthorized_total >= 0" -Block {
  $v = $receipt.result.unauthorized_total; if ($v -isnot [int] -or $v -lt 0) { return "invalid" }; $true
}
Check -Name "result.authenticated_passed >= 0" -Block {
  $v = $receipt.result.authenticated_passed; if ($v -isnot [int] -or $v -lt 0) { return "invalid" }; $true
}
Check -Name "result.authenticated_total >= 0" -Block {
  $v = $receipt.result.authenticated_total; if ($v -isnot [int] -or $v -lt 0) { return "invalid" }; $true
}
Check -Name "result.cleanup_passed is boolean" -Block {
  if ($null -eq $receipt.result.cleanup_passed -or $receipt.result.cleanup_passed -isnot [bool]) {
    return "not boolean"
  }
  $true
}
Check -Name "result.overall is pass/partial/fail" -Block {
  if ($receipt.result.overall -notin @("pass","partial","fail")) {
    return "invalid overall value"
  }
  $true
}

# Overall consistency
$allEndpointsPass = ($endpoints | ForEach-Object { $receipt.endpoints.$_ -eq "pass" }) -notcontains $false
Check -Name "consistency: all endpoints pass => authenticated_passed matches total" -Block {
  $epPassCount = ($endpoints | Where-Object { $receipt.endpoints.$_ -eq "pass" }).Count
  if ($receipt.result.authenticated_passed -ne $epPassCount) {
    return "endpoint passes=$epPassCount but result says $($receipt.result.authenticated_passed)"
  }
  $true
}
Check -Name "consistency: overall matches evidence" -Block {
  $allPass = $allEndpointsPass -and $receipt.result.unauthorized_passed -eq $receipt.result.unauthorized_total -and $receipt.result.cleanup_passed
  $anyPass = ($receipt.result.unauthorized_passed -gt 0) -or ($receipt.result.authenticated_passed -gt 0)
  $expected = if ($allPass) { "pass" } elseif ($anyPass) { "partial" } else { "fail" }
  if ($receipt.result.overall -ne $expected) {
    return "evidence says '$expected' but receipt says '$($receipt.result.overall)'"
  }
  $true
}

# ============================================================================
# Summary
# ============================================================================
Write-Host ""
Write-Host "=== Summary === " -ForegroundColor Cyan
$totalStr = "$($Global:Passed) passed, $($Global:Failed) failed ($($Global:CheckCount) total checks)"
if ($Global:Failed -eq 0) {
  Write-Host $totalStr -ForegroundColor Green
  Write-Host "RESULT: VERIFIED" -ForegroundColor Green
  exit 0
} else {
  Write-Host $totalStr -ForegroundColor Red
  Write-Host "RESULT: FAILED" -ForegroundColor Red
  exit 1
}
