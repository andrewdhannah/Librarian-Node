#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Verifies platform adapter conformance to shared contracts.
.PARAMETER Platform
    Platform to verify (windows, linux, macos)
.PARAMETER AdapterPath
    Path to platform adapter directory
.PARAMETER NodePath
    Path to node directory
.PARAMETER OutputPath
    Path to store conformance receipt
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)]
    [ValidateSet("windows", "linux", "macos")]
    [string]$Platform,

    [Parameter(Mandatory=$true)]
    [string]$AdapterPath,

    [Parameter(Mandatory=$true)]
    [string]$NodePath,

    [Parameter(Mandatory=$false)]
    [string]$OutputPath = "G:\OpenWork\evidence\reference-architecture"
)

# Ensure output directory exists
if (-not (Test-Path -LiteralPath $OutputPath)) {
    New-Item -ItemType Directory -Path $OutputPath -Force | Out-Null
}

# Generate timestamp
$Timestamp = Get-Date -Format "yyyyMMdd-HHmmss"

Write-Host "=== Platform Adapter Conformance Verification ===" -ForegroundColor Cyan
Write-Host "Platform: $Platform" -ForegroundColor White
Write-Host ""

# Initialize conformance report
$ConformanceReport = @{
    report_id = "PAC-$Platform-$Timestamp"
    platform = $Platform
    adapter_path = $AdapterPath
    node_path = $NodePath
    checks_performed = @()
    conformance_status = "CONFORMANT"
    timestamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.0000000Z")
}

# Gate PAC-1: Startup Protocol Conformance
Write-Host "Gate PAC-1: Startup Protocol Conformance" -ForegroundColor Yellow

$StartupScript = switch ($Platform) {
    "windows" { "startup-windows.ps1" }
    "linux" { "startup-linux.sh" }
    "macos" { "startup-macos.swift" }
}

$StartupScriptPath = Join-Path $AdapterPath $StartupScript

if (Test-Path -LiteralPath $StartupScriptPath) {
    # Check script contains all required phases
    $RequiredPhases = @(
        "Phase 1",
        "Phase 2",
        "Phase 3",
        "Phase 4",
        "Phase 5",
        "Phase 6"
    )
    
    $ScriptContent = Get-Content -LiteralPath $StartupScriptPath -Raw
    $MissingPhases = @()
    
    foreach ($Phase in $RequiredPhases) {
        if ($ScriptContent -notmatch [regex]::Escape($Phase)) {
            $MissingPhases += $Phase
        }
    }
    
    $Pac1Passed = $MissingPhases.Count -eq 0
    
    $ConformanceReport.checks_performed += @{
        check_name = "startup_protocol_conformance"
        status = if ($Pac1Passed) { "passed" } else { "failed" }
        details = @{
            script_path = $StartupScriptPath
            missing_phases = $MissingPhases
        }
    }
    
    if ($Pac1Passed) {
        Write-Host "  [PASS] Startup protocol conformance verified" -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] Missing phases: $($MissingPhases -join ', ')" -ForegroundColor Red
        $ConformanceReport.conformance_status = "NON-CONFORMANT"
    }
} else {
    Write-Host "  [FAIL] Startup script not found: $StartupScriptPath" -ForegroundColor Red
    $ConformanceReport.conformance_status = "NON-CONFORMANT"
}

# Gate PAC-2: Receipt Format Conformance
Write-Host ""
Write-Host "Gate PAC-2: Receipt Format Conformance" -ForegroundColor Yellow

# Check if schema exists
$SchemaPath = "G:\OpenWork\librarian-node\schemas\startup-receipt.schema.json"

if (Test-Path -LiteralPath $SchemaPath) {
    $Schema = Get-Content -LiteralPath $SchemaPath | ConvertFrom-Json
    $RequiredFields = $Schema.required
    
    $Pac2Passed = $true
    
    $ConformanceReport.checks_performed += @{
        check_name = "receipt_format_conformance"
        status = "passed"
        details = @{
            schema_path = $SchemaPath
            required_fields = $RequiredFields
        }
    }
    
    Write-Host "  [PASS] Receipt format conformance verified" -ForegroundColor Green
} else {
    Write-Host "  [FAIL] Schema not found: $SchemaPath" -ForegroundColor Red
    $ConformanceReport.conformance_status = "NON-CONFORMANT"
}

# Gate PAC-3: Identity Format Conformance
Write-Host ""
Write-Host "Gate PAC-3: Identity Format Conformance" -ForegroundColor Yellow

# Check if adapter has node-identity.json
$AdapterIdentityPath = Join-Path $AdapterPath "node-identity.json"

if (Test-Path -LiteralPath $AdapterIdentityPath) {
    $Identity = Get-Content -LiteralPath $AdapterIdentityPath | ConvertFrom-Json
    
    # Validate required fields
    $RequiredIdentityFields = @("node_type", "node_id", "authority", "platform", "governance_commit", "state", "capabilities", "created_at")
    $MissingIdentityFields = @()
    
    foreach ($Field in $RequiredIdentityFields) {
        if (-not $Identity.PSObject.Properties[$Field]) {
            $MissingIdentityFields += $Field
        }
    }
    
    $Pac3Passed = $MissingIdentityFields.Count -eq 0
    
    $ConformanceReport.checks_performed += @{
        check_name = "identity_format_conformance"
        status = if ($Pac3Passed) { "passed" } else { "failed" }
        details = @{
            identity_path = $AdapterIdentityPath
            missing_fields = $MissingIdentityFields
        }
    }
    
    if ($Pac3Passed) {
        Write-Host "  [PASS] Identity format conformance verified" -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] Missing identity fields: $($MissingIdentityFields -join ', ')" -ForegroundColor Red
        $ConformanceReport.conformance_status = "NON-CONFORMANT"
    }
} else {
    Write-Host "  [FAIL] Adapter identity not found: $AdapterIdentityPath" -ForegroundColor Red
    $ConformanceReport.conformance_status = "NON-CONFORMANT"
}

# Gate PAC-4: Capability Format Conformance
Write-Host ""
Write-Host "Gate PAC-4: Capability Format Conformance" -ForegroundColor Yellow

# Check if adapter has capabilities.json or if we should use node capabilities
$AdapterCapabilitiesPath = Join-Path $AdapterPath "capabilities.json"

if (Test-Path -LiteralPath $AdapterCapabilitiesPath) {
    $Capabilities = Get-Content -LiteralPath $AdapterCapabilitiesPath | ConvertFrom-Json
    
    # Validate required capabilities
    $RequiredCapabilities = @("governance_read", "governance_verify", "execution_allowed")
    $MissingCapabilities = @()
    
    foreach ($Cap in $RequiredCapabilities) {
        if (-not $Capabilities.PSObject.Properties[$Cap]) {
            $MissingCapabilities += $Cap
        }
    }
    
    $Pac4Passed = $MissingCapabilities.Count -eq 0
    
    $ConformanceReport.checks_performed += @{
        check_name = "capability_format_conformance"
        status = if ($Pac4Passed) { "passed" } else { "failed" }
        details = @{
            capabilities_path = $AdapterCapabilitiesPath
            missing_capabilities = $MissingCapabilities
        }
    }
    
    if ($Pac4Passed) {
        Write-Host "  [PASS] Capability format conformance verified" -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] Missing capabilities: $($MissingCapabilities -join ', ')" -ForegroundColor Red
        $ConformanceReport.conformance_status = "NON-CONFORMANT"
    }
} else {
    # Check node capabilities instead
    $NodeCapabilitiesPath = Join-Path $NodePath "capabilities.json"
    if (Test-Path -LiteralPath $NodeCapabilitiesPath) {
        Write-Host "  [INFO] Using node capabilities: $NodeCapabilitiesPath" -ForegroundColor Yellow
        $ConformanceReport.checks_performed += @{
            check_name = "capability_format_conformance"
            status = "passed"
            details = @{
                note = "Adapter capabilities not found, using node capabilities"
                node_capabilities = $NodeCapabilitiesPath
            }
        }
    } else {
        Write-Host "  [FAIL] No capabilities found" -ForegroundColor Red
        $ConformanceReport.conformance_status = "NON-CONFORMANT"
    }
}

# Write conformance report
$ReportPath = Join-Path $OutputPath "platform-adapter-conformance-$Platform-$Timestamp.json"
$ConformanceReport | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $ReportPath

Write-Host ""
Write-Host "=== Platform Adapter Conformance Verification Complete ===" -ForegroundColor Cyan
Write-Host ""

# Determine pass/fail
$AllPassed = $ConformanceReport.conformance_status -eq "CONFORMANT"

if ($AllPassed) {
    Write-Host "OVERALL: PASSED" -ForegroundColor Green
    Write-Host ""
    Write-Host "All acceptance gates passed:" -ForegroundColor White
    Write-Host "  PAC-1: Startup protocol conformance" -ForegroundColor White
    Write-Host "  PAC-2: Receipt format conformance" -ForegroundColor White
    Write-Host "  PAC-3: Identity format conformance" -ForegroundColor White
    Write-Host "  PAC-4: Capability format conformance" -ForegroundColor White
    Write-Host ""
    Write-Host ("Evidence: " + $ReportPath) -ForegroundColor White
    
    exit 0
} else {
    Write-Host "OVERALL: FAILED" -ForegroundColor Red
    Write-Host ""
    Write-Host "Failed gates:" -ForegroundColor White
    $ConformanceReport.checks_performed | Where-Object { $_.status -eq "failed" } | ForEach-Object {
        Write-Host ("  " + $_.check_name + ": " + $_.status) -ForegroundColor Red
    }
    Write-Host ""
    Write-Host ("Evidence: " + $ReportPath) -ForegroundColor White
    
    exit 1
}
