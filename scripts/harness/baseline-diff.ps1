<#
.SYNOPSIS
  Baseline drift detection tool for the Windows Agent Harness.

.DESCRIPTION
  Reads the frozen environment baseline from
  docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md,
  queries current machine state, and reports deviations.

  READ-ONLY: This tool never starts/stops services, never modifies state,
  never writes to disk. Drift report only.

  Exit code 0 = ALL COMPARED SECTIONS MATCH BASELINE
  Exit code 1 = ANY SECTION DRIFTED, or unknown section name

.PARAMETER BaselinePath
  Path to the baseline Markdown file. Defaults to
  docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md relative to RepoRoot.

.PARAMETER ListSections
  List available comparison sections. Does not query current state.

.PARAMETER Section
  One or more section keys (from -ListSections) to compare.

.PARAMETER All
  Compare all sections against baseline.

.PARAMETER Json
  Emit structured JSON drift report to stdout. Deterministic format.

.PARAMETER RepoRoot
  Path to the librarian-runtime-node repo root. Auto-detected.

.PARAMETER Quiet
  Suppress human-readable output.

.EXAMPLE
  .\scripts\harness\baseline-diff.ps1 -ListSections

.EXAMPLE
  .\scripts\harness\baseline-diff.ps1 -All

.EXAMPLE
  .\scripts\harness\baseline-diff.ps1 -Section service_state,port_state

.EXAMPLE
  .\scripts\harness\baseline-diff.ps1 -All -Json -Quiet

.LINK
  docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md
  scripts/harness/run-contract-checks.ps1
#>

param(
  [string]$BaselinePath = "",
  [switch]$ListSections,
  [string[]]$Section = @(),
  [switch]$All,
  [switch]$Json,
  [string]$RepoRoot = "",
  [switch]$Quiet
)

# ============================================================================
# Constants
# ============================================================================

$Script:ToolVersion = "1.0.0"
$Script:ToolId = "WIN-HARNESS-BASELINE-DIFF-1"

# ============================================================================
# Helper functions
# ============================================================================

function Write-Message {
  param([string]$Text, [string]$Color = "White")
  if (-not $Quiet) {
    Write-Host $Text -ForegroundColor $Color
  }
}

function Write-ResultLine {
  param([string]$Mark, [string]$Name, [string]$Detail, [string]$Color)
  if ($Quiet) { return }
  $extra = if ($Detail) { " -- $Detail" } else { "" }
  Write-Host "  $mark $Name$extra" -ForegroundColor $Color
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
# Resolve baseline path
# ============================================================================

if (-not $BaselinePath) {
  $BaselinePath = Join-Path -Path $RepoRoot -ChildPath "docs\planning\WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md"
}
if (-not (Test-Path -LiteralPath $BaselinePath -PathType Leaf)) {
  Write-Host "  FAIL Baseline not found: $BaselinePath" -ForegroundColor "Red"
  exit 1
}
$BaselinePath = (Resolve-Path $BaselinePath).Path

# ============================================================================
# Baseline Markdown reader
# ============================================================================

$Script:BaselineContent = @(Get-Content -Path $BaselinePath)
$Script:BaselineDate = ""

# Extract baseline date from header (line 4: "**Date:** 2026-06-29")
foreach ($line in $Script:BaselineContent) {
  if ($line -match '\*\*Date:\*\*\s+(\S+)') {
    $Script:BaselineDate = $Matches[1]
    break
  }
}

function Get-BaselineValue {
  param([string]$Pattern, [int]$ContextLines = 0)
  foreach ($line in $Script:BaselineContent) {
    if ($line -match $Pattern) {
      return $Matches[1]
    }
  }
  return $null
}

function Get-BaselineTableValue {
  param([string]$SectionHeader, [string]$RowPattern, [int]$ValueColumn = 2)
  $inSection = $false
  foreach ($line in $Script:BaselineContent) {
    if ($line -match "^\s*##\s+\d+\.\s+$([regex]::Escape($SectionHeader))") {
      $inSection = $true
      continue
    }
    if ($inSection -and $line -match "^\s*##\s") {
      break
    }
    if ($inSection -and $line -match $RowPattern) {
      $vals = $line -split '\|' | ForEach-Object { $_.Trim() }
      if ($ValueColumn -lt $vals.Count) {
        $v = $vals[$ValueColumn]
        # Strip Markdown formatting
        $v = $v -replace '\*\*', ''
        $v = $v -replace '✅', ''
        $v = $v -replace '❌', ''
        $v = $v -replace '`', ''
        $v = $v.Trim()
        if ($v -eq '') { continue }
        return $v
      }
    }
  }
  return $null
}

# ============================================================================
# Section registry
# ============================================================================
#
# Each section defines:
#   Key         - Short unique identifier
#   Display     - Human-readable heading
#   Description - What the section checks
#   Baseline    - Script block returning the expected value(s) from baseline doc
#   Current     - Script block returning the current value(s) from live system
#   Compare     - Script block($a=$baselineValue, $b=$currentValue) returning
#                 $true + $null if match, or $false + detail string if drift

function Get-SectionRegistry {
  $sections = @()

  # ---- Section 1: Service State (Baseline §20) ----
  # Baseline: Stopped / Manual

  $sections += @{
    Key = "service_state"
    Display = "Service State"
    Description = "LibrarianRunTimeNode service should be Stopped / Manual"
    Baseline = { @{ Status = "Stopped"; StartType = "Manual" } }
    Current = {
      $svc = Get-Service -Name "LibrarianRunTimeNode" -ErrorAction SilentlyContinue
      if (-not $svc) { return @{ Status = "MISSING"; StartType = "N/A" } }
      @{ Status = $svc.Status.ToString(); StartType = $svc.StartType.ToString() }
    }
    Compare = {
      param($a, $b)
      if ($a.Status -ne $b.Status) { return $false, "Status: baseline=Stopped current=$($b.Status)" }
      if ($a.StartType -ne $b.StartType) { return $false, "StartType: baseline=Manual current=$($b.StartType)" }
      return $true, $null
    }
  }

  # ---- Section 2: Port State (Baseline §21) ----
  # Baseline: all ports free

  $sections += @{
    Key = "port_state"
    Display = "Port State"
    Description = "Ports 9120-9125 and 9130 should be free (no LISTENING state)"
    Baseline = { @{ Free = $true; OccupiedPorts = @() } }
    Current = {
      $occupied = @()
      foreach ($port in 9120..9125 + @(9130)) {
        $listening = netstat -ano 2>$null | Select-String ":$port\s" | Select-String "LISTENING"
        if ($listening) { $occupied += $port }
      }
      @{ OccupiedPorts = $occupied; Free = ($occupied.Count -eq 0) }
    }
    Compare = {
      param($a, $b)
      if ($a.Free -ne $b.Free) {
        $bDesc = if ($b.Free) { "Free" } else { "Occupied: $($b.OccupiedPorts -join ',')" }
        return $false, "Ports: baseline=All free current=$bDesc"
      }
      return $true, $null
    }
  }

  # ---- Section 3: Orphan Process State (Baseline §22) ----
  # Baseline: 0 orphans

  $sections += @{
    Key = "orphan_processes"
    Display = "Orphan Processes"
    Description = "No orphan llama-server.exe, rust-router.exe, or python router processes"
    Baseline = { @{ Count = 0; Details = "" } }
    Current = {
      $orphanNames = @("llama-server.exe", "rust-router.exe")
      $orphans = @()
      $procs = Get-CimInstance Win32_Process -ErrorAction SilentlyContinue
      if (-not $procs) { return @{ Count = -1; Details = "Cannot enumerate processes" } }
      foreach ($proc in $procs) {
        if ($proc.Name -in $orphanNames) {
          $orphans += "$($proc.Name) PID $($proc.ProcessId)"
        }
        if ($proc.Name -eq "python.exe" -and $proc.CommandLine -match "router[\\/]router\.py") {
          $orphans += "python.exe (router) PID $($proc.ProcessId)"
        }
      }
      @{ Count = $orphans.Count; Details = ($orphans -join "; ") }
    }
    Compare = {
      param($a, $b)
      if ($a.Count -ne $b.Count) { return $false, "Orphans: baseline=$($a.Count) current=$($b.Count) ($($b.Details))" }
      return $true, $null
    }
  }

  # ---- Section 4: Disk Free Space (Baseline §7) ----
  # Baseline: C: 10.2 GB free, G: 132.3 GB free

  $sections += @{
    Key = "disk_free_space"
    Display = "Disk Free Space"
    Description = "C: and G: drive free space vs baseline"
    Baseline = {
      # Read baseline values from Markdown tables for C: and G:
      $bl = @{ "C:" = $null; "G:" = $null }
      $blContent = @(Get-Content -Path $Script:BaselinePath)
      $inDiskSection = $false
      foreach ($line in $blContent) {
        if ($line -match "^\s*##\s+7\.\s+Disks") { $inDiskSection = $true; continue }
        if ($inDiskSection -and $line -match "^\s*##\s") { break }
        if ($inDiskSection -and $line -match '^\|\s*([CDG]):') {
          $cols = $line -split '\|' | ForEach-Object { $_.Trim() }
          if ($cols.Count -ge 4) {
            $drive = $cols[1] -replace ':$', ''
            $freeStr = ($cols[4] -replace '\*', '').Trim()
            if ($drive -eq 'C' -or $drive -eq 'G') {
              $freeNum = 0
              if ($freeStr -match '([\d.]+)') { $freeNum = [double]$Matches[1] }
              $bl["$drive`:"] = $freeNum
            }
          }
        }
      }
      $bl
    }
    Current = {
      $disks = @{}
      $drives = Get-CimInstance Win32_LogicalDisk -ErrorAction SilentlyContinue
      foreach ($d in $drives) {
        $disks[$d.DeviceID] = @{
          FreeGB = [math]::Round($d.FreeSpace / 1GB, 1)
          SizeGB = [math]::Round($d.Size / 1GB, 1)
        }
      }
      $disks
    }
    Compare = {
      param($a, $b)
      $changes = @()
      foreach ($drive in @("C:", "G:")) {
        $ba = if ($a.ContainsKey($drive)) { $a[$drive] } else { $null }
        $bc = if ($b.ContainsKey($drive)) { $b[$drive] } else { $null }
        if ($ba -eq $null -or $bc -eq $null) { continue }
        $diff = $bc.FreeGB - $ba
        $pctChange = if ($ba -gt 0) { [math]::Round(($diff / $ba) * 100, 1) } else { 0 }
        if ([math]::Abs($diff) -ge 1.0) {
          $changes += "$drive baseline=$($ba)GB current=$($bc.FreeGB)GB ($($pctChange)% change)"
        }
      }
      if ($changes.Count -gt 0) { return $false, ($changes -join "; ") }
      return $true, $null
    }
  }

  # ---- Section 5: Git HEAD (Baseline §1) ----
  # Baseline: 08a8602

  $sections += @{
    Key = "git_head"
    Display = "Git HEAD"
    Description = "Git HEAD commit vs baseline HEAD (08a8602)"
    Baseline = { "08a8602" }
    Current = {
      $head = & git -C $RepoRoot rev-parse --short HEAD 2>&1
      if ($LASTEXITCODE -ne 0) { return "UNKNOWN" }
      $head.Trim()
    }
    Compare = {
      param($a, $b)
      if ($a -ne $b) { return $false, "HEAD: baseline=$a current=$b" }
      return $true, $null
    }
  }

  # ---- Section 6: Git Origin Sync (Baseline §1) ----
  # Baseline: ahead of origin/main by 20 commits

  $sections += @{
    Key = "git_origin"
    Display = "Git Origin Sync"
    Description = "Local HEAD vs origin/main sync"
    Baseline = { @{ InSync = $false; Note = "Baseline: ahead by 20" } }
    Current = {
      $local = & git -C $RepoRoot rev-parse HEAD 2>&1
      $remote = & git -C $RepoRoot rev-parse origin/main 2>&1
      if ($LASTEXITCODE -ne 0) { return @{ InSync = $false; Note = "Cannot fetch origin" } }
      $inSync = ($local.Trim() -eq $remote.Trim())
      if (-not $inSync) {
        $ahead = @(& git -C $RepoRoot rev-list --count "$remote..$local" 2>&1)
        $behind = @(& git -C $RepoRoot rev-list --count "$local..$remote" 2>&1)
        return @{ InSync = $false; Note = "ahead $ahead, behind $behind" }
      }
      @{ InSync = $true; Note = "up to date" }
    }
    Compare = {
      param($a, $b)
      if ($a.InSync -ne $b.InSync) {
        return $false, "Origin: baseline=$($a.Note) current=$($b.Note)"
      }
      return $true, $null
    }
  }

  # ---- Section 7: PowerShell Version (Baseline §9) ----
  # Baseline: 5.1.19041.7417

  $sections += @{
    Key = "ps_version"
    Display = "PowerShell Version"
    Description = "PowerShell engine version vs baseline (5.1.19041.7417)"
    Baseline = { "5.1.19041.7417" }
    Current = {
      "$($PSVersionTable.PSVersion.Major).$($PSVersionTable.PSVersion.Minor).$($PSVersionTable.PSVersion.Build)"
    }
    Compare = {
      param($a, $b)
      if ($a -ne $b) { return $false, "PS: baseline=$a current=$b" }
      return $true, $null
    }
  }

  # ---- Section 8: Python Version (Baseline §11) ----
  # Baseline: 3.14.3

  $sections += @{
    Key = "python_version"
    Display = "Python Version"
    Description = "Python interpreter version vs baseline (3.14.3)"
    Baseline = { "3.14.3" }
    Current = {
      $v = & python --version 2>&1
      if ($LASTEXITCODE -ne 0) { return "NOT_FOUND" }
      ($v -replace 'Python ', '').Trim()
    }
    Compare = {
      param($a, $b)
      if ($a -ne $b) { return $false, "Python: baseline=$a current=$b" }
      return $true, $null
    }
  }

  # ---- Section 9: Node Version (Baseline §12) ----
  # Baseline: 24.14.0

  $sections += @{
    Key = "node_version"
    Display = "Node.js Version"
    Description = "Node.js version vs baseline (24.14.0)"
    Baseline = { "24.14.0" }
    Current = {
      $v = & node --version 2>&1
      if ($LASTEXITCODE -ne 0) { return "NOT_FOUND" }
      $v.Trim() -replace 'v', ''
    }
    Compare = {
      param($a, $b)
      if ($a -ne $b) { return $false, "Node: baseline=$a current=$b" }
      return $true, $null
    }
  }

  # ---- Section 10: Rust Version (Baseline §13) ----
  # Baseline: 1.96.0

  $sections += @{
    Key = "rust_version"
    Display = "Rust Version"
    Description = "rustc version vs baseline (1.96.0)"
    Baseline = { "1.96.0" }
    Current = {
      $v = & rustc --version 2>&1
      if ($LASTEXITCODE -ne 0) { return "NOT_FOUND" }
      ($v -split ' ')[1]
    }
    Compare = {
      param($a, $b)
      if ($a -ne $b) { return $false, "Rust: baseline=$a current=$b" }
      return $true, $null
    }
  }

  # ---- Section 11: Baseline Findings (Baseline §24) ----
  # Baseline: 8 findings recorded

  $sections += @{
    Key = "baseline_findings"
    Display = "Baseline Findings"
    Description = "Re-evaluate which of the 8 baseline findings still apply"
    Baseline = { @{ Count = 8; Note = "All 8 findings recorded" } }
    Current = {
      $activeFindings = @()

      # F-001: C: drive critically low
      $cDrive = Get-CimInstance Win32_LogicalDisk -Filter "DeviceID='C:'" -ErrorAction SilentlyContinue
      if ($cDrive) {
        $freeGB = [math]::Round($cDrive.FreeSpace / 1GB, 1)
        if ($freeGB -lt 15) { $activeFindings += "F-001: C: drive low ($freeGB GB)" }
      }

      # F-002: dotnet SDK not found
      $dotnetTest = & dotnet --list-sdks 2>&1
      if ($LASTEXITCODE -ne 0 -or -not $dotnetTest) {
        $activeFindings += "F-002: dotnet SDK not found"
      }

      # F-003: MSVC compiler not in PATH
      $clTest = Get-Command cl.exe -ErrorAction SilentlyContinue
      if (-not $clTest) { $activeFindings += "F-003: MSVC compiler not in PATH" }

      # F-004: SESSION-HANDOFF.md staleness check
      $handoffPath = Join-Path -Path $RepoRoot -ChildPath "SESSION-HANDOFF.md"
      if (Test-Path -LiteralPath $handoffPath) {
        $handoffContent = Get-Content -Path $handoffPath -Raw
        if ($handoffContent -match "Updated:\s*(\S+)") {
          $handoffDate = $Matches[1]
          $activeFindings += "F-004: SESSION-HANDOFF.md updated $handoffDate"
        }
      }

      # F-005: No FEATURE-STATUS.md
      if (-not (Test-Path (Join-Path $RepoRoot "FEATURE-STATUS.md"))) {
        $activeFindings += "F-005: No FEATURE-STATUS.md"
      }

      # F-006: Multiple planning docs missing
      $missingDocs = @()
      foreach ($doc in @("WIN-AGENT-HARNESS-PLAN.md", "WIN-CUSTODY-SANDBOX-MODEL.md", "WIN-HARNESS-PARITY-ROADMAP.md", "WIN-LIBRARIAN-HOST-OPTIONS.md", "WIN-SPRINT-SEQUENCE.md")) {
        $docPath = Join-Path -Path $RepoRoot -ChildPath "docs\planning\$doc"
        if (-not (Test-Path -LiteralPath $docPath -PathType Leaf)) {
          $missingDocs += $doc
        }
      }
      if ($missingDocs.Count -gt 0) { $activeFindings += "F-006: Missing planning docs: $($missingDocs -join ', ')" }

      # F-007: Windows 10 past EOS
      $osInfo = Get-CimInstance Win32_OperatingSystem -ErrorAction SilentlyContinue
      if ($osInfo -and $osInfo.Caption -match "Windows 10") {
        $activeFindings += "F-007: Windows 10 (past EOS October 2025)"
      }

      # F-008: Multiple Ollama/LM Studio paths in PATH
      $pathDirs = $env:PATH -split ';'
      $altRuntimes = $pathDirs | Where-Object { $_ -match 'ollama|lmstudio' }
      if ($altRuntimes.Count -gt 1) { $activeFindings += "F-008: Multiple Ollama/LM Studio paths" }

      @{ Count = $activeFindings.Count; Details = ($activeFindings -join "; "); Findings = $activeFindings }
    }
    Compare = {
      param($a, $b)
      if ($b.Count -gt 0) {
        return $false, "$($b.Count) finding(s) still active: $($b.Details)"
      }
      return $true, "All $($a.Count) baseline findings resolved"
    }
  }

  return $sections
}

$Script:Registry = Get-SectionRegistry

# ============================================================================
# Mode selection & validation
# ============================================================================

$modeCount = @($ListSections -or $Section.Count -gt 0 -or $All)
if ($modeCount.Count -gt 1) {
  Write-Host "  FAIL Multiple modes specified. Use one of: -ListSections, -Section, -All" -ForegroundColor "Red"
  exit 1
}

if (-not $ListSections -and $Section.Count -eq 0 -and -not $All) {
  Write-Host "  FAIL No mode specified. Use -ListSections, -Section <key>, or -All." -ForegroundColor "Red"
  exit 1
}

# ============================================================================
# LIST mode
# ============================================================================

if ($ListSections) {
  Write-Message -Text "---" -Color "DarkGray"
  Write-Message -Text "Baseline Diff Sections -- $Script:ToolId" -Color "Cyan"
  Write-Message -Text "Baseline: $BaselinePath" -Color "Gray"
  Write-Message -Text "Baseline date: $Script:BaselineDate" -Color "Gray"
  Write-Message -Text "---" -Color "DarkGray"

  foreach ($sec in $Script:Registry | Sort-Object Key) {
    Write-Host "  $($sec.Key.PadRight(25)) $($sec.Display)" -ForegroundColor "Gray"
  }

  Write-Message -Text "`n---" -Color "DarkGray"
  Write-Message -Text "Total: $($Script:Registry.Count) comparison sections" -Color "Gray"
  Write-Message -Text "Use -Section <key> or -All to compare." -Color "Gray"
  Write-Message -Text "---" -Color "DarkGray"
  exit 0
}

# ============================================================================
# Resolve which sections to compare
# ============================================================================

$RunSections = @()

if ($All) {
  $RunSections = @($Script:Registry)
  Write-Message -Text "---" -Color "DarkGray"
  Write-Message -Text "Baseline Diff: All Sections -- $Script:ToolId" -Color "Cyan"
  Write-Message -Text "Baseline: $BaselinePath" -Color "Gray"
  Write-Message -Text "Baseline date: $Script:BaselineDate" -Color "Gray"
  Write-Message -Text "---" -Color "DarkGray"
}

if ($Section.Count -gt 0) {
  Write-Message -Text "---" -Color "DarkGray"
  Write-Message -Text "Baseline Diff: Named Sections -- $Script:ToolId" -Color "Cyan"
  Write-Message -Text "Baseline: $BaselinePath" -Color "Gray"
  Write-Message -Text "---" -Color "DarkGray"

  $unknownSections = @()
  foreach ($key in $Section) {
    $match = $Script:Registry | Where-Object { $_.Key -eq $key }
    if ($match) {
      $RunSections += $match
    } else {
      $unknownSections += $key
    }
  }
  if ($unknownSections.Count -gt 0) {
    Write-Host "  FAIL Unknown section key(s): $($unknownSections -join ', ')" -ForegroundColor "Red"
    Write-Host "  Use -ListSections to see available keys." -ForegroundColor "Yellow"
    exit 1
  }
}

# ============================================================================
# Compare sections
# ============================================================================

$Script:SectionResults = @()
$Script:MatchCount = 0
$Script:DriftCount = 0
$Script:ErrorCount = 0

foreach ($sec in $RunSections) {
  $baselineValue = $null
  $currentValue = $null
  $errorMsg = $null

  # Query baseline
  try {
    $baselineValue = & $sec.Baseline
  } catch {
    $errorMsg = "Baseline query error: $($_.Exception.Message)"
  }

  # Query current
  if (-not $errorMsg) {
    try {
      $currentValue = & $sec.Current
    } catch {
      $errorMsg = "Current query error: $($_.Exception.Message)"
    }
  }

  # Compare
  $drifted = $false
  $detail = $null
  if (-not $errorMsg) {
    try {
      $result = & $sec.Compare $baselineValue $currentValue
      $drifted = -not $result[0]
      $detail = $result[1]
    } catch {
      $errorMsg = "Compare error: $($_.Exception.Message)"
      $drifted = $true
    }
  }

  if ($errorMsg) {
    $Script:ErrorCount++
    Write-ResultLine -Mark "ERROR" -Name $sec.Display -Detail $errorMsg -Color "Red"
  } elseif ($drifted) {
    $Script:DriftCount++
    Write-ResultLine -Mark "DRIFT" -Name $sec.Display -Detail $detail -Color "Yellow"
  } else {
    $Script:MatchCount++
    Write-ResultLine -Mark "OK" -Name $sec.Display -Detail $detail -Color "Green"
  }

  # Format baseline/current for output
  $blStr = if ($baselineValue -is [hashtable] -or $baselineValue -is [PSCustomObject]) { ($baselineValue | ConvertTo-Json -Compress) } else { "$baselineValue" }
  $curStr = if ($currentValue -is [hashtable] -or $currentValue -is [PSCustomObject]) { ($currentValue | ConvertTo-Json -Compress) } else { "$currentValue" }

  $Script:SectionResults += @{
    "key" = $sec.Key
    "display" = $sec.Display
    "description" = $sec.Description
    "drifted" = if ($errorMsg) { $true } elseif ($drifted) { $true } else { $false }
    "error" = if ($errorMsg) { $errorMsg } else { $null }
    "detail" = if ($drifted -or -not $errorMsg) { $detail } else { $null }
    "baseline" = $blStr
    "current" = $curStr
  }
}

# ============================================================================
# Summary
# ============================================================================

$totalRun = $Script:MatchCount + $Script:DriftCount + $Script:ErrorCount
$overallClean = ($Script:DriftCount -eq 0 -and $Script:ErrorCount -eq 0)

Write-Message -Text "---" -Color "DarkGray"
$summaryText = "Compared $totalRun | Matched $Script:MatchCount | Drifted $Script:DriftCount | Errors $Script:ErrorCount"
Write-Message -Text "  OVERALL: $(if ($overallClean) { 'CLEAN' } else { 'DRIFT' }) ($summaryText)" -Color $(if ($overallClean) { "Green" } else { "Yellow" })
Write-Message -Text "---" -Color "DarkGray"

# ============================================================================
# JSON output
# ============================================================================

if ($Json) {
  $jsonResult = @{
    "tool_id" = $Script:ToolId
    "version" = $Script:ToolVersion
    "timestamp" = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssK")
    "baseline" = $BaselinePath
    "baseline_date" = $Script:BaselineDate
    "summary" = @{
      "total" = $totalRun
      "matched" = $Script:MatchCount
      "drifted" = $Script:DriftCount
      "errors" = $Script:ErrorCount
      "overall" = if ($overallClean) { "CLEAN" } else { "DRIFT" }
    }
    "sections" = $Script:SectionResults
  }

  $jsonText = $jsonResult | ConvertTo-Json -Depth 5
  if ($Quiet) {
    $jsonText
  } else {
    Write-Host $jsonText -ForegroundColor "Gray"
  }
}

# ============================================================================
# Exit code
# ============================================================================

if ($overallClean) {
  exit 0
} else {
  exit 1
}
