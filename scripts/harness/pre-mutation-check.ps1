<#
.SYNOPSIS
  Pre-mutation custody gate for the Windows Agent Harness.

.DESCRIPTION
  Verifies environment state before any agent or human mutation of the
  Librarian Runtime Node workspace. This is a read-only safety hook --
  it reports state and gates on preconditions but does NOT start, stop,
  or repair anything.

  Exit code 0 = ALL CHECKS PASSED -- safe to proceed
  Exit code 1 = ONE OR MORE CHECKS FAILED -- do not mutate

  Designed to be called at the start of every Windows PC sprint/session.
  Output is deterministic: same machine state produces same results.

.PARAMETER RepoRoot
  Path to the librarian-runtime-node repo root.
  Defaults to the directory containing this script's parent's parent.

.PARAMETER ExpectedHead
  Optional short SHA expected at HEAD. If provided and mismatched,
  the HEAD check fails.

.PARAMETER MinCdriveFreeGB
  Minimum acceptable free space on C: in gigabytes. Default 5.0.

.PARAMETER RequiredFiles
  Optional list of repo-relative paths that must exist.
  If omitted, a standard set of planning/baseline/receipt files is used.

.PARAMETER Quiet
  If set, suppress informational output; only emit pass/fail lines.

.EXAMPLE
  .\scripts\harness\pre-mutation-check.ps1

.EXAMPLE
  .\scripts\harness\pre-mutation-check.ps1 -ExpectedHead "7cc7d10" -MinCdriveFreeGB 5.0

.LINK
  docs/planning/WIN-AGENT-HARNESS-PLAN.md
  docs/planning/WIN-CUSTODY-SANDBOX-MODEL.md
#>

param(
  [string]$RepoRoot = "",
  [string]$ExpectedHead = "",
  [double]$MinCdriveFreeGB = 5.0,
  [string[]]$RequiredFiles = @(),
  [switch]$Quiet
)

# ============================================================================
# Helper functions
# ============================================================================

function Write-Message {
  param([string]$Text, [string]$Color = "White")
  if (-not $Quiet) {
    Write-Host $Text -ForegroundColor $Color
  }
}

function Write-Result {
  param([string]$Name, [bool]$Passed, [string]$Detail = "")
  $mark  = if ($Passed) { "PASS" } else { "FAIL" }
  $color = if ($Passed) { "Green" } else { "Red" }
  $extra = if ($Detail) { " ($Detail)" } else { "" }
  Write-Host "  $mark $Name$extra" -ForegroundColor $color
}

# ============================================================================
# Global check accumulator
# ============================================================================

$Script:TotalChecks = 0
$Script:PassedChecks = 0
$Script:FailedChecks = 0

function Add-Check {
  param(
    [string]$Name,
    [scriptblock]$Block
  )
  $Script:TotalChecks++
  try {
    $result = & $Block
    if ($result -is [bool] -and $result) {
      $Script:PassedChecks++
      Write-Result -Name $Name -Passed $true
    } elseif ($result -is [string] -and $result -ne "") {
      $Script:FailedChecks++
      Write-Result -Name $Name -Passed $false -Detail $result
    } else {
      $Script:FailedChecks++
      Write-Result -Name $Name -Passed $false -Detail "Check returned non-passing value"
    }
  } catch {
    $Script:FailedChecks++
    Write-Result -Name $Name -Passed $false -Detail $_.Exception.Message
  }
}

# ============================================================================
# Resolve repo root
# ============================================================================

if (-not $RepoRoot) {
  $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
  $RepoRoot = Split-Path -Parent (Split-Path -Parent $scriptDir)
}

$RepoRoot = (Resolve-Path $RepoRoot).Path
Write-Message -Text "---" -Color "DarkGray"
Write-Message -Text "Pre-Mutation Check -- Librarian Runtime Node" -Color "Cyan"
Write-Message -Text "Repo root: $RepoRoot" -Color "Gray"
Write-Message -Text "---" -Color "DarkGray"

# ============================================================================
# Check 1: Repo root exists
# ============================================================================

Add-Check -Name "Repo root accessible" -Block {
  Test-Path -LiteralPath $RepoRoot -PathType Container
}

# ============================================================================
# Check 2: Git HEAD verification
# ============================================================================

Add-Check -Name "Git HEAD" -Block {
  $head = & git -C $RepoRoot rev-parse --short HEAD 2>&1
  if (-not $head -or $LASTEXITCODE -ne 0) {
    return "Failed to read HEAD: $($head -join ' ')"
  }
  $head = $head.Trim()
  if ($ExpectedHead -and $head -ne $ExpectedHead) {
    return "HEAD is $head, expected $ExpectedHead"
  }
  $true
}

# ============================================================================
# Check 3: Working tree clean / dirty
# ============================================================================

Add-Check -Name "Working tree clean" -Block {
  $status = & git -C $RepoRoot status --short 2>&1
  if ($LASTEXITCODE -ne 0) {
    return "git status failed: $($status -join ' ')"
  }
  $dirty = @($status | Where-Object { $_ -match '^[MADRCU?! ]' })
  if ($dirty.Count -gt 0) {
    return "$($dirty.Count) dirty file(s) -- $($dirty -join '; ')"
  }
  $true
}

# ============================================================================
# Check 4: Git branch
# ============================================================================

Add-Check -Name "Git branch is main" -Block {
  $branch = & git -C $RepoRoot rev-parse --abbrev-ref HEAD 2>&1
  if ($LASTEXITCODE -ne 0) {
    return "Failed to read branch: $($branch -join ' ')"
  }
  $branch = $branch.Trim()
  if ($branch -ne "main") {
    return "On branch '$branch', expected 'main'"
  }
  $true
}

# ============================================================================
# Check 5: LibrarianRunTimeNode service state
# ============================================================================

Add-Check -Name "Service LibrarianRunTimeNode" -Block {
  $svc = Get-Service -Name "LibrarianRunTimeNode" -ErrorAction SilentlyContinue
  if (-not $svc) {
    return "Service not found"
  }
  $expectedStatus = "Stopped"
  $expectedStartType = "Manual"
  $issues = @()
  if ($svc.Status -ne $expectedStatus) {
    $issues += "Status is $($svc.Status), expected $expectedStatus"
  }
  if ($svc.StartType -ne $expectedStartType) {
    $issues += "StartType is $($svc.StartType), expected $expectedStartType"
  }
  if ($issues.Count -gt 0) {
    return $issues -join "; "
  }
  $true
}

# ============================================================================
# Check 6: Port occupancy -- backend ports 9120-9125
# ============================================================================

Add-Check -Name "Ports 9120-9125 free" -Block {
  $occupiedPorts = @()
  foreach ($port in 9120..9125) {
    $listening = netstat -ano 2>$null | Select-String ":$port\s" | Select-String "LISTENING"
    if ($listening) {
      $occupiedPorts += $port
    }
  }
  if ($occupiedPorts.Count -gt 0) {
    return "Port(s) $($occupiedPorts -join ', ') in LISTENING state"
  }
  $true
}

# ============================================================================
# Check 7: Port occupancy -- router port 9130
# ============================================================================

Add-Check -Name "Port 9130 free" -Block {
  $listening = netstat -ano 2>$null | Select-String ":9130\s" | Select-String "LISTENING"
  if ($listening) {
    return "Port 9130 in LISTENING state"
  }
  $true
}

# ============================================================================
# Check 8: Orphan runtime/router/model processes
# ============================================================================

Add-Check -Name "No orphan runtime/router/model processes" -Block {
  $orphanNames = @("llama-server.exe", "rust-router.exe")
  $orphanPids = @()

  $procs = Get-CimInstance Win32_Process -ErrorAction SilentlyContinue
  if (-not $procs) {
    return "Cannot enumerate processes"
  }

  foreach ($proc in $procs) {
    if ($proc.Name -in $orphanNames) {
      $orphanPids += "$($proc.Name) PID $($proc.ProcessId)"
    }
    if ($proc.Name -eq "python.exe" -and $proc.CommandLine -match "router[\\/]router\.py") {
      $orphanPids += "python.exe (router) PID $($proc.ProcessId)"
    }
  }

  if ($orphanPids.Count -gt 0) {
    return "Orphan(s) found: $($orphanPids -join '; ')"
  }
  $true
}

# ============================================================================
# Check 9: C: drive free space
# ============================================================================

Add-Check -Name "C: drive free space >= $MinCdriveFreeGB GB" -Block {
  $cDrive = Get-CimInstance Win32_LogicalDisk -Filter "DeviceID='C:'" -ErrorAction SilentlyContinue
  if (-not $cDrive) {
    return "Cannot enumerate C: drive"
  }
  $freeGB = [math]::Round($cDrive.FreeSpace / 1GB, 1)
  if ($freeGB -lt $MinCdriveFreeGB) {
    return "$freeGB GB free -- below threshold of $MinCdriveFreeGB GB"
  }
  $true
}

# ============================================================================
# Check 10: Git origin sync
# ============================================================================

Add-Check -Name "Origin/main in sync" -Block {
  $local = & git -C $RepoRoot rev-parse HEAD 2>&1
  if ($LASTEXITCODE -ne 0) { return "Cannot read local HEAD" }
  $remote = & git -C $RepoRoot rev-parse origin/main 2>&1
  if ($LASTEXITCODE -ne 0) { return "No origin/main ref (network fetch may be needed)" }
  $local = $local.Trim()
  $remote = $remote.Trim()
  if ($local -ne $remote) {
    $ahead = @(& git -C $RepoRoot rev-list --count "$remote..$local" 2>&1)
    $behind = @(& git -C $RepoRoot rev-list --count "$local..$remote" 2>&1)
    return "Local ($($local.Substring(0,7))) differs from origin/main ($($remote.Substring(0,7))) -- ahead $ahead, behind $behind"
  }
  $true
}

# ============================================================================
# Check 11: Required planning/baseline/receipt files exist
# ============================================================================

if ($RequiredFiles.Count -eq 0) {
  $RequiredFiles = @(
    "SESSION-HANDOFF.md",
    "docs/planning/WIN-AGENT-HARNESS-PLAN.md",
    "docs/planning/WIN-CUSTODY-SANDBOX-MODEL.md",
    "docs/planning/WIN-HARNESS-PARITY-ROADMAP.md",
    "docs/planning/WIN-LIBRARIAN-HOST-OPTIONS.md",
    "docs/planning/WIN-SPRINT-SEQUENCE.md",
    "docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md",
    "docs/receipts/WIN-AGENT-HARNESS-PLAN-1-RECEIPT.md",
    "docs/receipts/WIN-AGENT-HARNESS-ENV-BASELINE-1-RECEIPT.md"
  )
}

Add-Check -Name "Required planning/baseline/receipt files" -Block {
  $missing = @()
  foreach ($relPath in $RequiredFiles) {
    $fullPath = Join-Path -Path $RepoRoot -ChildPath $relPath
    if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
      $missing += $relPath
    }
  }
  if ($missing.Count -gt 0) {
    return "Missing: $($missing -join ', ')"
  }
  $true
}

# ============================================================================
# Summary
# ============================================================================

Write-Message -Text "---" -Color "DarkGray"
$overallPass = ($Script:FailedChecks -eq 0)
$summaryText = "Checked $($Script:TotalChecks) | Passed $($Script:PassedChecks) | Failed $($Script:FailedChecks)"
if ($overallPass) {
  Write-Host "  OVERALL: PASS ($summaryText)" -ForegroundColor "Green"
} else {
  Write-Host "  OVERALL: FAIL ($summaryText)" -ForegroundColor "Red"
}
Write-Message -Text "---" -Color "DarkGray"

# ============================================================================
# Exit code
# ============================================================================

if ($overallPass) {
  exit 0
} else {
  exit 1
}
