<#
.SYNOPSIS
  Validate the sprint-ledger.json file for structural integrity.

.DESCRIPTION
  Parses project-state/sprint-ledger.json and validates:
  - JSON parses correctly
  - Required top-level fields exist (schema_version, generated_at, current_head,
    origin_sync_state, active_sprint, next_authorized_sprint, sprints)
  - Each sprint entry has all required fields
  - receipt_path files exist (when not null)
  - sprint_doc_path files exist (when not null)
  - commit hashes are non-empty
  - next_authorized_sprint is present and non-empty
  - active_sprint is null or references a known sprint_id
  - Deterministic exit code: 0 = pass, 1 = fail

  Exit code 0 = ALL CHECKS PASSED -- ledger is valid
  Exit code 1 = ONE OR MORE CHECKS FAILED -- review ledger

.PARAMETER LedgerPath
  Path to sprint-ledger.json. Defaults to project-state/sprint-ledger.json
  relative to RepoRoot.

.PARAMETER RepoRoot
  Path to the librarian-runtime-node repo root.
  Auto-detected from the script location if omitted.

.PARAMETER Quiet
  Suppress informational output; only emit pass/fail lines.

.EXAMPLE
  .\scripts\harness\validate-sprint-ledger.ps1

.EXAMPLE
  .\scripts\harness\validate-sprint-ledger.ps1 -Quiet

.LINK
  project-state/sprint-ledger.json
  scripts/harness/pre-mutation-check.ps1
#>

param(
  [string]$LedgerPath = "",
  [string]$RepoRoot = "",
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
# Resolve paths
# ============================================================================

if (-not $RepoRoot) {
  $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
  $RepoRoot = Split-Path -Parent (Split-Path -Parent $scriptDir)
}
$RepoRoot = (Resolve-Path $RepoRoot).Path

if (-not $LedgerPath) {
  $LedgerPath = Join-Path -Path $RepoRoot -ChildPath "project-state\sprint-ledger.json"
} elseif (-not [System.IO.Path]::IsPathRooted($LedgerPath)) {
  $LedgerPath = Join-Path -Path $RepoRoot -ChildPath $LedgerPath
}
$LedgerPath = (Resolve-Path $LedgerPath -ErrorAction SilentlyContinue).Path

# ============================================================================
# Required fields for each sprint entry
# ============================================================================

$RequiredSprintFields = @(
  "sprint_id",
  "status",
  "commit",
  "pushed",
  "branch",
  "receipt_path",
  "sprint_doc_path",
  "primary_files",
  "category",
  "phase",
  "owner_review_required",
  "next_sprint",
  "notes"
)

$ValidStatuses = @("sealed", "ready_for_review", "active", "planned")

# ============================================================================
# Check 1: Ledger file exists
# ============================================================================

Add-Check -Name "Ledger file exists" -Block {
  if (-not $LedgerPath) {
    $resolved = Join-Path -Path $RepoRoot -ChildPath "project-state\sprint-ledger.json"
    if (Test-Path -LiteralPath $resolved -PathType Leaf) {
      $Script:LedgerPathActual = $resolved
      return $true
    }
    return "File not found at: $resolved"
  }
  if (-not (Test-Path -LiteralPath $LedgerPath -PathType Leaf)) {
    return "File not found at: $LedgerPath"
  }
  $Script:LedgerPathActual = $LedgerPath
  $true
}

# Stop early if ledger doesn't exist
if ($Script:FailedChecks -gt 0) {
  Write-Message -Text "---" -Color "DarkGray"
  Write-Host "  OVERALL: FAIL (Ledger file not found)" -ForegroundColor "Red"
  exit 1
}

# ============================================================================
# Check 2: JSON parses correctly
# ============================================================================

$Script:LedgerData = $null

Add-Check -Name "JSON parses correctly" -Block {
  try {
    $raw = Get-Content -Path $Script:LedgerPathActual -Raw -ErrorAction Stop
    $parsed = $raw | ConvertFrom-Json -ErrorAction Stop
    $Script:LedgerData = $parsed
    $true
  } catch {
    return "JSON parse error: $($_.Exception.Message)"
  }
}

# ============================================================================
# Check 3: Required top-level fields exist
# ============================================================================

$RequiredTopFields = @(
  "schema_version",
  "generated_at",
  "current_head",
  "origin_sync_state",
  "active_sprint",
  "next_authorized_sprint",
  "sprints"
)

Add-Check -Name "Required top-level fields exist" -Block {
  $missing = @()
  $existingProps = @($Script:LedgerData.PSObject.Properties.Name)
  foreach ($field in $RequiredTopFields) {
    if ($existingProps -notcontains $field) {
      $missing += $field
    }
  }
  if ($missing.Count -gt 0) {
    return "Missing top-level field(s): $($missing -join ', ')"
  }
  $true
}

# ============================================================================
# Check 4: current_head has required sub-fields
# ============================================================================

Add-Check -Name "current_head has valid structure" -Block {
  $ch = $Script:LedgerData.current_head
  if (-not $ch.full -or $ch.full -eq "") { return "current_head.full is missing or empty" }
  if (-not $ch.short -or $ch.short -eq "") { return "current_head.short is missing or empty" }
  if (-not $ch.branch -or $ch.branch -eq "") { return "current_head.branch is missing or empty" }
  $true
}

# ============================================================================
# Check 5: origin_sync_state is non-empty
# ============================================================================

Add-Check -Name "origin_sync_state is present" -Block {
  $val = $Script:LedgerData.origin_sync_state
  if (-not $val -or $val -eq "") { return "origin_sync_state is missing or empty" }
  $true
}

# ============================================================================
# Check 6: next_authorized_sprint is present and non-empty
# ============================================================================

Add-Check -Name "next_authorized_sprint is present" -Block {
  $val = $Script:LedgerData.next_authorized_sprint
  if (-not $val -or $val -eq "") { return "next_authorized_sprint is missing or empty" }
  $true
}

# ============================================================================
# Check 7: active_sprint is null or references a known sprint_id
# ============================================================================

Add-Check -Name "active_sprint is valid" -Block {
  $active = $Script:LedgerData.active_sprint
  if ($null -eq $active -or $active -eq "") {
    # null/empty is valid
    return $true
  }
  # If non-null, must be a known sprint_id
  $knownIds = @($Script:LedgerData.sprints | ForEach-Object { $_.sprint_id })
  if ($knownIds -contains $active) {
    return $true
  }
  return "active_sprint '$active' does not match any known sprint_id"
}

# ============================================================================
# Check 8: sprints array is non-empty
# ============================================================================

Add-Check -Name "sprints array is non-empty" -Block {
  $count = @($Script:LedgerData.sprints).Count
  if ($count -eq 0) { return "sprints array is empty" }
  $true
}

# ============================================================================
# Check 9: Each sprint entry has required fields
# ============================================================================

Add-Check -Name "Sprint entries have required fields" -Block {
  $entries = @($Script:LedgerData.sprints)
  $errors = @()
  $nullableFields = @("receipt_path", "sprint_doc_path")
  foreach ($entry in $entries) {
    $sid = if ($entry.sprint_id) { $entry.sprint_id } else { "(unnamed)" }
    $existingProps = @($entry.PSObject.Properties.Name)
    foreach ($field in $RequiredSprintFields) {
      if ($existingProps -notcontains $field) {
        $errors += "$sid missing field: $field"
      }
    }
  }
  if ($errors.Count -gt 0) {
    return ($errors -join "; ")
  }
  $true
}

# ============================================================================
# Check 10: Commit hashes are non-empty
# ============================================================================

Add-Check -Name "Sprint commit hashes are non-empty" -Block {
  $entries = @($Script:LedgerData.sprints)
  $errors = @()
  foreach ($entry in $entries) {
    $sid = $entry.sprint_id
    $commit = $entry.commit
    if (-not $commit -or $commit -eq "") {
      $errors += "$sid has empty commit"
    }
  }
  if ($errors.Count -gt 0) {
    return ($errors -join "; ")
  }
  $true
}

# ============================================================================
# Check 11: receipt_path files exist (when not null)
# ============================================================================

Add-Check -Name "Receipt files exist" -Block {
  $entries = @($Script:LedgerData.sprints)
  $errors = @()
  foreach ($entry in $entries) {
    $sid = $entry.sprint_id
    $rpath = $entry.receipt_path
    if ($rpath -and $rpath -ne "") {
      $fullPath = Join-Path -Path $RepoRoot -ChildPath $rpath
      if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
        $errors += "$sid receipt not found: $rpath"
      }
    }
  }
  if ($errors.Count -gt 0) {
    return ($errors -join "; ")
  }
  $true
}

# ============================================================================
# Check 12: sprint_doc_path files exist (when not null)
# ============================================================================

Add-Check -Name "Sprint doc files exist" -Block {
  $entries = @($Script:LedgerData.sprints)
  $errors = @()
  foreach ($entry in $entries) {
    $sid = $entry.sprint_id
    $dpath = $entry.sprint_doc_path
    if ($dpath -and $dpath -ne "") {
      $fullPath = Join-Path -Path $RepoRoot -ChildPath $dpath
      if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
        $errors += "$sid sprint doc not found: $dpath"
      }
    }
  }
  if ($errors.Count -gt 0) {
    return ($errors -join "; ")
  }
  $true
}

# ============================================================================
# Check 13: Sprint status values are valid
# ============================================================================

Add-Check -Name "Sprint status values are valid" -Block {
  $entries = @($Script:LedgerData.sprints)
  $errors = @()
  foreach ($entry in $entries) {
    $sid = $entry.sprint_id
    if ($ValidStatuses -notcontains $entry.status) {
      $errors += "$sid has invalid status '$($entry.status)'"
    }
  }
  if ($errors.Count -gt 0) {
    return ($errors -join "; ")
  }
  $true
}

# ============================================================================
# Check 14: Sprint IDs are unique
# ============================================================================

Add-Check -Name "Sprint IDs are unique" -Block {
  $ids = @($Script:LedgerData.sprints | ForEach-Object { $_.sprint_id })
  $duplicates = $ids | Group-Object | Where-Object { $_.Count -gt 1 }
  if ($duplicates.Count -gt 0) {
    $dupNames = ($duplicates | ForEach-Object { $_.Name }) -join ", "
    return "Duplicate sprint_id(s): $dupNames"
  }
  $true
}

# ============================================================================
# Check 15: schema_version is non-empty
# ============================================================================

Add-Check -Name "schema_version is present" -Block {
  $val = $Script:LedgerData.schema_version
  if (-not $val -or $val -eq "") { return "schema_version is missing or empty" }
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
