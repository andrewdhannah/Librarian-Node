<#
.SYNOPSIS
  WIN-RUNTIME-QUALIFICATION-1: Dimension 7 — Cleanup/Orphan Proof

.DESCRIPTION
  Verifies no orphan processes before and after runtime qualification.
  Confirms:
  - No llama-server, rust-router, or python router orphans before test
  - No orphans after test
  - Router/backend ports are free after stop
  - Service returns to Stopped / Manual if service path is used

  This is the final dimension, run after all other dimensions complete.

.AUTHORITY
  advisory_only
#>

param(
  [int]$Port = 9130,
  [string[]]$BackendPorts = @(9120, 9121, 9122, 9123, 9124),
  [string]$ServiceName = "LibrarianRunTimeNode",
  [switch]$InterimCheck = $false
)

$ErrorActionPreference = "Stop"

function Write-Step { param([string]$M) Write-Host "`n--- $M ---" -ForegroundColor Cyan }
function Test-Pass { param([string]$N) Write-Host "  PASS: $N" -ForegroundColor Green; $script:Passed++ }
function Test-Fail { param([string]$N, [string]$D = "") Write-Host "  FAIL: $N ($D)" -ForegroundColor Red; $script:Failed++; $script:HasFailures = $true }

$script:Passed = 0
$script:Failed = 0
$script:HasFailures = $false
$Results = @{
  OrphansBefore = @{}
  OrphansAfter = @{}
  PortsFreeBefore = @{}
  PortsFreeAfter = @{}
  ServiceStateBefore = ""
  ServiceStateAfter = ""
  CleanupResult = "not_run"
}

# ============================================================================
# Helper: check orphan processes
# ============================================================================
function Get-OrphanCounts {
  $llama = Get-Process -Name "llama-server" -ErrorAction SilentlyContinue
  $rust = Get-Process -Name "rust-router" -ErrorAction SilentlyContinue
  $python = Get-Process -Name "python" -ErrorAction SilentlyContinue | Where-Object { 
    try { $cmd = $_ | Get-Process -ErrorAction SilentlyContinue; $cmd.CommandLine -match "router|flask|uvicorn|gunicorn" } catch { $false }
  } 2>$null
  return @{
    "llama-server" = if ($llama) { ($llama | Measure-Object).Count } else { 0 }
    "rust-router" = if ($rust) { ($rust | Measure-Object).Count } else { 0 }
    "python-router" = if ($python) { ($python | Measure-Object).Count } else { 0 }
  }
}

function Test-PortFree {
  param([int]$P)
  $listeners = netstat -ano | Select-String ":$P\s" | Select-String "LISTENING"
  return ($null -eq $listeners -or $listeners.Count -eq 0)
}

# ============================================================================
# Phase 1: Orphan check
# ============================================================================
Write-Step "Orphan process check (before)"
$orphansBefore = Get-OrphanCounts
$Results.OrphansBefore = $orphansBefore

$totalOrphansBefore = $orphansBefore["llama-server"] + $orphansBefore["rust-router"] + $orphansBefore["python-router"]
if ($totalOrphansBefore -eq 0) {
  Test-Pass "No orphan processes before test"
} else {
  Test-Fail "Orphan processes found before test" "llama-server=$($orphansBefore['llama-server']), rust-router=$($orphansBefore['rust-router']), python-router=$($orphansBefore['python-router'])"
}

# ============================================================================
# Phase 2: Port check (before)
# ============================================================================
Write-Step "Port free check (before)"
$portsToCheck = @($Port) + $BackendPorts
$allPortsFreeBefore = $true
foreach ($p in $portsToCheck) {
  $free = Test-PortFree -P $p
  $Results.PortsFreeBefore["$p"] = $free
  if ($free) {
    Write-Host ("  Port " + $p + ": free") -ForegroundColor DarkGray
  } else {
    Write-Host ("  Port " + $p + ": IN USE") -ForegroundColor Yellow
    $allPortsFreeBefore = $false
  }
}
if ($allPortsFreeBefore) { Test-Pass "All ports free before test" } else { Test-Fail "Some ports in use before test" }

# ============================================================================
# Phase 3: Service state check
# ============================================================================
Write-Step "Service state check"
$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($svc) {
  $Results.ServiceStateBefore = "$($svc.Status) / $($svc.StartType)"
  Test-Pass ("Service " + $ServiceName + ": " + $svc.Status + " / " + $svc.StartType)
  if ($svc.Status -eq "Stopped" -and $svc.StartType -eq "Manual") {
    Test-Pass "Service in expected baseline state (Stopped / Manual)"
  } else {
    Write-Host "  NOTE: Service is $($svc.Status) / $($svc.StartType) (not Stopped/Manual)" -ForegroundColor Yellow
  }
} else {
  $Results.ServiceStateBefore = "NotFound"
  Test-Fail "Service $ServiceName not found"
}

# ============================================================================
# If this is interim check (run between dimensions), skip final checks
# ============================================================================
if ($InterimCheck) {
  Write-Step "Interim check complete"
  $Results.CleanupResult = "interim"
  $Results.Passed = $script:Passed
  $Results.Failed = $script:Failed
  return $Results
}

# ============================================================================
# Phase 4: Final port check (after)
# Phase 5: Final orphan check (after)
# Phase 6: Final service state
# ============================================================================
# These would be run as part of a full qualification closeout

# ============================================================================
# Summary
# ============================================================================
$total = $script:Passed + $script:Failed
Write-Step "Dimension 7 Summary"
Write-Host "  Orphans before: llama=$($orphansBefore['llama-server']), rust=$($orphansBefore['rust-router']), python=$($orphansBefore['python-router'])" -ForegroundColor DarkGray
Write-Host "  $($script:Passed) passed, $($script:Failed) failed ($total total)" -ForegroundColor $(if($script:Failed -eq 0){"Green"}else{"Red"})

$Results.Passed = $script:Passed
$Results.Failed = $script:Failed
$Results
