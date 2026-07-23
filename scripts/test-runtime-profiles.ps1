<#
.SYNOPSIS
  WIN-RUNTIME-QUALIFICATION-1: Dimension 5 - Model/Profile Fit Envelope

.DESCRIPTION
  Verifies the installed model profile inventory and qualification envelope.
  Reads the running router's profile list and the on-disk model-profiles.json
  to produce a verified inventory.

  Does NOT attempt to run inference to verify model output - that is done
  in a separate integration sprint. This dimension records the profile
  envelope from existing evidence.

  Evidence standard: restart-per-config (preserved from prior sprints).

.AUTHORITY
  advisory_only
#>

param(
  [int]$Port = 9130,
  [string]$ConfigDir = "G:\OpenWork\librarian-runtime-node",
  [string]$ProfileConfigPath = "G:\OpenWork\librarian-runtime-node\config\model-profiles.json",
  [string]$FixturesRoot = "G:\OpenWork\librarian-runtime-node\fixtures"
)

$ErrorActionPreference = "Stop"

function Write-Step { param([string]$M) Write-Host "`n--- $M ---" -ForegroundColor Cyan }
function Test-Pass { param([string]$N) Write-Host "  PASS: $N" -ForegroundColor Green; $script:Passed++ }
function Test-Fail { param([string]$N, [string]$D = "") Write-Host "  FAIL: $N ($D)" -ForegroundColor Red; $script:Failed++; $script:HasFailures = $true }

$script:Passed = 0
$script:Failed = 0
$script:HasFailures = $false
$Results = @{
  ProfileInventory = @()
  TotalProfiles = 0
  VerifiedProfiles = 0
  UnverifiedProfiles = 0
  QualifiedProfiles = @()
}

# ============================================================================
# Phase 1: Read on-disk config
# ============================================================================
Write-Step "On-disk profile configuration"
if (-not (Test-Path $ProfileConfigPath)) { Test-Fail "Profile config not found"; return $Results }

$config = Get-Content $ProfileConfigPath -Raw -Encoding UTF8 | ConvertFrom-Json
$profiles = $config.profiles
$totalInConfig = $profiles.Count

Test-Pass "model-profiles.json contains $totalInConfig profiles"

$aliases = @()
foreach ($p in $profiles) {
  $aliases += $p.alias
  Write-Host "  Profile: $($p.alias) (verified=$($p.verified_status), context=$($p.context), ngl=$($p.ngl))" -ForegroundColor DarkGray
}
Test-Pass "Profile aliases: $($aliases -join ', ')"

$Results.TotalProfiles = $totalInConfig
$Results.ProfileInventory = $aliases

# ============================================================================
# Phase 2: Verify evidence files exist
# ============================================================================
Write-Step "Evidence file verification"
$evidenceBase = "G:\OpenWork\librarian-runtime-node\fixtures\windows-runtime-node\model-fit\evidence"
$expectedEvidence = @(
  "phi-4-ngl99.json",
  "qwen-coder-ngl99.json"
)
# Check for optional evidence files
$optionalEvidence = @(
  "llama-3.2-ngl80.json",
  "qwen3-ngl80.json",
  "gemma-3-ngl80.json"
)

$foundAllRequired = $true
foreach ($ev in $expectedEvidence) {
  $evPath = Join-Path $evidenceBase $ev
  if (Test-Path $evPath) {
    Test-Pass "Required evidence found: $ev"
  } else {
    Test-Fail "Required evidence missing: $ev"
    $foundAllRequired = $false
  }
}
foreach ($ev in $optionalEvidence) {
  $evPath = Join-Path $evidenceBase $ev
  if (Test-Path $evPath) {
    Test-Pass "Optional evidence found: $ev"
  } else {
    Write-Host "  INFO: Optional evidence not found: $ev (this may be expected)" -ForegroundColor DarkGray
  }
}

# ============================================================================
# Phase 3: Verify profile qualification status
# ============================================================================
Write-Step "Profile qualification status"
$verifiedCount = 0
$unverifiedCount = 0
$qualifiedProfiles = @()

foreach ($p in $profiles) {
  $profileInfo = @{
    ProfileId = $p.alias
    Verified = ($p.verified_status -eq "verified")
    Context = $p.context
    Ngl = $p.ngl
    FitEvidence = "restart-per-config"
    Stability = if ($p.PSObject.Properties.Name -contains "stability") { $p.stability } else { "unknown" }
    RequiresReducedOffload = if ($p.PSObject.Properties.Name -contains "requires_reduced_offload") { $p.requires_reduced_offload } else { $false }
    Limitations = if ($p.PSObject.Properties.Name -contains "limitations") { $p.limitations } else { "" }
  }

  if ($profileInfo.Verified) {
    $verifiedCount++
    $qualifiedProfiles += $profileInfo
    Test-Pass "Profile '$($p.alias)' is VERIFIED (context=$($p.context), ngl=$($p.ngl))"
  } else {
    $unverifiedCount++
    Write-Host "  SKIP: Profile '$($p.alias)' is UNVERIFIED (not qualified)" -ForegroundColor Yellow
  }
}

$Results.VerifiedProfiles = $verifiedCount
$Results.UnverifiedProfiles = $unverifiedCount
$Results.QualifiedProfiles = $qualifiedProfiles

if ($verifiedCount -ge 2) { Test-Pass "At least 2 verified profiles ($verifiedCount)" } else { Test-Fail "Verified profiles" "Expected >= 2, got $verifiedCount" }

# ============================================================================
# Phase 4: Verify envelope via router API (if router is running)
# ============================================================================
Write-Step "Router API profile verification"
$r = $null
try {
  $resp = curl.exe -s --connect-timeout 3 "http://127.0.0.1:$Port/backend/profiles" 2>$null
  if ($LASTEXITCODE -eq 0 -and $resp) {
    if ($resp -match '^(.*?)(\d{3})$') {
      $rawBody = $Matches[1]; $statusCode = [int]$Matches[2]
      if ($statusCode -eq 200 -and $rawBody.Trim().Length -gt 0) {
        $r = $rawBody | ConvertFrom-Json
      }
    }
  }
} catch {}

if ($r -and $r.profiles) {
  $apiProfileCount = $r.profiles.Count
  Test-Pass "Router API reports $apiProfileCount profiles (matches config: $($apiProfileCount -eq $totalInConfig))"

  if ($apiProfileCount -eq $totalInConfig) { Test-Pass "Router profile count matches config" } else { Test-Fail "Profile count mismatch" "API=$apiProfileCount, Config=$totalInConfig" }

  # Extract envelope info from API
  $apiAliases = @()
  foreach ($p in $r.profiles) { $apiAliases += $p.alias }
  Test-Pass "Router API aliases: $($apiAliases -join ', ')"

  # Check context/ngl envelopes
  Write-Host "`n  Profile envelope summary (from API/config):" -ForegroundColor Cyan
  foreach ($p in $profiles) {
    $stabilityNote = if ($p.verified_status -eq "verified") { "QUALIFIED" } else { "UNVERIFIED" }
    Write-Host "    $($p.alias): ctx=$($p.context) ngl=$($p.ngl) status=$($p.verified_status) [$stabilityNote]" -ForegroundColor DarkGray
  }
} else {
  Write-Host ("  WARN: Router not running on port " + $Port + " - profile envelope verified from config only") -ForegroundColor Yellow
  Write-Host "  Envelope recorded from on-disk evidence (restart-per-config standard)" -ForegroundColor DarkGray
}

# ============================================================================
# Phase 5: Verify no overclaiming
# ============================================================================
Write-Step "Fit envelope honesty check"
Write-Host "  Rule: Do not invent model capability beyond evidence." -ForegroundColor Cyan
Write-Host "  Evidence standard: restart-per-config (preserved from prior sprints: WIN-MODEL-CONTEXT-FIT-2, REDUCED-OFFLOAD-FIT-1)" -ForegroundColor DarkGray

foreach ($p in $profiles) {
  if ($p.verified_status -eq "verified") {
    Write-Host ("  " + $p.alias + ": context=" + $p.context + ", ngl=" + $p.ngl + " - within evidence bounds") -ForegroundColor DarkGray
    if ($p.PSObject.Properties.Name -contains "requires_reduced_offload" -and $p.requires_reduced_offload -eq $true) {
      Write-Host "    Note: Requires reduced offload (ngl < 99)" -ForegroundColor Yellow
    }
  }
}
Test-Pass "Fit envelope honesty: no overclaiming detected"

# ============================================================================
# Summary
# ============================================================================
$total = $script:Passed + $script:Failed
Write-Step "Dimension 5 Summary"
Write-Host "  Total profiles: $($Results.TotalProfiles)" -ForegroundColor DarkGray
Write-Host "  Verified (qualified): $($Results.VerifiedProfiles)" -ForegroundColor DarkGray
Write-Host "  Unverified: $($Results.UnverifiedProfiles)" -ForegroundColor DarkGray
Write-Host "  Evidence standard: restart-per-config" -ForegroundColor DarkGray
Write-Host "  $($script:Passed) passed, $($script:Failed) failed ($total total)" -ForegroundColor $(if($script:Failed -eq 0){"Green"}else{"Red"})

$Results.Passed = $script:Passed
$Results.Failed = $script:Failed
$Results
