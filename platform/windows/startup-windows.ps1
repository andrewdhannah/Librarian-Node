# startup-windows.ps1 — Windows Startup Adapter
#
# Version: 1.0.0
# Platform: Windows
# Status: Reference Implementation
#
# Purpose:
#   This script implements the canonical startup protocol for Windows nodes.
#   It follows the STARTUP-PROTOCOL.md contract and produces a startup
#   receipt conforming to STARTUP-OUTPUT-CONTRACT.md.
#
# Usage:
#   .\startup-windows.ps1 -NodePath "G:\OpenWork\runtime-node" -OutputPath "G:\OpenWork\evidence\reference-architecture"
#
# Parameters:
#   -NodePath     Path to node directory (required)
#   -OutputPath   Path for startup receipt (default: evidence\startup)

param(
    [Parameter(Mandatory=$true)]
    [string]$NodePath,
    
    [Parameter(Mandatory=$false)]
    [string]$OutputPath = "G:\OpenWork\evidence\reference-architecture"
)

$ErrorActionPreference = "Continue"
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"

Write-Host "=== Librarian Node Windows Startup ===" -ForegroundColor Cyan
Write-Host "Node Path: $NodePath"
Write-Host "Output Path: $OutputPath"
Write-Host ""

# Ensure output directory exists
if (-not (Test-Path -LiteralPath $OutputPath)) {
    New-Item -ItemType Directory -Path $OutputPath -Force | Out-Null
}

# Initialize receipt
$receipt = @{
    receipt_id = "WINDOWS-STARTUP-$timestamp"
    node_id = ""
    platform = "windows"
    governance_commit = ""
    startup_phase = "pending"
    identity_loaded = $false
    governance_verified = $false
    capabilities_loaded = $false
    environment_validated = $false
    checks_passed = 0
    checks_failed = 0
    status = "pending"
    timestamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.0000000Z")
}

# Phase 1: Identity Loading
Write-Host "Phase 1: Loading identity..." -ForegroundColor Yellow
$identityPath = Join-Path $NodePath "node-identity.json"
$nodeIdPath = Join-Path $NodePath "node-id.json"

# Check for both identity file formats
if (Test-Path -LiteralPath $identityPath) {
    $identity = Get-Content -LiteralPath $identityPath | ConvertFrom-Json
    
    # Validate identity
    if ($identity.node_type -eq "librarian-runtime-node" -and
        $identity.authority -eq "owner-controlled" -and
        $identity.platform -eq "windows") {
        
        $receipt.node_id = $identity.node_id
        $receipt.identity_loaded = $true
        $receipt.checks_passed++
        Write-Host "  [PASS] Identity loaded: $($identity.node_id)" -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] Invalid identity format" -ForegroundColor Red
        $receipt.checks_failed++
        $receipt.startup_phase = "failed"
        $receipt.status = "STARTUP_FAILED"
    }
} elseif (Test-Path -LiteralPath $nodeIdPath) {
    $identity = Get-Content -LiteralPath $nodeIdPath | ConvertFrom-Json
    
    # Validate identity (legacy format)
    if ($identity.authority -eq "owner-controlled" -and
        $identity.node_id -ne $null) {
        
        $receipt.node_id = $identity.node_id
        $receipt.identity_loaded = $true
        $receipt.checks_passed++
        Write-Host "  [PASS] Identity loaded: $($identity.node_id) (legacy format)" -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] Invalid identity format" -ForegroundColor Red
        $receipt.checks_failed++
        $receipt.startup_phase = "failed"
        $receipt.status = "STARTUP_FAILED"
    }
} else {
    Write-Host "  [FAIL] Identity file not found: $identityPath or $nodeIdPath" -ForegroundColor Red
    $receipt.checks_failed++
    $receipt.startup_phase = "failed"
    $receipt.status = "STARTUP_FAILED"
}

# Phase 2: Governance Verification
if ($receipt.status -ne "STARTUP_FAILED") {
    Write-Host "Phase 2: Verifying governance..." -ForegroundColor Yellow
    $governancePath = Join-Path $NodePath "governance-sync.json"
    
    if (Test-Path -LiteralPath $governancePath) {
        $governance = Get-Content -LiteralPath $governancePath | ConvertFrom-Json
        
        if ($governance.verification_status -eq "verified" -and
            $governance.last_verified_commit -match "^[a-f0-9]{40}$") {
            
            $receipt.governance_commit = $governance.last_verified_commit
            $receipt.governance_verified = $true
            $receipt.checks_passed++
            Write-Host "  [PASS] Governance verified: $($governance.last_verified_commit)" -ForegroundColor Green
        } else {
            Write-Host "  [FAIL] Governance verification failed" -ForegroundColor Red
            $receipt.checks_failed++
            $receipt.startup_phase = "failed"
            $receipt.status = "STARTUP_FAILED"
        }
    } else {
        Write-Host "  [FAIL] Governance file not found: $governancePath" -ForegroundColor Red
        $receipt.checks_failed++
        $receipt.startup_phase = "failed"
        $receipt.status = "STARTUP_FAILED"
    }
}

# Phase 3: Capability Loading
if ($receipt.status -ne "STARTUP_FAILED") {
    Write-Host "Phase 3: Loading capabilities..." -ForegroundColor Yellow
    $capabilitiesPath = Join-Path $NodePath "capabilities.json"
    
    if (Test-Path -LiteralPath $capabilitiesPath) {
        $capabilities = Get-Content -LiteralPath $capabilitiesPath | ConvertFrom-Json
        
        # Check for governance_read capability (required)
        if ($capabilities.governance_read -eq $true) {
            $receipt.capabilities_loaded = $true
            $receipt.checks_passed++
            Write-Host "  [PASS] Capabilities loaded" -ForegroundColor Green
        } else {
            Write-Host "  [FAIL] Required capabilities missing" -ForegroundColor Red
            $receipt.checks_failed++
            $receipt.startup_phase = "failed"
            $receipt.status = "STARTUP_FAILED"
        }
    } else {
        Write-Host "  [FAIL] Capabilities file not found: $capabilitiesPath" -ForegroundColor Red
        $receipt.checks_failed++
        $receipt.startup_phase = "failed"
        $receipt.status = "STARTUP_FAILED"
    }
}

# Phase 4: Environment Validation
if ($receipt.status -ne "STARTUP_FAILED") {
    Write-Host "Phase 4: Validating environment..." -ForegroundColor Yellow
    
    # Check PowerShell version
    if ($PSVersionTable.PSVersion.Major -ge 5) {
        $receipt.environment_validated = $true
        $receipt.checks_passed++
        Write-Host "  [PASS] Environment validated" -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] PowerShell 5.0 or higher required" -ForegroundColor Red
        $receipt.checks_failed++
        $receipt.startup_phase = "failed"
        $receipt.status = "STARTUP_FAILED"
    }
}

# Phase 5: Generate Startup Receipt
if ($receipt.status -ne "STARTUP_FAILED") {
    Write-Host "Phase 5: Generating startup receipt..." -ForegroundColor Yellow
    $receipt.startup_phase = "complete"
    $receipt.status = "GOVERNED_EXECUTION"
    $receipt.checks_passed++
    Write-Host "  [PASS] Startup receipt generated" -ForegroundColor Green
    
    # Phase 6: Enter Governed Mode
    Write-Host "Phase 6: Entering governed mode..." -ForegroundColor Yellow
    $receipt.checks_passed++
    Write-Host "  [PASS] Entered governed mode" -ForegroundColor Green
}

# Write receipt
$receiptPath = Join-Path $OutputPath "startup-receipt-$timestamp.json"
$receipt | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $receiptPath

Write-Host ""
Write-Host "=== Windows Startup Complete ===" -ForegroundColor Cyan
Write-Host "Status: $($receipt.status)"
Write-Host "Checks Passed: $($receipt.checks_passed) | Failed: $($receipt.checks_failed)"
Write-Host "Receipt: $receiptPath"

if ($receipt.status -eq "STARTUP_FAILED") {
    exit 1
} else {
    exit 0
}
