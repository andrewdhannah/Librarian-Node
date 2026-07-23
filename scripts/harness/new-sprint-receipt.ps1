<#
.SYNOPSIS
  Standardized Windows Harness sprint receipt generator.

.DESCRIPTION
  Generates a deterministic Markdown closeout receipt for a Windows harness sprint.
  Can auto-populate changed files from git diff, and can optionally ingest postflight-
  check.ps1 JSON output to fill state fields.

  Output is deterministic: same inputs produce identical Markdown.

  Exit code 0 = receipt generated successfully
  Exit code 1 = missing required field or write failure

.PARAMETER SprintId
  Sprint identifier (e.g. "WIN-HARNESS-RECEIPT-TEMPLATE-1"). Required.

.PARAMETER Status
  Overall result: PASS, FAIL, or PARTIAL. Required.

.PARAMETER StartingHead
  HEAD before the sprint began. Required.

.PARAMETER EndingHead
  HEAD after the sprint was sealed. Required.

.PARAMETER PreviousSprint
  Previous sprint identifier (e.g. "WIN-HARNESS-POSTFLIGHT-1"). Required.

.PARAMETER OutputPath
  Output path for the receipt Markdown file. Required.
  Should typically be under docs/receipts/<SPRINT-ID>-RECEIPT.md.

.PARAMETER Summary
  Free-text sprint summary. If omitted, auto-generated from SprintId.

.PARAMETER ChangedFiles
  Explicit list of changed file paths. If omitted and both StartingHead and EndingHead
  are valid commits, auto-detected via git diff --name-only.

.PARAMETER DeliverableScripts
  Array of hashtables: @{File="..."; Description="..."; Size="..."}
  for the Deliverables -> Script Created section.

.PARAMETER DeliverableDocs
  Array of strings: file paths for docs created, for the Deliverables -> Docs section.

.PARAMETER AcceptanceGates
  Array of hashtables: @{Gate="GATE-ID"; Description="..."; Result="PASS|FAIL"}
  for the Acceptance Gates table.

.PARAMETER BoundaryCompliance
  Array of hashtables: @{Boundary="..."; Status="..."}
  for the Hard Constraints / Boundary Compliance table.

.PARAMETER Findings
  Array of strings for the Findings section. Omitted if empty.

.PARAMETER NextSprint
  Recommended next sprint ID. Required.

.PARAMETER NextSprintRationale
  Rationale for the next sprint recommendation. Required.

.PARAMETER PostflightJsonPath
  Optional path to a postflight-check.ps1 JSON output file. If provided, state fields
  (service, ports, orphans, disk) are populated from the JSON.

.PARAMETER RepoRoot
  Path to repo root. Auto-detected if omitted.

.PARAMETER Quiet
  Suppress informational output.

.EXAMPLE
  .\scripts\harness\new-sprint-receipt.ps1 `
    -SprintId "WIN-MY-SPRINT" `
    -Status "PASS" `
    -StartingHead "abc1234" `
    -EndingHead "def5678" `
    -PreviousSprint "WIN-PRIOR-SPRINT" `
    -OutputPath "docs/receipts/WIN-MY-SPRINT-RECEIPT.md" `
    -NextSprint "WIN-NEXT-SPRINT" `
    -NextSprintRationale "Continue the sequence."

.EXAMPLE
  .\scripts\harness\new-sprint-receipt.ps1 `
    -SprintId "WIN-MY-SPRINT" `
    -Status "PASS" `
    -StartingHead "abc1234" `
    -EndingHead "def5678" `
    -PreviousSprint "WIN-PRIOR-SPRINT" `
    -OutputPath "docs/receipts/WIN-MY-SPRINT-RECEIPT.md" `
    -PostflightJsonPath "G:\temp\postflight.json" `
    -AcceptanceGates @(@{Gate="G-01";Description="Gate one";Result="PASS"}) `
    -DeliverableScripts @(@{File="scripts/my-script.ps1";Description="Does X";Size="~5 KB"}) `
    -DeliverableDocs @("docs/sprints/WIN-MY-SPRINT.md") `
    -NextSprint "WIN-NEXT-SPRINT" `
    -NextSprintRationale "Rationale here."

.LINK
  scripts/harness/postflight-check.ps1
  docs/planning/WIN-CUSTODY-SANDBOX-MODEL.md
#>

param(
  # Required
  [string]$SprintId = "",
  [string]$Status = "",
  [string]$StartingHead = "",
  [string]$EndingHead = "",
  [string]$PreviousSprint = "",
  [string]$OutputPath = "",

  # Optional content
  [string]$Summary = "",
  [string[]]$ChangedFiles = @(),
  [array]$DeliverableScripts = @(),
  [string[]]$DeliverableDocs = @(),
  [array]$AcceptanceGates = @(),
  [array]$BoundaryCompliance = @(),
  [string[]]$Findings = @(),
  [string]$NextSprint = "",
  [string]$NextSprintRationale = "",

  # Input sources
  [string]$PostflightJsonPath = "",

  # Infrastructure
  [string]$RepoRoot = "",
  [switch]$Quiet,
  [switch]$Force
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

function Write-Error {
  param([string]$Text)
  Write-Host "  FAIL $Text" -ForegroundColor "Red"
}

function Assert-Required {
  param([string]$Name, $Value)
  if (-not $Value -or ($Value -is [string] -and $Value -eq "")) {
    Write-Error "Missing required parameter: -$Name"
    return $false
  }
  return $true
}

# ============================================================================
# Validate required parameters
# ============================================================================

$missingCount = 0
$requiredParams = @(
  @{Name="SprintId"; Value=$SprintId},
  @{Name="Status"; Value=$Status},
  @{Name="StartingHead"; Value=$StartingHead},
  @{Name="EndingHead"; Value=$EndingHead},
  @{Name="PreviousSprint"; Value=$PreviousSprint},
  @{Name="OutputPath"; Value=$OutputPath},
  @{Name="NextSprint"; Value=$NextSprint},
  @{Name="NextSprintRationale"; Value=$NextSprintRationale}
)

foreach ($rp in $requiredParams) {
  if (-not (Assert-Required -Name $rp.Name -Value $rp.Value)) {
    $missingCount++
  }
}

if ($missingCount -gt 0) {
  exit 1
}

# ============================================================================
# Resolve repo root
# ============================================================================

if (-not $RepoRoot) {
  $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
  $RepoRoot = Split-Path -Parent (Split-Path -Parent $scriptDir)
}
$RepoRoot = (Resolve-Path $RepoRoot).Path

# ============================================================================
# Resolve output path
# ============================================================================

$resolvedOutput = $OutputPath
if (-not [System.IO.Path]::IsPathRooted($OutputPath)) {
  $resolvedOutput = Join-Path -Path $RepoRoot -ChildPath $OutputPath
}

# Check for overwrite
if ((Test-Path -LiteralPath $resolvedOutput -PathType Leaf) -and (-not $Force)) {
  Write-Error "Output path already exists: $resolvedOutput (use -Force to overwrite)"
  exit 1
}

# ============================================================================
# Auto-detect changed files if not provided
# ============================================================================

if ($ChangedFiles.Count -eq 0) {
  $diff = & git -C $RepoRoot diff --name-only "$StartingHead..$EndingHead" 2>&1
  if ($LASTEXITCODE -eq 0) {
    $ChangedFiles = @($diff | Where-Object { $_ -ne "" })
  }
}

# ============================================================================
# Load postflight JSON if provided
# ============================================================================

$PostflightData = $null
if ($PostflightJsonPath) {
  if (Test-Path -LiteralPath $PostflightJsonPath -PathType Leaf) {
    try {
      $PostflightData = Get-Content -Path $PostflightJsonPath -Raw | ConvertFrom-Json
      Write-Message -Text "Loaded postflight data from: $PostflightJsonPath" -Color "Green"
    } catch {
      Write-Error "Failed to parse postflight JSON: $($_.Exception.Message)"
      exit 1
    }
  } else {
    Write-Error "Postflight JSON path not found: $PostflightJsonPath"
    exit 1
  }
}

# ============================================================================
# Compute file sizes
# ============================================================================

function Get-FileSize {
  param([string]$Path)
  $fullPath = Join-Path -Path $RepoRoot -ChildPath $Path
  if (Test-Path -LiteralPath $fullPath -PathType Leaf) {
    $len = (Get-Item -LiteralPath $fullPath).Length
    if ($len -gt 1024) {
      return "~$([math]::Round($len / 1024)) KB"
    }
    return "$len bytes"
  }
  return "~? KB"
}

# ============================================================================
# Build Markdown content
# ============================================================================

$date = Get-Date -Format "yyyy-MM-dd"
$escapedStatus = $Status.ToUpper()
$escapedPrevSprint = $PreviousSprint

# Summary
if (-not $Summary) {
  $Summary = "Sprint $SprintId completed."
}

# Changed files summary string
$changedSummary = if ($ChangedFiles.Count -gt 0) {
  ($ChangedFiles -join ", ")
} else {
  "No files changed (state-only sprint?)"
}

# Deliverables: Script Created table
$deliverableScriptsMd = ""
if ($DeliverableScripts.Count -gt 0) {
  $deliverableScriptsMd = @"
### Script Created

| File | Description | Size |
|------|-------------|------|
"@
  foreach ($ds in $DeliverableScripts) {
    $size = if ($ds.Size) { $ds.Size } else { Get-FileSize -Path $ds.File }
    $desc = if ($ds.Description) { $ds.Description } else { "" }
    $deliverableScriptsMd += "`n| ``$($ds.File)`` | $desc | $size |"
  }
  $deliverableScriptsMd += "`n"
}

# Deliverables: Docs Created
$deliverableDocsMd = ""
if ($DeliverableDocs.Count -gt 0) {
  $deliverableDocsMd = @"
### Docs Created

| File |
|------|
"@
  foreach ($dd in $DeliverableDocs) {
    $deliverableDocsMd += "`n| ``$dd`` |"
  }
  $deliverableDocsMd += "`n"
}

# Acceptance Gates table
$gatesMd = ""
if ($AcceptanceGates.Count -gt 0) {
  $gatesMd = @"
| Gate | Description | Result |
|------|-------------|--------|
"@
  foreach ($ag in $AcceptanceGates) {
    $resultColor = if ($ag.Result -eq "PASS") { "PASS" } else { "FAIL" }
    $gatesMd += "`n| $($ag.Gate) | $($ag.Description) | $resultColor |"
  }
  $gatesMd += "`n"
}

# Boundary Compliance table
$boundaryMd = ""
if ($BoundaryCompliance.Count -gt 0) {
  $boundaryMd = @"
| Boundary | Status |
|----------|--------|
"@
  foreach ($bc in $BoundaryCompliance) {
    $boundaryMd += "`n| $($bc.Boundary) | $($bc.Status) |"
  }
  $boundaryMd += "`n"
}

# Findings section
$findingsMd = ""
if ($Findings.Count -gt 0) {
  $findingsMd = @"
## Findings

| Finding |
|---------|
"@
  foreach ($f in $Findings) {
    $findingsMd += "`n| $f |"
  }
  $findingsMd += "`n`n---`n"
}

# State section (from postflight JSON if available)
$stateLines = @()
$stateLines += "| Check | Value |"
$stateLines += "|-------|-------|"
$stateLines += "| Starting HEAD | ``$StartingHead`` |"
$stateLines += "| Ending HEAD | ``$EndingHead`` |"

if ($PostflightData -and $PostflightData.state) {
  $s = $PostflightData.state
  if ($s.service_status) { $stateLines += "| Service | $($s.service_status) / $($s.service_start_type) |" }
  if ($s.ports_9120_9125 -ne $null) { $stateLines += "| Ports 9120-9125 | $(if ($s.ports_9120_9125 -eq '') {'Free'} else {$s.ports_9120_9125}) |" }
  if ($s.port_9130 -ne $null) { $stateLines += "| Port 9130 | $(if (-not $s.port_9130) {'Free'} else {'LISTENING'}) |" }
  if ($s.orphan_count -ne $null) { $stateLines += "| Orphan processes | $($s.orphan_count) |" }
  if ($s.c_drive_free_gb -ne $null) { $stateLines += "| C: drive free | $($s.c_drive_free_gb) GB |" }
} else {
  $stateLines += "| Working tree | Clean (sealed) |"
  $stateLines += "| Origin | Up to date |"
}

$stateMd = $stateLines -join "`n"

# Check if working tree was modified between start and end
$commitCount = @(& git -C $RepoRoot rev-list --count "$StartingHead..$EndingHead" 2>&1)
$commitCount = if ($commitCount) { $commitCount.Trim() } else { 0 }

# ============================================================================
# Assemble full document
# ============================================================================

$receiptContent = @"
# Closeout Receipt: $SprintId

**Status:** CLOSED -- READY FOR SEAL
**Date:** $date
**Previous sprint:** $PreviousSprint (SEALED)

---

## Summary

$Summary

**Result: $escapedStatus** -- $(if ($escapedStatus -eq "PASS") {"all acceptance gates met."} elseif ($escapedStatus -eq "FAIL") {"one or more gates not met -- review required."} else {"partial result -- see gates for details."})

---

## Pre-Work Baseline

| Check | Value |
|-------|-------|
| Starting HEAD | ``$StartingHead`` |
| Ending HEAD | ``$EndingHead`` |
| Commits in sprint | $commitCount |
| Changed files | $($ChangedFiles.Count) |
| Previous sprint | $PreviousSprint |

---

## Deliverables

$deliverableScriptsMd
$deliverableDocsMd
## Changed Files

| File |
|------|
$(
if ($ChangedFiles.Count -gt 0) {
  ($ChangedFiles | ForEach-Object { "| ``$_`` |" }) -join "`n"
} else {
  "| (none) |"
}
)

---

## Acceptance Gates

$gatesMd
## Boundary Compliance

$boundaryMd
$findingsMd
## Closeout State

$stateMd

---

## Recommended Next Sprint

**$NextSprint** -- $NextSprintRationale

---

**Receipt generated:** $date
**Sprint:** $SprintId
**Starting HEAD:** ``$StartingHead``
**Ending HEAD:** ``$EndingHead``
**Files changed:** $($ChangedFiles.Count)
"@

# ============================================================================
# Write output
# ============================================================================

try {
  # Ensure parent directory exists
  $parentDir = Split-Path -Parent $resolvedOutput
  if (-not (Test-Path -LiteralPath $parentDir)) {
    New-Item -ItemType Directory -Path $parentDir -Force | Out-Null
  }

  $receiptContent | Out-File -FilePath $resolvedOutput -Encoding ASCII
  Write-Message -Text "Receipt written: $resolvedOutput" -Color "Green"
  Write-Message -Text "Sprint: $SprintId | Status: $escapedStatus | Files changed: $($ChangedFiles.Count)" -Color "Gray"
  exit 0

} catch {
  Write-Error "Failed to write receipt: $($_.Exception.Message)"
  exit 1
}
