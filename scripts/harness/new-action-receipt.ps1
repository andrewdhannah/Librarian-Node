<#
.SYNOPSIS
  Standardized Windows Harness granular action receipt generator.

.DESCRIPTION
  Generates a deterministic Markdown (and optional JSON) closeout receipt for a
  discrete Windows harness action. Action receipts capture individual bounded
  execution events such as preflight run, postflight run, contract-check run,
  baseline-diff run, receipt generation, or ledger validation.

  Output is deterministic: identical inputs produce identical Markdown and JSON
  output unless the -Timestamp switch is explicitly passed.

  Exit code 0 = receipt generated successfully
  Exit code 1 = missing required field, invalid action type, or write failure

.PARAMETER ActionId
  Unique action identifier (e.g. "WIN-HARNESS-AR-001"). Required.

.PARAMETER SprintId
  Sprint identifier this action belongs to (e.g. "WIN-HARNESS-ACTION-RECEIPTS-1").
  Required.

.PARAMETER ActionType
  Type of action. Must be one of the recognized action types:
    preflight_check, postflight_check, contract_runner, baseline_diff,
    ledger_validation, receipt_generation, toolchain_check, manual_owner_action,
    read_only_investigation. Required.

.PARAMETER CommandInvoked
  The command or script that was invoked. Required.

.PARAMETER CustodyClass
  Custody classification (e.g. "read_only", "controlled_mutation", "audit").
  Required.

.PARAMETER AllowedMutationScope
  Comma-separated list of file globs or directories that were allowed to change.
  Required.

.PARAMETER ForbiddenMutationScope
  Comma-separated list of file globs or directories that were forbidden from
  changing. Required.

.PARAMETER StartingHead
  HEAD before the action was performed. Required.

.PARAMETER EndingHead
  HEAD after the action was performed. Required.

.PARAMETER ExitCode
  Process exit code from the action. Required.

.PARAMETER Result
  Overall result: PASS, FAIL, or PARTIAL. Required.

.PARAMETER OutputPath
  Output path for the receipt Markdown file. Required.
  Should be under docs/receipts/actions/<ACTION-ID>.md by convention.

.PARAMETER WorkingTreeBefore
  Working tree state before the action (e.g. "Clean", "Modified files: foo.txt").
  Optional.

.PARAMETER WorkingTreeAfter
  Working tree state after the action. Optional.

.PARAMETER EvidencePaths
  Array of evidence file paths supporting this action. Optional.

.PARAMETER Notes
  Array of notes or findings strings. Optional.

.PARAMETER JsonOutputPath
  Optional path for JSON output. If provided, a JSON representation of the receipt
  is written alongside the Markdown.

.PARAMETER Timestamp
  Switch. If provided, include a timestamp in the output (non-deterministic).
  Without this flag, the date field is "DETERMINISTIC" to ensure repeatable output.

.PARAMETER RepoRoot
  Path to repo root. Auto-detected from script location if omitted.

.PARAMETER Quiet
  Suppress informational output.

.EXAMPLE
  .\scripts\harness\new-action-receipt.ps1 `
    -ActionId "WIN-HARNESS-AR-001" `
    -SprintId "WIN-HARNESS-ACTION-RECEIPTS-1" `
    -ActionType "preflight_check" `
    -CommandInvoked ".\scripts\harness\pre-mutation-check.ps1 -ExpectedHead 44d1bcf" `
    -CustodyClass "controlled_mutation" `
    -AllowedMutationScope "scripts/harness/, docs/receipts/actions/" `
    -ForbiddenMutationScope "rust-router/, runtime/bin/" `
    -StartingHead "44d1bcf" `
    -EndingHead "44d1bcf" `
    -ExitCode 0 `
    -Result "PASS" `
    -OutputPath "docs/receipts/actions/WIN-HARNESS-AR-001.md" `
    -Notes @("All 11 pre-mutation checks passed") `
    -JsonOutputPath "docs/receipts/actions/WIN-HARNESS-AR-001.json"

.EXAMPLE
  .\scripts\harness\new-action-receipt.ps1 `
    -ActionId "WIN-HARNESS-AR-002" `
    -SprintId "WIN-HARNESS-ACTION-RECEIPTS-1" `
    -ActionType "contract_runner" `
    -CommandInvoked ".\scripts\harness\run-contract-checks.ps1 -AllSafe" `
    -CustodyClass "read_only" `
    -AllowedMutationScope "none" `
    -ForbiddenMutationScope "all" `
    -StartingHead "44d1bcf" `
    -EndingHead "44d1bcf" `
    -ExitCode 0 `
    -Result "PASS" `
    -OutputPath "docs/receipts/actions/WIN-HARNESS-AR-002.md" `
    -WorkingTreeBefore "Clean" `
    -WorkingTreeAfter "Clean"

.LINK
  scripts/harness/new-sprint-receipt.ps1
  scripts/harness/pre-mutation-check.ps1
  scripts/harness/postflight-check.ps1
#>

param(
  # Required
  [string]$ActionId = "",
  [string]$SprintId = "",
  [string]$ActionType = "",
  [string]$CommandInvoked = "",
  [string]$CustodyClass = "",
  [string]$AllowedMutationScope = "",
  [string]$ForbiddenMutationScope = "",
  [string]$StartingHead = "",
  [string]$EndingHead = "",
  [int]$ExitCode = -1,
  [string]$Result = "",
  [string]$OutputPath = "",

  # Optional content
  [string]$WorkingTreeBefore = "",
  [string]$WorkingTreeAfter = "",
  [string[]]$EvidencePaths = @(),
  [string[]]$Notes = @(),

  # Optional JSON output
  [string]$JsonOutputPath = "",

  # Non-deterministic switch
  [switch]$Timestamp,

  # Infrastructure
  [string]$RepoRoot = "",
  [switch]$Quiet
)

# ============================================================================
# Constants
# ============================================================================

$ValidActionTypes = @(
  "preflight_check",
  "postflight_check",
  "contract_runner",
  "baseline_diff",
  "ledger_validation",
  "receipt_generation",
  "toolchain_check",
  "manual_owner_action",
  "read_only_investigation"
)

$ValidResults = @("PASS", "FAIL", "PARTIAL")

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
  if ($null -eq $Value -or ($Value -is [string] -and $Value -eq "")) {
    Write-Error "Missing required parameter: -$Name"
    return $false
  }
  if ($Name -eq "ExitCode" -and $Value -eq -1) {
    Write-Error "Missing required parameter: -ExitCode (must be provided, -1 is sentinel)"
    return $false
  }
  return $true
}

function Escape-Markdown {
  param([string]$Text)
  # Escape backtick first, then pipe for Markdown table safety
  return ($Text -replace '`', '\''' -replace '\|', '\|')
}

# ============================================================================
# Validate required parameters
# ============================================================================

$missingCount = 0

$requiredParams = @(
  @{Name="ActionId"; Value=$ActionId},
  @{Name="SprintId"; Value=$SprintId},
  @{Name="ActionType"; Value=$ActionType},
  @{Name="CommandInvoked"; Value=$CommandInvoked},
  @{Name="CustodyClass"; Value=$CustodyClass},
  @{Name="AllowedMutationScope"; Value=$AllowedMutationScope},
  @{Name="ForbiddenMutationScope"; Value=$ForbiddenMutationScope},
  @{Name="StartingHead"; Value=$StartingHead},
  @{Name="EndingHead"; Value=$EndingHead},
  @{Name="ExitCode"; Value=$ExitCode},
  @{Name="Result"; Value=$Result},
  @{Name="OutputPath"; Value=$OutputPath}
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
# Validate action type
# ============================================================================

$normalizedType = $ActionType.ToLower()
if ($ValidActionTypes -notcontains $normalizedType) {
  Write-Error "Invalid action type: '$ActionType'. Must be one of: $($ValidActionTypes -join ', ')"
  exit 1
}

# ============================================================================
# Validate result
# ============================================================================

$normalizedResult = $Result.ToUpper()
if ($ValidResults -notcontains $normalizedResult) {
  Write-Error "Invalid result: '$Result'. Must be one of: $($ValidResults -join ', ')"
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

# ============================================================================
# Determine date/timestamp
# ============================================================================

if ($Timestamp) {
  $dateStr = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
} else {
  $dateStr = "DETERMINISTIC"
}

# ============================================================================
# Build Markdown sections (pre-computed to avoid nested here-strings)
# ============================================================================

$escapedActionId   = Escape-Markdown -Text $ActionId
$escapedSprintId   = Escape-Markdown -Text $SprintId
$escapedCustodyClass = Escape-Markdown -Text $CustodyClass
$escapedAllowedScope = Escape-Markdown -Text $AllowedMutationScope
$escapedForbiddenScope = Escape-Markdown -Text $ForbiddenMutationScope
$escapedCommand    = Escape-Markdown -Text $CommandInvoked

$escapedWorkBefore = if ($WorkingTreeBefore) { Escape-Markdown -Text $WorkingTreeBefore } else { "Not recorded" }
$escapedWorkAfter  = if ($WorkingTreeAfter)  { Escape-Markdown -Text $WorkingTreeAfter } else { "Not recorded" }

# ---- Evidence table ----
$evidenceLines = @("| Evidence Path |", "|---------------|")
if ($EvidencePaths.Count -gt 0) {
  foreach ($ep in $EvidencePaths) {
    $evidenceLines += "| ``$(Escape-Markdown -Text $ep)`` |"
  }
} else {
  $evidenceLines += "| (none) |"
}
$evidenceSection = $evidenceLines -join "`n"

# ---- Notes table ----
$notesLines = @("| Note |", "|------|")
if ($Notes.Count -gt 0) {
  foreach ($n in $Notes) {
    $notesLines += "| $(Escape-Markdown -Text $n) |"
  }
} else {
  $notesLines += "| (none) |"
}
$notesSection = $notesLines -join "`n"

# ============================================================================
# Assemble Markdown content
# ============================================================================

$receiptContent = @"
# Action Receipt: $escapedActionId

**Generated:** $dateStr
**Sprint:** $escapedSprintId
**Action Type:** $normalizedType
**Result:** $normalizedResult

---

## Action Details

| Field | Value |
|-------|-------|
| Action ID | ``$escapedActionId`` |
| Sprint ID | ``$escapedSprintId`` |
| Action Type | $normalizedType |
| Custody Class | $escapedCustodyClass |
| Command Invoked | ``$escapedCommand`` |
| Exit Code | $ExitCode |
| Result | $normalizedResult |

---

## Mutation Boundaries

| Boundary | Scope |
|----------|-------|
| Allowed Mutation | $escapedAllowedScope |
| Forbidden Mutation | $escapedForbiddenScope |

---

## Version Control State

| Check | Value |
|-------|-------|
| Starting HEAD | ``$StartingHead`` |
| Ending HEAD | ``$EndingHead`` |
| Working Tree Before | $escapedWorkBefore |
| Working Tree After | $escapedWorkAfter |

---

## Evidence

$evidenceSection

---

## Notes / Findings

$notesSection

---

**Receipt generated:** $dateStr
**Action:** $escapedActionId
**Sprint:** $escapedSprintId
**Result:** $normalizedResult
**Starting HEAD:** ``$StartingHead``
**Ending HEAD:** ``$EndingHead``
**Exit code:** $ExitCode
"@

# ============================================================================
# Build JSON object
# ============================================================================

$jsonObject = @{
  schema = "action-receipt/v1"
  generated_at = if ($Timestamp) { (Get-Date -Format "o") } else { "DETERMINISTIC" }
  action_id = $ActionId
  sprint_id = $SprintId
  action_type = $normalizedType
  custody_class = $CustodyClass
  allowed_mutation_scope = $AllowedMutationScope
  forbidden_mutation_scope = $ForbiddenMutationScope
  starting_head = $StartingHead
  ending_head = $EndingHead
  working_tree_before = if ($WorkingTreeBefore) { $WorkingTreeBefore } else { $null }
  working_tree_after = if ($WorkingTreeAfter) { $WorkingTreeAfter } else { $null }
  command_invoked = $CommandInvoked
  exit_code = $ExitCode
  result = $normalizedResult
  evidence_paths = if ($EvidencePaths.Count -gt 0) { @($EvidencePaths) } else { @() }
  notes = if ($Notes.Count -gt 0) { @($Notes) } else { @() }
}

# ============================================================================
# Write Markdown output
# ============================================================================

try {
  $parentDir = Split-Path -Parent $resolvedOutput
  if (-not (Test-Path -LiteralPath $parentDir)) {
    New-Item -ItemType Directory -Path $parentDir -Force | Out-Null
  }

  $receiptContent | Out-File -FilePath $resolvedOutput -Encoding ASCII
  Write-Message -Text "Action receipt written: $resolvedOutput" -Color "Green"
  Write-Message -Text "Action: $escapedActionId | Type: $normalizedType | Result: $normalizedResult" -Color "Gray"
} catch {
  Write-Error "Failed to write receipt Markdown: $($_.Exception.Message)"
  exit 1
}

# ============================================================================
# Write JSON output (optional)
# ============================================================================

if ($JsonOutputPath) {
  try {
    $resolvedJson = $JsonOutputPath
    if (-not [System.IO.Path]::IsPathRooted($JsonOutputPath)) {
      $resolvedJson = Join-Path -Path $RepoRoot -ChildPath $JsonOutputPath
    }

    $jsonDir = Split-Path -Parent $resolvedJson
    if (-not (Test-Path -LiteralPath $jsonDir)) {
      New-Item -ItemType Directory -Path $jsonDir -Force | Out-Null
    }

    $jsonObject | ConvertTo-Json -Depth 4 | Out-File -FilePath $resolvedJson -Encoding ASCII
    Write-Message -Text "Action receipt JSON written: $resolvedJson" -Color "Green"
  } catch {
    Write-Error "Failed to write receipt JSON: $($_.Exception.Message)"
    exit 1
  }
}

exit 0
