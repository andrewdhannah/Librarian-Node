<#
.SYNOPSIS
  WIN-RUNTIME-QUALIFICATION-1: Dimension 1 — Executable Artifact Identity

.DESCRIPTION
  Identifies the actual router executable being run.
  Records binary path, SHA256, build timestamp, source HEAD,
  and whether source HEAD and artifact provenance match.

  Outputs a PSObject with all identity fields.

.AUTHORITY
  advisory_only
#>

param(
  [string]$BinaryPath = "G:\OpenWork\librarian-runtime-node\rust-router\target\release\rust-router.exe",
  [string]$RuntimeNodeDir = "G:\OpenWork\librarian-runtime-node",
  [string]$RouterDir = "G:\OpenWork\librarian-runtime-node\rust-router",
  [string]$TheLibrarianDir = "G:\OpenWork\TheLibrarian-main"
)

$ErrorActionPreference = "Stop"

function Write-Step { param([string]$M) Write-Host "`n--- $M ---" -ForegroundColor Cyan }
function Test-Pass { param([string]$N) Write-Host "  PASS: $N" -ForegroundColor Green; $script:Passed++ }
function Test-Fail { param([string]$N, [string]$D = "") Write-Host "  FAIL: $N ($D)" -ForegroundColor Red; $script:Failed++ }

$script:Passed = 0
$script:Failed = 0
$Results = @{}

Write-Step "Binary Path Verification"
if (Test-Path -LiteralPath $BinaryPath) {
  $item = Get-Item -LiteralPath $BinaryPath
  $Results.BinaryPath = $BinaryPath
  Test-Pass "Binary exists at: $BinaryPath"
  Test-Pass "Binary size: $($item.Length) bytes"
} else {
  $Results.BinaryPath = $null
  Test-Fail "Binary not found at: $BinaryPath"
}

Write-Step "Binary SHA-256"
if (Test-Path -LiteralPath $BinaryPath) {
  $hash = (Get-FileHash -Path $BinaryPath -Algorithm SHA256).Hash
  $Results.SHA256 = $hash
  Test-Pass "SHA-256: $hash"
} else {
  $Results.SHA256 = $null
  Test-Fail "Cannot compute SHA-256: binary not found"
}

Write-Step "Binary Build Timestamp"
if (Test-Path -LiteralPath $BinaryPath) {
  $modUtc = (Get-Item $BinaryPath).LastWriteTimeUtc.ToString("yyyy-MM-ddTHH:mm:ssZ")
  $Results.BuildTimestamp = $modUtc
  Test-Pass "Build timestamp (UTC): $modUtc"
} else {
  $Results.BuildTimestamp = $null
  Test-Fail "Cannot get build timestamp: binary not found"
}

Write-Step "Source HEADs"
$rnHead = & git -C $RuntimeNodeDir rev-parse --short HEAD 2>$null
$tlHead = & git -C $TheLibrarianDir rev-parse --short HEAD 2>$null
$rnFullHead = & git -C $RuntimeNodeDir rev-parse HEAD 2>$null
$Results.RuntimeNodeHEAD = $rnHead
$Results.RuntimeNodeFullHEAD = $rnFullHead
$Results.TheLibrarianHEAD = $tlHead
if ($rnHead) {
  Test-Pass "runtime-node HEAD: $rnHead"
} else {
  Test-Fail "Could not get runtime-node HEAD"
}
if ($tlHead) {
  Test-Pass "TheLibrarian-main HEAD: $tlHead"
} else {
  Test-Fail "Could not get TheLibrarian-main HEAD"
}

Write-Step "Source HEAD vs Artifact Provenance"
$rnFull = & git -C $RuntimeNodeDir rev-parse HEAD 2>$null
$buildInfoPath = "$RouterDir\target\release\rust-router.exe"
# Check if the binary has embedded version info via git
$sourceMatchesArtifact = "unknown"
$reason = ""

# Check if git HEAD has any uncommitted changes that would affect the binary
$rnStatus = & git -C $RuntimeNodeDir status --porcelain 2>$null
$tlStatus = & git -C $TheLibrarianDir status --porcelain 2>$null
$hasChanges = (-not [string]::IsNullOrEmpty($rnStatus)) -or (-not [string]::IsNullOrEmpty($tlStatus))

# The binary was likely built from HEAD if there are no rust-router source changes
# Check if binary timestamp is after the last commit timestamp
$lastCommitTime = & git -C $RuntimeNodeDir log -1 --format="%ct" 2>$null
if ($lastCommitTime) {
  $commitDate = [DateTimeOffset]::FromUnixTimeSeconds([long]$lastCommitTime).UtcDateTime
  $binaryTime = (Get-Item $BinaryPath).LastWriteTimeUtc
  if ($binaryTime -ge $commitDate) {
    $sourceMatchesArtifact = $true
    $reason = "Binary timestamp ($($binaryTime.ToString('yyyy-MM-dd HH:mm:ss'))) is after last commit ($($commitDate.ToString('yyyy-MM-dd HH:mm:ss')))"
  } else {
    $sourceMatchesArtifact = $false
    $reason = "Binary timestamp ($($binaryTime.ToString('yyyy-MM-dd HH:mm:ss'))) is BEFORE last commit ($($commitDate.ToString('yyyy-MM-dd HH:mm:ss')))"
  }
} else {
  $sourceMatchesArtifact = $false
  $reason = "Could not determine last commit timestamp"
}

$Results.SourceMatchesArtifact = $sourceMatchesArtifact
$Results.SourceProvenanceReason = $reason

if ($sourceMatchesArtifact -eq $true) {
  Test-Pass "Source HEAD matches artifact provenance: $reason"
} else {
  Test-Fail "Source HEAD matches artifact" $reason
}

Write-Step "Artifact Identity Summary"
  Write-Host ("  Binary Path: " + $Results.BinaryPath) -ForegroundColor DarkGray
  Write-Host ("  SHA-256: " + $Results.SHA256) -ForegroundColor DarkGray
  Write-Host ("  Build Timestamp (UTC): " + $Results.BuildTimestamp) -ForegroundColor DarkGray
  Write-Host ("  runtime-node HEAD: " + $Results.RuntimeNodeHEAD) -ForegroundColor DarkGray
  Write-Host ("  TheLibrarian HEAD: " + $Results.TheLibrarianHEAD) -ForegroundColor DarkGray
Write-Host "  Source matches artifact: $sourceMatchesArtifact" -ForegroundColor $(if($sourceMatchesArtifact -eq $true){"Green"}else{"Yellow"})
Write-Host "  Reason: $reason" -ForegroundColor DarkGray

$total = $script:Passed + $script:Failed
Write-Host "`nDimension 1: $($script:Passed) passed, $($script:Failed) failed ($total total)" -ForegroundColor $(if($script:Failed -eq 0){"Green"}else{"Red"})

# Return results as object
$Results.Passed = $script:Passed
$Results.Failed = $script:Failed
$Results
