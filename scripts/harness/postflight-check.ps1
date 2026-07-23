<#
.SYNOPSIS
  Post-flight custody verification for the Windows Agent Harness.

.DESCRIPTION
  Verifies and records environment state after a mutation sprint, producing a
  deterministic receipt. This is the post-flight counterpart to pre-mutation-check.ps1,
  completing the custody loop defined in WIN-CUSTODY-SANDBOX-MODEL.md.

  The script checks current state, diffs files against a starting HEAD, validates
  an expected changed-file allowlist, and emits a structured receipt to the console
  and optionally to a JSON file.

  Exit code 0 = ALL CHECKS PASSED -- sprint closeout valid
  Exit code 1 = ONE OR MORE CHECKS FAILED -- review before sealing

  Receipt output is deterministic: same sprint state produces same receipt content.

.PARAMETER RepoRoot
  Path to the librarian-runtime-node repo root.
  Defaults to the directory containing this script's parent's parent.

.PARAMETER SprintId
  Sprint identifier (e.g. "WIN-HARNESS-POSTFLIGHT-1"). Included in receipt output.

.PARAMETER StartingHead
  HEAD before the sprint began (required for file diff). Must be a valid SHA or ref.

.PARAMETER ExpectedHead
  Optional expected HEAD at closeout. If provided and mismatched, the HEAD check fails.

.PARAMETER ExpectedChangedFiles
  Optional allowlist of repo-relative paths expected to have changed.
  Any changed file NOT in this list causes the allowlist check to fail.
  Example: @("scripts/harness/*", "docs/sprints/*", "docs/receipts/*")

.PARAMETER RequiredSprintDoc
  Optional path to the expected sprint doc (repo-relative).
  Fails if the file does not exist at closeout.

.PARAMETER RequiredSprintReceipt
  Optional path to the expected closeout receipt (repo-relative).
  Fails if the file does not exist.

.PARAMETER ReceiptOutputPath
  Optional absolute path to write the structured receipt as JSON.
  If omitted, receipt is printed to console only.

.PARAMETER MinCdriveFreeGB
  Minimum acceptable free space on C: in gigabytes. Default 5.0.

.PARAMETER Quiet
  If set, suppress informational output; only emit pass/fail lines and receipt.

.EXAMPLE
  .\scripts\harness\postflight-check.ps1 -SprintId "WIN-HARNESS-POSTFLIGHT-1" -StartingHead "6b1abf2"

.EXAMPLE
  .\scripts\harness\postflight-check.ps1 -SprintId "WIN-MY-SPRINT" -StartingHead "abc1234" `
    -ExpectedHead "def5678" `
    -ExpectedChangedFiles @("scripts/harness/*", "docs/sprints/*.md") `
    -RequiredSprintDoc "docs/sprints/WIN-MY-SPRINT.md" `
    -ReceiptOutputPath "G:\temp\postflight-receipt.json"

.LINK
  scripts/harness/pre-mutation-check.ps1
  docs/planning/WIN-CUSTODY-SANDBOX-MODEL.md
#>

param(
  [string]$RepoRoot = "",
  [string]$SprintId = "UNKNOWN",
  [string]$StartingHead = "",
  [string]$ExpectedHead = "",
  [string[]]$ExpectedChangedFiles = @(),
  [string]$RequiredSprintDoc = "",
  [string]$RequiredSprintReceipt = "",
  [string]$ReceiptOutputPath = "",
  [double]$MinCdriveFreeGB = 5.0,
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
# Receipt accumulator (built in parallel with checks)
# ============================================================================

$Script:ReceiptData = @{
  "sprint_id" = $SprintId
  "timestamp" = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssK")
  "overall" = "unknown"
  "checks" = @{}
  "state" = @{}
  "files" = @{}
}

function Add-ReceiptField {
  param([string]$Section, [string]$Key, $Value)
  if (-not $Script:ReceiptData[$Section]) {
    $Script:ReceiptData[$Section] = @{}
  }
  $Script:ReceiptData[$Section][$Key] = $Value
}

# ============================================================================
# Resolve repo root
# ============================================================================

if (-not $RepoRoot) {
  $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
  $RepoRoot = Split-Path -Parent (Split-Path -Parent $scriptDir)
}

$RepoRoot = (Resolve-Path $RepoRoot).Path

if (-not $StartingHead) {
  Write-Host "  FAIL Parameter -StartingHead is required" -ForegroundColor "Red"
  exit 1
}

Write-Message -Text "---" -Color "DarkGray"
Write-Message -Text "Post-Flight Check -- Librarian Runtime Node" -Color "Cyan"
Write-Message -Text "Sprint: $SprintId" -Color "Gray"
Write-Message -Text "Repo root: $RepoRoot" -Color "Gray"
Write-Message -Text "Starting HEAD: $StartingHead" -Color "Gray"
Write-Message -Text "---" -Color "DarkGray"

Add-ReceiptField -Section "meta" -Key "sprint_id" -Value $SprintId
Add-ReceiptField -Section "meta" -Key "starting_head" -Value $StartingHead
Add-ReceiptField -Section "meta" -Key "repo_root" -Value $RepoRoot
Add-ReceiptField -Section "meta" -Key "timestamp" -Value (Get-Date -Format "yyyy-MM-ddTHH:mm:ssK")

# ============================================================================
# Check 1: Repo root exists
# ============================================================================

Add-Check -Name "Repo root accessible" -Block {
  $result = Test-Path -LiteralPath $RepoRoot -PathType Container
  Add-ReceiptField -Section "state" -Key "repo_accessible" -Value $result
  $result
}

# ============================================================================
# Check 2: Git HEAD
# ============================================================================

$Script:CurrentHead = ""

Add-Check -Name "Git HEAD" -Block {
  $head = & git -C $RepoRoot rev-parse --short HEAD 2>&1
  if (-not $head -or $LASTEXITCODE -ne 0) {
    return "Failed to read HEAD: $($head -join ' ')"
  }
  $head = $head.Trim()
  $Script:CurrentHead = $head
  Add-ReceiptField -Section "state" -Key "current_head" -Value $head
  Add-ReceiptField -Section "state" -Key "current_head_full" -Value (& git -C $RepoRoot rev-parse HEAD 2>&1).Trim()
  if ($ExpectedHead -and $head -ne $ExpectedHead) {
    return "HEAD is $head, expected $ExpectedHead"
  }
  $true
}

# ============================================================================
# Check 3: Working tree state
# ============================================================================

$Script:DirtyFiles = @()

Add-Check -Name "Working tree state" -Block {
  $status = & git -C $RepoRoot status --short 2>&1
  if ($LASTEXITCODE -ne 0) {
    return "git status failed: $($status -join ' ')"
  }
  $dirty = @($status | Where-Object { $_ -match '^[MADRCU?! ]' })
  $Script:DirtyFiles = $dirty
  Add-ReceiptField -Section "state" -Key "dirty_count" -Value ($dirty.Count)
  Add-ReceiptField -Section "state" -Key "dirty_files" -Value ($dirty -join "; ")
  if ($dirty.Count -gt 0) {
    return "$($dirty.Count) dirty/unstaged file(s): $($dirty -join '; ')"
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
  Add-ReceiptField -Section "state" -Key "branch" -Value $branch
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
    Add-ReceiptField -Section "state" -Key "service_status" -Value "MISSING"
    Add-ReceiptField -Section "state" -Key "service_start_type" -Value "N/A"
    return "Service not found"
  }
  Add-ReceiptField -Section "state" -Key "service_status" -Value $svc.Status.ToString()
  Add-ReceiptField -Section "state" -Key "service_start_type" -Value $svc.StartType.ToString()
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
  Add-ReceiptField -Section "state" -Key "ports_9120_9125" -Value ($occupiedPorts -join ", ")
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
  Add-ReceiptField -Section "state" -Key "port_9130" -Value ($listening -ne $null)
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

  Add-ReceiptField -Section "state" -Key "orphan_count" -Value ($orphanPids.Count)
  Add-ReceiptField -Section "state" -Key "orphan_details" -Value ($orphanPids -join "; ")
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
  Add-ReceiptField -Section "state" -Key "c_drive_free_gb" -Value $freeGB
  if ($freeGB -lt $MinCdriveFreeGB) {
    return "$freeGB GB free -- below threshold of $MinCdriveFreeGB GB"
  }
  $true
}

# ============================================================================
# Check 10: Origin/main in sync
# ============================================================================

Add-Check -Name "Origin/main in sync" -Block {
  $local = & git -C $RepoRoot rev-parse HEAD 2>&1
  if ($LASTEXITCODE -ne 0) { return "Cannot read local HEAD" }
  $remote = & git -C $RepoRoot rev-parse origin/main 2>&1
  if ($LASTEXITCODE -ne 0) { return "No origin/main ref (network fetch may be needed)" }
  $local = $local.Trim()
  $remote = $remote.Trim()
  Add-ReceiptField -Section "state" -Key "local_head_full" -Value $local
  Add-ReceiptField -Section "state" -Key "remote_head_full" -Value $remote
  if ($local -ne $remote) {
    $ahead = @(& git -C $RepoRoot rev-list --count "$remote..$local" 2>&1)
    $behind = @(& git -C $RepoRoot rev-list --count "$local..$remote" 2>&1)
    return "Local ($($local.Substring(0,7))) differs from origin/main ($($remote.Substring(0,7))) -- ahead $ahead, behind $behind"
  }
  $true
}

# ============================================================================
# Check 11: Changed file summary (diff from StartingHead)
# ============================================================================

$Script:ChangedFiles = @()

Add-Check -Name "Changed file summary" -Block {
  # Verify StartingHead exists
  $startExists = & git -C $RepoRoot cat-file -e "$StartingHead^{commit}" 2>&1
  if ($LASTEXITCODE -ne 0) {
    return "Starting HEAD '$StartingHead' is not a valid commit in this repo"
  }

  $diff = & git -C $RepoRoot diff --name-only "$StartingHead..HEAD" 2>&1
  if ($LASTEXITCODE -ne 0) {
    return "Failed to diff $StartingHead..HEAD: $($diff -join ' ')"
  }
  $Script:ChangedFiles = @($diff | Where-Object { $_ -ne "" })

  Add-ReceiptField -Section "files" -Key "changed_count" -Value ($Script:ChangedFiles.Count)
  Add-ReceiptField -Section "files" -Key "changed_files" -Value ($Script:ChangedFiles -join "; ")

  if ($Script:ChangedFiles.Count -eq 0) {
    return "No files changed between $StartingHead and HEAD -- was any work done?"
  }
  $true
}

# ============================================================================
# Check 12: Expected changed file allowlist
# ============================================================================

if ($ExpectedChangedFiles.Count -gt 0) {
  Add-Check -Name "Changed file allowlist" -Block {
    # Build a simple glob matcher: each pattern in ExpectedChangedFiles is checked
    # as a substring/regex match against each changed file path.
    $unexpected = @()
    foreach ($changedFile in $Script:ChangedFiles) {
      $matched = $false
      foreach ($pattern in $ExpectedChangedFiles) {
        # Convert simple glob to regex (escape dots, replace * with .*)
        $regex = '^' + [regex]::Escape($pattern).Replace('\*', '.*').Replace('\?', '.') + '$'
        if ($changedFile -match $regex) {
          $matched = $true
          break
        }
      }
      if (-not $matched) {
        $unexpected += $changedFile
      }
    }

    Add-ReceiptField -Section "files" -Key "unexpected_changes" -Value ($unexpected -join "; ")
    if ($unexpected.Count -gt 0) {
      return "Unexpected changed file(s): $($unexpected -join '; ')"
    }
    $true
  }
} else {
  Add-Check -Name "Changed file allowlist" -Block {
    Add-ReceiptField -Section "files" -Key "allowlist_used" -Value $false
    "No allowlist provided -- skipping (INFO)"
  }
}

# ============================================================================
# Check 13: Required sprint doc exists
# ============================================================================

if ($RequiredSprintDoc) {
  Add-Check -Name "Required sprint doc exists" -Block {
    $fullPath = Join-Path -Path $RepoRoot -ChildPath $RequiredSprintDoc
    $exists = Test-Path -LiteralPath $fullPath -PathType Leaf
    Add-ReceiptField -Section "files" -Key "sprint_doc" -Value $RequiredSprintDoc
    Add-ReceiptField -Section "files" -Key "sprint_doc_exists" -Value $exists
    if (-not $exists) {
      return "Missing required sprint doc: $RequiredSprintDoc"
    }
    $true
  }
}

# ============================================================================
# Check 14: Required sprint receipt exists
# ============================================================================

if ($RequiredSprintReceipt) {
  Add-Check -Name "Required sprint receipt exists" -Block {
    $fullPath = Join-Path -Path $RepoRoot -ChildPath $RequiredSprintReceipt
    $exists = Test-Path -LiteralPath $fullPath -PathType Leaf
    Add-ReceiptField -Section "files" -Key "sprint_receipt" -Value $RequiredSprintReceipt
    Add-ReceiptField -Section "files" -Key "sprint_receipt_exists" -Value $exists
    if (-not $exists) {
      return "Missing required sprint receipt: $RequiredSprintReceipt"
    }
    $true
  }
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
# Emit receipt
# ============================================================================

$Script:ReceiptData["overall"] = if ($overallPass) { "PASS" } else { "FAIL" }
$Script:ReceiptData["summary"] = @{
  "total" = $Script:TotalChecks
  "passed" = $Script:PassedChecks
  "failed" = $Script:FailedChecks
}

# Print receipt block
Write-Host "--- RECEIPT ---" -ForegroundColor "Cyan"
$receiptJson = $Script:ReceiptData | ConvertTo-Json -Depth 4
Write-Host $receiptJson -ForegroundColor "Gray"
Write-Host "--- END RECEIPT ---" -ForegroundColor "Cyan"

# Write receipt file if path provided
if ($ReceiptOutputPath) {
  try {
    $parentDir = Split-Path -Parent $ReceiptOutputPath
    if (-not (Test-Path -LiteralPath $parentDir)) {
      New-Item -ItemType Directory -Path $parentDir -Force | Out-Null
    }
    $receiptJson | Out-File -FilePath $ReceiptOutputPath -Encoding ASCII
    Write-Message -Text "Receipt written to: $ReceiptOutputPath" -Color "Green"
  } catch {
    Write-Host "  FAIL Failed to write receipt: $($_.Exception.Message)" -ForegroundColor "Red"
  }
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
