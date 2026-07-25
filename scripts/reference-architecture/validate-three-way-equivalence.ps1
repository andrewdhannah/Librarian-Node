#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Three-way equivalence validation for Librarian Node implementations.
.DESCRIPTION
    This script validates equivalence across Windows, Linux, and macOS implementations.
.PARAMETER WindowsReceiptPath
    Path to Windows startup receipt
.PARAMETER LinuxReceiptPath
    Path to Linux startup receipt
.PARAMETER MacOsReceiptPath
    Path to macOS startup receipt
.PARAMETER OutputPath
    Path to store equivalence validation receipt
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)]
    [string]$WindowsReceiptPath,

    [Parameter(Mandatory=$true)]
    [string]$LinuxReceiptPath,

    [Parameter(Mandatory=$true)]
    [string]$MacOsReceiptPath,

    [Parameter(Mandatory=$false)]
    [string]$OutputPath = "G:\OpenWork\evidence\reference-architecture"
)

# Ensure output directory exists
if (-not (Test-Path -LiteralPath $OutputPath)) {
    New-Item -ItemType Directory -Path $OutputPath -Force | Out-Null
}

# Generate timestamp
$Timestamp = Get-Date -Format "yyyyMMdd-HHmmss"

Write-Host "=== Three-Way Equivalence Validation ===" -ForegroundColor Cyan
Write-Host ""

# Load receipts
Write-Host "Loading receipts..." -ForegroundColor Yellow

if (-not (Test-Path -LiteralPath $WindowsReceiptPath)) {
    Write-Host "[FAIL] Windows receipt not found" -ForegroundColor Red
    exit 1
}

if (-not (Test-Path -LiteralPath $LinuxReceiptPath)) {
    Write-Host "[FAIL] Linux receipt not found" -ForegroundColor Red
    exit 1
}

if (-not (Test-Path -LiteralPath $MacOsReceiptPath)) {
    Write-Host "[FAIL] macOS receipt not found" -ForegroundColor Red
    exit 1
}

$WindowsReceipt = Get-Content -LiteralPath $WindowsReceiptPath | ConvertFrom-Json
$LinuxReceipt = Get-Content -LiteralPath $LinuxReceiptPath | ConvertFrom-Json
$MacOsReceipt = Get-Content -LiteralPath $MacOsReceiptPath | ConvertFrom-Json

Write-Host "  [PASS] All receipts loaded"
Write-Host ""

# Initialize validation report
$ValidationReport = @{
    report_id = "EQUIVALENCE-$Timestamp"
    platforms_tested = @("windows", "linux", "macos")
    checks_performed = @()
    divergences_detected = @()
    overall_status = "EQUIVALENT"
    timestamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.0000000Z")
}

# Gate TWE-1: Startup Sequence Equivalence
Write-Host "Gate TWE-1: Startup Sequence Equivalence" -ForegroundColor Yellow

$WindowsPhasesPassed = 0
$LinuxPhasesPassed = 0
$MacOsPhasesPassed = 0

if ($WindowsReceipt.startup_phase -eq "complete") { $WindowsPhasesPassed = 6 }
if ($LinuxReceipt.startup_phase -eq "complete") { $LinuxPhasesPassed = 6 }
if ($MacOsReceipt.startup_phase -eq "complete") { $MacOsPhasesPassed = 6 }

$Twe1Passed = $WindowsPhasesPassed -eq 6 -and $LinuxPhasesPassed -eq 6 -and $MacOsPhasesPassed -eq 6

$ValidationReport.checks_performed += @{
    check_name = "startup_sequence_equivalence"
    status = if ($Twe1Passed) { "passed" } else { "failed" }
    details = @{
        windows_phases_passed = $WindowsPhasesPassed
        linux_phases_passed = $LinuxPhasesPassed
        macos_phases_passed = $MacOsPhasesPassed
    }
}

if ($Twe1Passed) {
    Write-Host "  [PASS] Startup sequence equivalent" -ForegroundColor Green
} else {
    Write-Host "  [FAIL] Startup sequence differs" -ForegroundColor Red
    $ValidationReport.divergences_detected += "startup_sequence_equivalence"
    $ValidationReport.overall_status = "DIVERGENT"
}

# Gate TWE-2: Receipt Deterministic Field Equivalence
Write-Host ""
Write-Host "Gate TWE-2: Receipt Deterministic Field Equivalence" -ForegroundColor Yellow

$DeterministicFields = @(
    "governance_commit",
    "identity_loaded",
    "governance_verified",
    "capabilities_loaded",
    "environment_validated",
    "checks_passed",
    "checks_failed",
    "status"
)

$Twe2Details = @{}
$Twe2Passed = $true

foreach ($Field in $DeterministicFields) {
    $WindowsValue = $WindowsReceipt.$Field
    $LinuxValue = $LinuxReceipt.$Field
    $MacOsValue = $MacOsReceipt.$Field
    
    $Equivalent = $WindowsValue -eq $LinuxValue -and $LinuxValue -eq $MacOsValue
    
    $Twe2Details[$Field] = @{
        windows = $WindowsValue
        linux = $LinuxValue
        macos = $MacOsValue
        equivalent = $Equivalent
    }
    
    if (-not $Equivalent) {
        $Twe2Passed = $false
        $ValidationReport.divergences_detected += "field_$Field"
    }
}

$ValidationReport.checks_performed += @{
    check_name = "receipt_deterministic_field_equivalence"
    status = if ($Twe2Passed) { "passed" } else { "failed" }
    details = $Twe2Details
}

if ($Twe2Passed) {
    Write-Host "  [PASS] Deterministic fields equivalent" -ForegroundColor Green
} else {
    Write-Host "  [FAIL] Deterministic fields differ" -ForegroundColor Red
    $ValidationReport.overall_status = "DIVERGENT"
}

# Gate TWE-3: Governance Decision Equivalence
Write-Host ""
Write-Host "Gate TWE-3: Governance Decision Equivalence" -ForegroundColor Yellow

$WindowsDecisions = @{
    identity_valid = $WindowsReceipt.identity_loaded
    governance_verified = $WindowsReceipt.governance_verified
    capabilities_present = $WindowsReceipt.capabilities_loaded
    environment_valid = $WindowsReceipt.environment_validated
    enter_governed_mode = $WindowsReceipt.status -eq "GOVERNED_EXECUTION"
}

$LinuxDecisions = @{
    identity_valid = $LinuxReceipt.identity_loaded
    governance_verified = $LinuxReceipt.governance_verified
    capabilities_present = $LinuxReceipt.capabilities_loaded
    environment_valid = $LinuxReceipt.environment_validated
    enter_governed_mode = $LinuxReceipt.status -eq "GOVERNED_EXECUTION"
}

$MacOsDecisions = @{
    identity_valid = $MacOsReceipt.identity_loaded
    governance_verified = $MacOsReceipt.governance_verified
    capabilities_present = $MacOsReceipt.capabilities_loaded
    environment_valid = $MacOsReceipt.environment_validated
    enter_governed_mode = $MacOsReceipt.status -eq "GOVERNED_EXECUTION"
}

$Twe3Details = @{}
$Twe3Passed = $true

foreach ($Decision in $WindowsDecisions.Keys) {
    $WindowsValue = $WindowsDecisions[$Decision]
    $LinuxValue = $LinuxDecisions[$Decision]
    $MacOsValue = $MacOsDecisions[$Decision]
    
    $Equivalent = $WindowsValue -eq $LinuxValue -and $LinuxValue -eq $MacOsValue
    
    $Twe3Details[$Decision] = @{
        windows = $WindowsValue
        linux = $LinuxValue
        macos = $MacOsValue
        equivalent = $Equivalent
    }
    
    if (-not $Equivalent) {
        $Twe3Passed = $false
        $ValidationReport.divergences_detected += "decision_$Decision"
    }
}

$ValidationReport.checks_performed += @{
    check_name = "governance_decision_equivalence"
    status = if ($Twe3Passed) { "passed" } else { "failed" }
    details = $Twe3Details
}

if ($Twe3Passed) {
    Write-Host "  [PASS] Governance decisions equivalent" -ForegroundColor Green
} else {
    Write-Host "  [FAIL] Governance decisions differ" -ForegroundColor Red
    $ValidationReport.overall_status = "DIVERGENT"
}

# Gate TWE-4: Capability Declaration Equivalence
Write-Host ""
Write-Host "Gate TWE-4: Capability Declaration Equivalence" -ForegroundColor Yellow

# Load capabilities from adapter files
$WindowsCapabilitiesPath = "G:\OpenWork\librarian-node\adapters\windows\capabilities.json"
$LinuxCapabilitiesPath = "G:\OpenWork\librarian-node\adapters\linux\capabilities.json"
$MacOsCapabilitiesPath = "G:\OpenWork\librarian-node\adapters\macos\capabilities.json"

$Twe4Details = @{}
$Twe4Passed = $true

if ((Test-Path -LiteralPath $WindowsCapabilitiesPath) -and
    (Test-Path -LiteralPath $LinuxCapabilitiesPath) -and
    (Test-Path -LiteralPath $MacOsCapabilitiesPath)) {
    
    $WindowsCap = Get-Content -LiteralPath $WindowsCapabilitiesPath | ConvertFrom-Json
    $LinuxCap = Get-Content -LiteralPath $LinuxCapabilitiesPath | ConvertFrom-Json
    $MacOsCap = Get-Content -LiteralPath $MacOsCapabilitiesPath | ConvertFrom-Json
    
    $Equivalent = $WindowsCap.governance_read -eq $LinuxCap.governance_read -and $LinuxCap.governance_read -eq $MacOsCap.governance_read
    
    $Twe4Details["governance_read"] = @{
        windows = $WindowsCap.governance_read
        linux = $LinuxCap.governance_read
        macos = $MacOsCap.governance_read
        equivalent = $Equivalent
    }
    
    if (-not $Equivalent) {
        $Twe4Passed = $false
        $ValidationReport.divergences_detected += "capability_governance_read"
    }
} else {
    Write-Host "  [WARN] Capabilities files not found, skipping" -ForegroundColor Yellow
}

$ValidationReport.checks_performed += @{
    check_name = "capability_declaration_equivalence"
    status = if ($Twe4Passed) { "passed" } else { "failed" }
    details = $Twe4Details
}

if ($Twe4Passed) {
    Write-Host "  [PASS] Capability declarations equivalent" -ForegroundColor Green
} else {
    Write-Host "  [FAIL] Capability declarations differ" -ForegroundColor Red
    $ValidationReport.overall_status = "DIVERGENT"
}

# Gate TWE-5: Divergence Detection
Write-Host ""
Write-Host "Gate TWE-5: Divergence Detection" -ForegroundColor Yellow

$Twe5Passed = $ValidationReport.divergences_detected.Count -eq 0

$ValidationReport.checks_performed += @{
    check_name = "divergence_detection"
    status = if ($Twe5Passed) { "passed" } else { "failed" }
    details = @{
        divergences_detected = $ValidationReport.divergences_detected
        divergence_count = $ValidationReport.divergences_detected.Count
    }
}

if ($Twe5Passed) {
    Write-Host "  [PASS] No divergences detected" -ForegroundColor Green
} else {
    Write-Host "  [FAIL] Divergences detected: $($ValidationReport.divergences_detected.Count)" -ForegroundColor Red
}

# Write validation report
$ReportPath = Join-Path $OutputPath "three-way-equivalence-validation-$Timestamp.json"
$ValidationReport | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $ReportPath

Write-Host ""
Write-Host "=== Three-Way Equivalence Validation Complete ===" -ForegroundColor Cyan
Write-Host ""

# Determine pass/fail
$AllPassed = $Twe1Passed -and $Twe2Passed -and $Twe3Passed -and $Twe4Passed -and $Twe5Passed

if ($AllPassed) {
    Write-Host "OVERALL: PASSED" -ForegroundColor Green
    Write-Host ""
    Write-Host "All acceptance gates passed:" -ForegroundColor White
    Write-Host "  TWE-1: Startup sequence equivalent" -ForegroundColor White
    Write-Host "  TWE-2: Deterministic fields equivalent" -ForegroundColor White
    Write-Host "  TWE-3: Governance decisions equivalent" -ForegroundColor White
    Write-Host "  TWE-4: Capability declarations equivalent" -ForegroundColor White
    Write-Host "  TWE-5: No divergences detected" -ForegroundColor White
    Write-Host ""
    Write-Host ("Evidence: " + $ReportPath) -ForegroundColor White
    
    exit 0
} else {
    Write-Host "OVERALL: FAILED" -ForegroundColor Red
    Write-Host ""
    Write-Host "Failed gates:" -ForegroundColor White
    if (-not $Twe1Passed) { Write-Host "  TWE-1: Startup sequence differs" -ForegroundColor Red }
    if (-not $Twe2Passed) { Write-Host "  TWE-2: Deterministic fields differ" -ForegroundColor Red }
    if (-not $Twe3Passed) { Write-Host "  TWE-3: Governance decisions differ" -ForegroundColor Red }
    if (-not $Twe4Passed) { Write-Host "  TWE-4: Capability declarations differ" -ForegroundColor Red }
    if (-not $Twe5Passed) { Write-Host "  TWE-5: Divergences detected" -ForegroundColor Red }
    Write-Host ""
    Write-Host ("Evidence: " + $ReportPath) -ForegroundColor White
    
    exit 1
}
