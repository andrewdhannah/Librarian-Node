<#
.SYNOPSIS
  Verifier/Gate for WIN-RUNTIME-QUALIFICATION-1 qualification records.

.DESCRIPTION
  Validates a qualification record JSON and optionally runs gate checks
  against the source v2 receipt.

  Gate mode checks (exit 1 on failure):
    - Qualification record schema structure
    - Source receipt artifact section must exist (QUAL-006)
    - Source receipt artifact hash must be valid format (QUAL-007)
    - Qualification record has required fields
    - Machine final state consistent

  Does NOT fail on hash mismatch -- that is honestly recorded (QUAL-004).

.PARAMETER QualificationPath
  Path to the qualification record JSON.

.PARAMETER ReceiptPath
  Path to the source v2 receipt to cross-validate against.

.PARAMETER GateMode
  Run strict gate checks; exit 1 on gate violations.

.PARAMETER SkipReceiptCheck
  Skip cross-validation against the receipt (just validate the qualification record).

.EXAMPLE
  .\scripts\verify-runtime-qualification.ps1 -QualificationPath "G:\OpenWork\receipts\runtime-qualification\record.json"
  .\scripts\verify-runtime-qualification.ps1 -QualificationPath "G:\OpenWork\receipts\runtime-qualification\record.json" -ReceiptPath "G:\OpenWork\receipts\runtime-integration\receipt.json" -GateMode
#>

param(
  [Parameter(Mandatory = $true)]
  [string]$QualificationPath,

  [string]$ReceiptPath = "",

  [switch]$GateMode = $false,

  [switch]$SkipReceiptCheck = $false
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
# Load qualification record
# ============================================================================
Write-Host "=== Verifying Qualification Record ===" -ForegroundColor Cyan
Write-Host "File: $QualificationPath" -ForegroundColor DarkGray

if (-not (Test-Path $QualificationPath)) {
  Write-Host "FATAL: Qualification record not found: $QualificationPath" -ForegroundColor Red
  exit 1
}

$raw = Get-Content $QualificationPath -Raw -Encoding UTF8
$qual = $raw | ConvertFrom-Json

# ============================================================================
# Gate: Schema structure (QUAL-005)
# ============================================================================
Write-Host ""
Write-Host "--- Schema ---" -ForegroundColor Cyan

Check -Name "schema_version is win-runtime-qualification/v1" -Block {
  if ($qual.schema_version -ne "win-runtime-qualification/v1") {
    return "expected 'win-runtime-qualification/v1', got '$($qual.schema_version)'"
  }
  $true
}

Check -Name "record_type is runtime_qualification_proof" -Block {
  if ($qual.record_type -ne "runtime_qualification_proof") {
    return "expected 'runtime_qualification_proof', got '$($qual.record_type)'"
  }
  $true
}

Check -Name "created_at_utc is valid ISO 8601" -Block {
  try {
    $d = [datetime]::ParseExact($qual.created_at_utc, "yyyy-MM-ddTHH:mm:ssZ", [System.Globalization.CultureInfo]::InvariantCulture, [System.Globalization.DateTimeStyles]::AssumeUniversal)
    if ($d.Kind -eq [System.DateTimeKind]::Utc -or $qual.created_at_utc.EndsWith('Z')) { $true } else { "not UTC" }
  } catch { return "invalid date format: $_" }
}

# ============================================================================
# Gate: Source section
# ============================================================================
Write-Host ""
Write-Host "--- Source ---" -ForegroundColor Cyan

Check -Name "source.receipt_path present" -Block {
  if ([string]::IsNullOrEmpty($qual.source.receipt_path)) { return "missing" }
  if (-not (Test-Path $qual.source.receipt_path)) { return "file not found: $($qual.source.receipt_path)" }
  $true
}

Check -Name "source.receipt_schema_version present" -Block {
  if ([string]::IsNullOrEmpty($qual.source.receipt_schema_version)) { return "missing" }
  $true
}

Check -Name "source.receipt_runtime_head valid SHA" -Block {
  if ($qual.source.receipt_runtime_head -notmatch '^[0-9a-f]{7,40}$') { return "invalid SHA format" }
  $true
}

Check -Name "source.receipt_thelibrarian_head valid SHA" -Block {
  if ($qual.source.receipt_thelibrarian_head -notmatch '^[0-9a-f]{7,40}$') { return "invalid SHA format" }
  $true
}

# ============================================================================
# Gate: Rebuild section
# ============================================================================
Write-Host ""
Write-Host "--- Rebuild ---" -ForegroundColor Cyan

Check -Name "rebuild.source_head present" -Block {
  if ([string]::IsNullOrEmpty($qual.rebuild.source_head)) { return "missing" }
  $true
}

Check -Name "rebuild.binary_sha256 valid format" -Block {
  $h = $qual.rebuild.binary_sha256
  if ($h -cnotmatch '^[0-9A-F]{64}$') {
    return "SHA-256 must be exactly 64 uppercase hex characters, got '$h'"
  }
  $true
}

Check -Name "rebuild.binary_path present" -Block {
  if ([string]::IsNullOrEmpty($qual.rebuild.binary_path)) { return "missing" }
  if (-not (Test-Path $qual.rebuild.binary_path)) { return "binary not found at path: $($qual.rebuild.binary_path)" }
  $true
}

Check -Name "rebuild.binary_modified_utc valid ISO 8601" -Block {
  try {
    $d = [datetime]::ParseExact($qual.rebuild.binary_modified_utc, "yyyy-MM-ddTHH:mm:ssZ", $null, [System.Globalization.DateTimeStyles]::AssumeUniversal)
    $true
  } catch { return "invalid date format: $_" }
}

Check -Name "rebuild.binary_size_bytes > 0" -Block {
  if ($qual.rebuild.binary_size_bytes -le 0) { return "invalid size: $($qual.rebuild.binary_size_bytes)" }
  $true
}

# ============================================================================
# Gate: Comparison section
# ============================================================================
Write-Host ""
Write-Host "--- Comparison ---" -ForegroundColor Cyan

Check -Name "comparison.rebuilt_hash_matches_receipt is boolean" -Block {
  if ($null -eq $qual.comparison.rebuilt_hash_matches_receipt -or $qual.comparison.rebuilt_hash_matches_receipt -isnot [bool]) {
    return "not boolean"
  }
  $true
}

Check -Name "comparison.reason present" -Block {
  if ([string]::IsNullOrEmpty($qual.comparison.reason)) { return "missing reason" }
  $true
}

Check -Name "comparison.receipt_artifact_sha256 valid format" -Block {
  $h = $qual.comparison.receipt_artifact_sha256
  if ($h -cnotmatch '^[0-9A-F]{64}$') { return "invalid SHA-256 format: '$h'" }
  $true
}

Check -Name "comparison.rebuilt_artifact_sha256 valid format" -Block {
  $h = $qual.comparison.rebuilt_artifact_sha256
  if ($h -cnotmatch '^[0-9A-F]{64}$') { return "invalid SHA-256 format: '$h'" }
  $true
}

Check -Name "comparison hash consistency" -Block {
  $matchesReceipt = $qual.comparison.rebuilt_hash_matches_receipt
  $receiptHash = $qual.comparison.receipt_artifact_sha256
  $rebuiltHash = $qual.comparison.rebuilt_artifact_sha256
  $actualMatch = ($receiptHash -eq $rebuiltHash)
  if ($matchesReceipt -ne $actualMatch) {
    return "rebuilt_hash_matches_receipt=$matchesReceipt but receipt=$receiptHash rebuilt=$rebuiltHash (actual match=$actualMatch)"
  }
  $true
}

# ============================================================================
# Gate: Build metadata section
# ============================================================================
Write-Host ""
Write-Host "--- Build Metadata ---" -ForegroundColor Cyan

Check -Name "build_metadata.cargo_version present" -Block {
  if ([string]::IsNullOrEmpty($qual.build_metadata.cargo_version)) { return "missing" }
  $true
}

Check -Name "build_metadata.rustc_version present" -Block {
  if ([string]::IsNullOrEmpty($qual.build_metadata.rustc_version)) { return "missing" }
  $true
}

Check -Name "build_metadata.target_triple present" -Block {
  if ([string]::IsNullOrEmpty($qual.build_metadata.target_triple)) { return "missing" }
  $true
}

Check -Name "build_metadata.profile present" -Block {
  if ([string]::IsNullOrEmpty($qual.build_metadata.profile)) { return "missing" }
  $true
}

Check -Name "build_metadata.build_command present" -Block {
  if ([string]::IsNullOrEmpty($qual.build_metadata.build_command)) { return "missing" }
  $true
}

Check -Name "build_metadata.source_head matches rebuild.source_head" -Block {
  if ($qual.build_metadata.source_head -ne $qual.rebuild.source_head) {
    return "mismatch: build_metadata.source_head='$($qual.build_metadata.source_head)' vs rebuild.source_head='$($qual.rebuild.source_head)'"
  }
  $true
}

# ============================================================================
# Gate: Machine section
# ============================================================================
Write-Host ""
Write-Host "--- Machine State ---" -ForegroundColor Cyan

Check -Name "machine.role is windows_runtime_node" -Block {
  if ($qual.machine.role -ne "windows_runtime_node") { return "expected windows_runtime_node" }
  $true
}

Check -Name "machine.service_name is LibrarianRunTimeNode" -Block {
  if ($qual.machine.service_name -ne "LibrarianRunTimeNode") { return "expected LibrarianRunTimeNode" }
  $true
}

Check -Name "machine.service_start_type is Manual" -Block {
  if ($qual.machine.service_start_type -ne "Manual") { return "expected Manual, got $($qual.machine.service_start_type)" }
  $true
}

Check -Name "machine.service_final_state is Stopped" -Block {
  if ($qual.machine.service_final_state -ne "Stopped") { return "expected Stopped, got $($qual.machine.service_final_state)" }
  $true
}

Check -Name "machine.port_9130_free is true" -Block {
  if ($qual.machine.port_9130_free -ne $true) { return "port 9130 not free" }
  $true
}

Check -Name "machine.orphan_llama_server_count is 0" -Block {
  if ($qual.machine.orphan_llama_server_count -ne 0) { return "orphans present: $($qual.machine.orphan_llama_server_count)" }
  $true
}

Check -Name "machine.git_working_trees_clean is true" -Block {
  if ($qual.machine.git_working_trees_clean -ne $true) { return "working trees not clean" }
  $true
}

Check -Name "machine.stashes_empty is true" -Block {
  if ($qual.machine.stashes_empty -ne $true) { return "stashes not empty" }
  $true
}

# ============================================================================
# Gate: Result section
# ============================================================================
Write-Host ""
Write-Host "--- Result ---" -ForegroundColor Cyan

Check -Name "result.overall is pass or fail" -Block {
  if ($qual.result.overall -notin @("pass", "fail")) { return "unexpected overall: $($qual.result.overall)" }
  $true
}

Check -Name "result.total_checks_failed >= 0" -Block {
  if ($qual.result.total_checks_failed -lt 0) { return "negative" }
  $true
}

# Overall consistency
Check -Name "result overall consistent with failed count" -Block {
  $expectedOverall = if ($qual.result.total_checks_failed -eq 0) { "pass" } else { "fail" }
  if ($qual.result.overall -ne $expectedOverall) {
    return "overall='$($qual.result.overall)' but total_checks_failed=$($qual.result.total_checks_failed)"
  }
  $true
}

# ============================================================================
# Optional: Cross-validate against source receipt
# ============================================================================
if (-not $SkipReceiptCheck -and -not [string]::IsNullOrEmpty($ReceiptPath)) {
  Write-Host ""
  Write-Host "--- Cross-Validation with Receipt ---" -ForegroundColor Cyan

  if (Test-Path $ReceiptPath) {
    $receiptRaw = Get-Content $ReceiptPath -Raw -Encoding UTF8
    $receipt = $receiptRaw | ConvertFrom-Json

    # QUAL-006: Gate fails on missing artifact proof
    Check -Name "RECEIPT GATE: artifact section exists" -Block {
      if ($null -eq $receipt.artifact) { return "MISSING ARTIFACT PROOF -- receipt has no artifact section" }
      $true
    }

    # QUAL-007: Gate fails on malformed hash
    if ($null -ne $receipt.artifact) {
      Check -Name "RECEIPT GATE: artifact SHA-256 format valid" -Block {
        $h = $receipt.artifact.router_binary_sha256
        if ($h -cnotmatch '^[0-9A-F]{64}$') { return "MALFORMED HASH -- '$h' is not 64 uppercase hex chars" }
        $true
      }

      Check -Name "receipt artifact hash matches qualification record" -Block {
        $receiptHash = $receipt.artifact.router_binary_sha256
        $qualReceiptHash = $qual.comparison.receipt_artifact_sha256
        if ($receiptHash -ne $qualReceiptHash) {
          return "mismatch: receipt says $receiptHash, qualification record says $qualReceiptHash"
        }
        $true
      }
    }

    Check -Name "receipt HEAD matches qualification source" -Block {
      $receiptRuntimeHead = $receipt.repos.runtime_node_head
      $qualReceiptHead = $qual.source.receipt_runtime_head
      if ($receiptRuntimeHead -ne $qualReceiptHead) {
        return "mismatch: receipt runtime HEAD=$receiptRuntimeHead, qual source=$qualReceiptHead"
      }
      $true
    }
  } else {
    Write-Host "  (Receipt path not found: $ReceiptPath -- skipping cross-validation)" -ForegroundColor DarkGray
  }
} else {
  Write-Host ""
  Write-Host "--- Receipt Cross-Validation ---" -ForegroundColor Cyan
  Write-Host "  (Skipped: no receipt path provided or SkipReceiptCheck set)" -ForegroundColor DarkGray
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
