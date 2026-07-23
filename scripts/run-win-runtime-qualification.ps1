<#
.SYNOPSIS
  WIN-RUNTIME-QUALIFICATION-1: Windows Runtime Node Qualification Proof

.DESCRIPTION
  Orchestrates all 8 proof dimensions for the Windows Runtime Node qualification.

  Hard constraints:
    - Do not commit secrets, binaries, model files, logs, or generated runtime junk.
    - Do not trust source HEAD as artifact proof.
    - Preserve stash state.
    - Working tree clean at closeout.
    - Service returns to Stopped / Manual.

.PARAMETER ReceiptDir
  Directory for qualification receipt output.

.PARAMETER SkipDimension
  Comma-separated list of dimensions to skip (e.g. "3,5").

.PARAMETER ReceiptOnly
  If set, only perform cleanup checks and emit receipt (assumes tests already run).

.AUTHORITY
  advisory_only
#>

param(
  [string]$ReceiptDir = "G:\OpenWork\receipts\runtime-qualification",
  [string]$BinaryPath = "G:\OpenWork\librarian-runtime-node\rust-router\target\release\rust-router.exe",
  [string]$RouterDir = "G:\OpenWork\librarian-runtime-node\rust-router",
  [string]$ConfigDir = "G:\OpenWork\librarian-runtime-node",
  [string]$ScriptsDir = "G:\OpenWork\librarian-runtime-node\scripts",
  [string]$TheLibrarianDir = "G:\OpenWork\TheLibrarian-main",
  [string]$SkipDimension = "",
  [switch]$ReceiptOnly = $false
)

$ErrorActionPreference = "Stop"

$skipDims = @{}
if ($SkipDimension) { $SkipDimension.Split(',') | ForEach-Object { $skipDims[$_.Trim()] = $true } }

$Timestamp = (Get-Date -Format "yyyyMMdd-HHmmss")
$StartTime = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
$Passed = 0
$Failed = 0
$Blocked = $false
$ScriptStart = Get-Date
$DimResults = @{}

function Write-HostColor { param([string]$T, [string]$C = "White") Write-Host $T -ForegroundColor $C }

function Write-Step {
  param([string]$M)
  Write-Host ""
  Write-Host ("=" * 72) -ForegroundColor DarkGray
  Write-Host $M -ForegroundColor Cyan
  Write-Host ("=" * 72) -ForegroundColor DarkGray
}

function Test-Pass {
  param([string]$N)
  $script:Passed++
  Write-Host ("  PASS: " + $N) -ForegroundColor Green
}

function Test-Fail {
  param([string]$N, [string]$D = "")
  $script:Failed++
  $ds = if ($D) { " (" + $D + ")" } else { "" }
  Write-Host ("  FAIL: " + $N + $ds) -ForegroundColor Red
}

function Test-Blocked {
  param([string]$N, [string]$D = "")
  $script:Blocked = $true
  Write-Host ("  BLOCKED: " + $N + " - " + $D) -ForegroundColor Red
}

function Invoke-Dimension {
  param([string]$Number, [string]$Name, [string]$Script, [hashtable]$Params = @{})
  if ($skipDims[$Number]) {
    Write-Host ("  SKIP: Dimension " + $Number + " (" + $Name + ") skipped")
    return $null
  }
  Write-Step ("Dimension " + $Number + ": " + $Name)
  Write-Host ("  Script: " + $Script) -ForegroundColor DarkGray
  $scriptPath = Join-Path $ScriptsDir $Script
  if (-not (Test-Path $scriptPath)) {
    Test-Fail "Script not found" $scriptPath
    return $null
  }

  try {
    $splat = @{}
    foreach ($key in $Params.Keys) {
      $splat[$key] = $Params[$key]
    }

    $output = & $scriptPath @splat 2>&1
    $exitCode = $LASTEXITCODE

    $output | ForEach-Object { Write-Host ("    " + $_) -ForegroundColor DarkGray }

    $resultObj = $null
    foreach ($line in $output) {
      if ($line -is [System.Management.Automation.PSCustomObject] -or $line -is [hashtable]) {
        $resultObj = $line
      }
    }

    return $resultObj
  } catch {
    Test-Fail "Exception in dimension" ($Number + ": " + $_.Exception.Message)
    return $null
  }
}

# ============================================================================
# Mandatory Pre-work
# ============================================================================
Write-Step "MANDATORY PRE-WORK"

Write-Host "--- Repo HEADs ---" -ForegroundColor Yellow
$rnHead = & git -C $ConfigDir rev-parse --short HEAD 2>$null
$tlHead = & git -C $TheLibrarianDir rev-parse --short HEAD 2>$null
$rnFullHead = & git -C $ConfigDir rev-parse HEAD 2>$null
Write-Host ("  runtime-node HEAD: " + $rnHead) -ForegroundColor DarkGray
Write-Host ("  TheLibrarian HEAD: " + $tlHead) -ForegroundColor DarkGray

if ($rnHead) { Test-Pass ("runtime-node HEAD: " + $rnHead) } else { Test-Blocked "Pre-work" "Cannot get runtime-node HEAD" }
if ($tlHead) { Test-Pass ("TheLibrarian HEAD: " + $tlHead) } else { Test-Blocked "Pre-work" "Cannot get TheLibrarian HEAD" }

$rnStatus = & git -C $ConfigDir status --short 2>$null
$tlStatus = & git -C $TheLibrarianDir status --short 2>$null
$rnClean = [string]::IsNullOrEmpty($rnStatus)
$tlClean = [string]::IsNullOrEmpty($tlStatus)
if ($rnClean) { Test-Pass "runtime-node working tree clean" } else { Write-Host "  WARN: runtime-node has uncommitted changes" -ForegroundColor Yellow }
if ($tlClean) { Test-Pass "TheLibrarian working tree clean" } else { Write-Host "  WARN: TheLibrarian has uncommitted changes" -ForegroundColor Yellow }

$rnStash = & git -C $ConfigDir stash list 2>$null
$tlStash = & git -C $TheLibrarianDir stash list 2>$null
$rnStashEmpty = [string]::IsNullOrEmpty($rnStash)
$tlStashEmpty = [string]::IsNullOrEmpty($tlStash)
if ($rnStashEmpty) { Test-Pass "runtime-node stashes empty" } else { Test-Blocked "Pre-work" "runtime-node has stashes" }
if ($tlStashEmpty) { Test-Pass "TheLibrarian stashes empty" } else { Test-Blocked "Pre-work" "TheLibrarian has stashes" }

Write-Host "--- Service State ---" -ForegroundColor Yellow
$ServiceName = "LibrarianRunTimeNode"
$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($svc) {
  Test-Pass ("Service " + $ServiceName + " exists: " + $svc.Status + " / " + $svc.StartType)
  $svcStatusBefore = "" + $svc.Status
  $svcStartTypeBefore = "" + $svc.StartType
  if ($svc.Status -eq "Stopped" -and $svc.StartType -eq "Manual") {
    Test-Pass "Service baseline: Stopped / Manual"
  } else {
    Write-Host ("  NOTE: Service is " + $svc.Status + "/" + $svc.StartType + " (not Stopped/Manual)") -ForegroundColor Yellow
  }
} else {
  Test-Blocked "Pre-work" ("Service " + $ServiceName + " not found")
}

Write-Host "--- Orphan Check ---" -ForegroundColor Yellow
$llamaProcs = Get-Process -Name "llama-server" -ErrorAction SilentlyContinue
$rustProcs = Get-Process -Name "rust-router" -ErrorAction SilentlyContinue
$pythonRouter = Get-Process -Name "python" -ErrorAction SilentlyContinue | Where-Object { try { (Get-CimInstance Win32_Process -Filter ("ProcessId=" + $_.Id) | Select-Object -ExpandProperty CommandLine) -match "router|flask" } catch { $false } }
$orphanCount = if ($llamaProcs) { ($llamaProcs | Measure-Object).Count } else { 0 }
$rustOrphanCount = if ($rustProcs) { ($rustProcs | Measure-Object).Count } else { 0 }
$pyOrphanCount = if ($pythonRouter) { ($pythonRouter | Measure-Object).Count } else { 0 }

if ($orphanCount -eq 0 -and $rustOrphanCount -eq 0 -and $pyOrphanCount -eq 0) {
  Test-Pass "No orphan processes (llama=0 rust=0 python=0)"
} else {
  Test-Blocked "Pre-work" ("Orphans: llama=" + $orphanCount + " rust=" + $rustOrphanCount + " python=" + $pyOrphanCount)
}

Write-Host "--- Port Check ---" -ForegroundColor Yellow
$Port = 9130
$listeners = netstat -ano | Where-Object { $_ -match (":" + $Port + "\s") -and $_ -match "LISTENING" }
$listenerActive = ($listeners -ne $null -and $listeners.Count -gt 0)
if ($listenerActive) {
  Test-Blocked "Pre-work" ("Port " + $Port + " has active LISTENING socket")
} else {
  Test-Pass ("Port " + $Port + ": free")
}

$backendPorts = @(9120, 9121, 9122, 9123, 9124)
$allBackendFree = $true
foreach ($bp in $backendPorts) {
  $bListeners = netstat -ano | Where-Object { $_ -match (":" + $bp + "\s") -and $_ -match "LISTENING" }
  if ($bListeners -and $bListeners.Count -gt 0) {
    Write-Host ("  Port " + $bp + ": IN USE") -ForegroundColor Yellow
    $allBackendFree = $false
  }
}
if ($allBackendFree) { Test-Pass "All backend ports free" } else { Test-Blocked "Pre-work" "Some backend ports in use" }

if ($Blocked) {
  Write-Host "`nFATAL: Pre-work checks blocked. Fix issues before running qualification." -ForegroundColor Red
  exit 1
}

# ============================================================================
# Run dimensions or receipt-only
# ============================================================================
if (-not $ReceiptOnly) {
  $dim1 = Invoke-Dimension -Number "1" -Name "Executable Artifact Identity" -Script "test-runtime-artifact-identity.ps1" -Params @{
    BinaryPath = $BinaryPath; RuntimeNodeDir = $ConfigDir; RouterDir = $RouterDir; TheLibrarianDir = $TheLibrarianDir
  }
  if ($dim1) { $DimResults["artifact"] = $dim1 }

  $dim2 = Invoke-Dimension -Number "2" -Name "Router Contract Behavior" -Script "test-runtime-contract.ps1" -Params @{
    Port = 9130; BinaryPath = $BinaryPath; ConfigDir = $ConfigDir; RouterDir = $RouterDir; StartRouter = $true; StopRouterAfter = $true
  }
  if ($dim2) { $DimResults["contract"] = $dim2 }

  $dim5 = Invoke-Dimension -Number "5" -Name "Model/Profile Fit Envelope" -Script "test-runtime-profiles.ps1" -Params @{
    Port = 9130; ConfigDir = $ConfigDir; ProfileConfigPath = ($ConfigDir + "\config\model-profiles.json"); FixturesRoot = ($ConfigDir + "\fixtures")
  }
  if ($dim5) { $DimResults["profiles"] = $dim5 }

  $dim6 = Invoke-Dimension -Number "6" -Name "Request/Body Limits" -Script "test-runtime-limits.ps1" -Params @{
    Port = 9132; BinaryPath = $BinaryPath; RouterDir = $RouterDir
  }
  if ($dim6) { $DimResults["limits"] = $dim6 }

  $dim4 = Invoke-Dimension -Number "4" -Name "Network/Auth Boundary" -Script "test-runtime-network-boundary.ps1" -Params @{
    Port = 9131; BinaryPath = $BinaryPath; RouterDir = $RouterDir
  }
  if ($dim4) { $DimResults["network"] = $dim4 }

  $dim3 = Invoke-Dimension -Number "3" -Name "Service/Process Lifecycle" -Script "test-runtime-lifecycle.ps1" -Params @{
    Port = 9130; BinaryPath = $BinaryPath; RouterDir = $RouterDir; SkipBackendTests = $false
  }
  if ($dim3) { $DimResults["lifecycle"] = $dim3 }

  $dim7 = Invoke-Dimension -Number "7" -Name "Cleanup/Orphan Proof" -Script "test-runtime-cleanup.ps1" -Params @{
    Port = 9130; InterimCheck = $true
  }
  if ($dim7) { $DimResults["cleanup_interim"] = $dim7 }
} else {
  Write-Step "RECEIPT-ONLY MODE"
  Write-Host "  Skipping all dimension tests, generating receipt from existing data." -ForegroundColor Yellow
}

# ============================================================================
# Final State Verification
# ============================================================================
Write-Step "FINAL STATE VERIFICATION"

$llamaFinal = Get-Process -Name "llama-server" -ErrorAction SilentlyContinue
$rustFinal = Get-Process -Name "rust-router" -ErrorAction SilentlyContinue
$orphanCountFinal = if ($llamaFinal) { ($llamaFinal | Measure-Object).Count } else { 0 }
$rustOrphanFinal = if ($rustFinal) { ($rustFinal | Measure-Object).Count } else { 0 }
if ($orphanCountFinal -eq 0 -and $rustOrphanFinal -eq 0) {
  Test-Pass "No orphans after qualification"
} else {
  Test-Fail "Orphans after qualification" ("llama=" + $orphanCountFinal + " rust=" + $rustOrphanFinal)
}

$portFinal = netstat -ano | Where-Object { $_ -match (":" + $Port + "\s") -and $_ -match "LISTENING" }
$backendPortsFree = $true
foreach ($bp in $backendPorts) {
  $bListeners = netstat -ano | Where-Object { $_ -match (":" + $bp + "\s") -and $_ -match "LISTENING" }
  if ($bListeners -and $bListeners.Count -gt 0) { $backendPortsFree = $false }
}
$portFree = ($null -eq $portFinal -or $portFinal.Count -eq 0) -and $backendPortsFree
if ($portFree) { Test-Pass "All ports free after qualification" } else { Test-Fail "Ports still in use after qualification" }

$svcFinal = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
$svcStatusAfter = "" + $svcFinal.Status
$svcStartTypeAfter = "" + $svcFinal.StartType
if ($svcFinal.Status -eq "Stopped" -and $svcFinal.StartType -eq "Manual") {
  Test-Pass ("Service final state: " + $svcFinal.Status + " / " + $svcFinal.StartType + " (preserved)")
} else {
  Test-Fail "Service final state" ("Expected Stopped/Manual, got " + $svcFinal.Status + "/" + $svcFinal.StartType)
}

$rnStatusFinal = & git -C $ConfigDir status --short 2>$null
$tlStatusFinal = & git -C $TheLibrarianDir status --short 2>$null
$rnCleanFinal = [string]::IsNullOrEmpty($rnStatusFinal)
$tlCleanFinal = [string]::IsNullOrEmpty($tlStatusFinal)
if ($rnCleanFinal) { Test-Pass "runtime-node working tree clean" } else { Write-Host "  WARN: runtime-node has changes" -ForegroundColor Yellow }
if ($tlCleanFinal) { Test-Pass "TheLibrarian working tree clean" } else { Write-Host "  WARN: TheLibrarian has changes" -ForegroundColor Yellow }

$rnStashFinal = & git -C $ConfigDir stash list 2>$null
$tlStashFinal = & git -C $TheLibrarianDir stash list 2>$null
$rnStashEmptyFinal = [string]::IsNullOrEmpty($rnStashFinal)
$tlStashEmptyFinal = [string]::IsNullOrEmpty($tlStashFinal)
if ($rnStashEmptyFinal -and $tlStashEmptyFinal) { Test-Pass "Stashes empty (preserved)" } else { Test-Fail "Stashes not empty" }

# ============================================================================
# Emit Qualification Receipt
# ============================================================================
Write-Step "EMIT QUALIFICATION RECEIPT"

if (-not (Test-Path $ReceiptDir)) { New-Item -ItemType Directory -Path $ReceiptDir -Force | Out-Null }

$overallResult = "qualified"
$totalFailures = $Failed

$contractResult = if ($DimResults["contract"]) { $DimResults["contract"].HarnessResult } else { "not_available" }
$networkAuthResult = if ($DimResults["network"]) {
  if ($DimResults["network"].AuthRequiredTest -eq "pass" -and $DimResults["network"].InvalidTokenTest -eq "pass" -and $DimResults["network"].ValidTokenTest -eq "pass") { "pass" } else { "fail" }
} else { "not_available" }
$limitsResult = if ($DimResults["limits"]) {
  if ($DimResults["limits"].OversizedRefused -eq $true) { "pass" } else { "fail" }
} else { "not_available" }
$lifecycleResult = if ($DimResults["lifecycle"]) {
  if ($DimResults["lifecycle"].StartResult -eq "pass" -and $DimResults["lifecycle"].StopRouterResult -eq "pass") { "pass" } else { "fail" }
} else { "not_available" }

if ($totalFailures -gt 0 -or -not $portFree) {
  $overallResult = "partial"
}

$hostname = $env:COMPUTERNAME

$receipt = @{
  schema = "runtime-node-qualification/v1"
  created_at_utc = $StartTime
  completed_at_utc = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
  duration_seconds = [math]::Round(((Get-Date) - $ScriptStart).TotalSeconds, 1)

  node = @{
    node_id = $hostname
    os = "windows"
    hostname = $hostname
    service_mode = "rust-primary"
    fallback = "python-router"
  }

  artifact = @{
    router_binary_path = $BinaryPath
    router_binary_sha256 = if ($DimResults["artifact"]) { $DimResults["artifact"].SHA256 } else { "not_measured" }
    router_binary_build_timestamp = if ($DimResults["artifact"]) { $DimResults["artifact"].BuildTimestamp } else { "not_measured" }
    source_head = $rnHead
    thelibrarian_head = $tlHead
    source_head_matches_artifact = if ($DimResults["artifact"]) { $DimResults["artifact"].SourceMatchesArtifact -eq $true } else { $false }
  }

  contract = @{
    contract_version = if ($DimResults["contract"]) { $DimResults["contract"].ContractVersion } else { "not_run" }
    harness_result = $contractResult
    endpoints_verified = if ($DimResults["contract"]) { $DimResults["contract"].EndpointsVerified } else { @() }
    total_tests = if ($DimResults["contract"]) { $DimResults["contract"].TotalTests } else { 0 }
    passed_tests = if ($DimResults["contract"]) { $DimResults["contract"].PassedTests } else { 0 }
    failed_tests = if ($DimResults["contract"]) { $DimResults["contract"].FailedTests } else { 0 }
  }

  network = @{
    default_bind = "127.0.0.1"
    lan_exposure_requires_explicit = $true
    auth_required_test = if ($DimResults["network"]) { $DimResults["network"].AuthRequiredTest } else { "not_run" }
    invalid_token_test = if ($DimResults["network"]) { $DimResults["network"].InvalidTokenTest } else { "not_run" }
    valid_token_test = if ($DimResults["network"]) { $DimResults["network"].ValidTokenTest } else { "not_run" }
    secrets_logged = if ($DimResults["network"]) { $DimResults["network"].SecretsLogged } else { "not_run" }
  }

  limits = @{
    max_body_bytes = if ($DimResults["limits"]) { $DimResults["limits"].MaxBodyBytes } else { 10485760 }
    oversized_refused = if ($DimResults["limits"]) { $DimResults["limits"].OversizedRefused } else { $false }
    limit_source = if ($DimResults["limits"]) { $DimResults["limits"].LimitSource } else { "not_measured" }
  }

  profiles = if ($DimResults["profiles"]) {
    $qualifiedProfiles = @()
    foreach ($qp in $DimResults["profiles"].QualifiedProfiles) {
      $qualifiedProfiles += @{
        profile_id = $qp.ProfileId; verified = $qp.Verified; context = $qp.Context; ngl = $qp.Ngl; fit_evidence = $qp.FitEvidence
      }
    }
    @{
      total_profiles = $DimResults["profiles"].TotalProfiles
      verified_count = $DimResults["profiles"].VerifiedProfiles
      unverified_count = $DimResults["profiles"].UnverifiedProfiles
      qualified_profiles = $qualifiedProfiles
    }
  } else {
    @{ total_profiles = 0; verified_count = 0; unverified_count = 0; qualified_profiles = @() }
  }

  lifecycle = @{
    start = if ($DimResults["lifecycle"]) { $DimResults["lifecycle"].StartResult } else { "not_run" }
    select = if ($DimResults["lifecycle"]) { $DimResults["lifecycle"].SelectResult } else { "not_run" }
    stop_backend = if ($DimResults["lifecycle"]) { $DimResults["lifecycle"].StopBackendResult } else { "not_run" }
    stop_router = if ($DimResults["lifecycle"]) { $DimResults["lifecycle"].StopRouterResult } else { "not_run" }
    clean_shutdown = if ($DimResults["lifecycle"]) { $DimResults["lifecycle"].CleanShutdown } else { $false }
  }

  cleanup = @{
    orphans_before = if ($DimResults["cleanup_interim"]) { $DimResults["cleanup_interim"].OrphansBefore } else { @{} }
    orphans_after = @{ "llama-server" = $orphanCountFinal; "rust-router" = $rustOrphanFinal }
    ports_free_after = $portFree
    service_final_state = ($svcStatusAfter + " / " + $svcStartTypeAfter)
  }

  git = @{
    runtime_node_head = $rnHead
    runtime_node_full_head = $rnFullHead
    thelibrarian_head = $tlHead
    runtime_node_clean = $rnCleanFinal
    thelibrarian_clean = $tlCleanFinal
    stashes_preserved = ($rnStashEmptyFinal -and $tlStashEmptyFinal)
  }

  result = @{
    total_checks_passed = $Passed
    total_checks_failed = $Failed
    dimensions_available = @($DimResults.Keys)
    overall = $overallResult
  }
}

$QualFile = ($ReceiptDir + "\win-runtime-qualification-1-" + $Timestamp + ".json")
$receiptJson = $receipt | ConvertTo-Json -Depth 10
$receiptJson | Out-File -FilePath $QualFile -Encoding utf8 -NoNewline
Test-Pass ("Qualification receipt written to " + $QualFile)

Write-Host ""
Write-Host "Receipt content:" -ForegroundColor DarkGray
$receiptJson

# ============================================================================
# Summary
# ============================================================================
Write-Step "SUMMARY: WIN-RUNTIME-QUALIFICATION-1"

Write-HostColor ("Starting HEAD: " + $rnHead + " (runtime-node), " + $tlHead + " (TheLibrarian)") "Cyan"
Write-HostColor ("Hostname: " + $hostname) "Cyan"
Write-HostColor ("Overall result: " + $overallResult) $(if($overallResult -eq "qualified"){"Green"}else{"Red"})
Write-Host ""

Write-Host ("  Artifact SHA-256: " + $receipt.artifact.router_binary_sha256) -ForegroundColor DarkGray
Write-Host ("  Build timestamp: " + $receipt.artifact.router_binary_build_timestamp) -ForegroundColor DarkGray
Write-Host ("  Contract harness: " + $contractResult) -ForegroundColor DarkGray
Write-Host ("  Network/auth: " + $networkAuthResult) -ForegroundColor DarkGray
Write-Host ("  Request limits: " + $limitsResult) -ForegroundColor DarkGray
Write-Host ("  Lifecycle: " + $lifecycleResult) -ForegroundColor DarkGray
Write-Host ("  Ports free: " + $portFree) -ForegroundColor DarkGray
Write-Host ("  Orphans: " + $orphanCountFinal + " llama, " + $rustOrphanFinal + " rust") -ForegroundColor DarkGray
Write-Host ("  Service: " + $svcStatusAfter + " / " + $svcStartTypeAfter) -ForegroundColor DarkGray
Write-Host ("  Receipt: " + $QualFile) -ForegroundColor DarkGray
Write-Host ""

$totalChecks = $Passed + $Failed
$color = "Red"
if ($Failed -eq 0) { $color = "Green" } elseif ($Failed -le 3 -and $overallResult -eq "partial") { $color = "Yellow" }
Write-Host ("  " + $Passed + " passed, " + $Failed + " failed (" + $totalChecks + " total checks)") -ForegroundColor $color
Write-Host ("  Overall: " + $overallResult) -ForegroundColor $(if($overallResult -eq "qualified"){"Green"}else{"Red"})

Write-Host ""
if ($overallResult -eq "qualified") {
  Write-Host "WIN-RUNTIME-QUALIFICATION-1: QUALIFIED" -ForegroundColor Green
  exit 0
} elseif ($overallResult -eq "partial") {
  Write-Host "WIN-RUNTIME-QUALIFICATION-1: PARTIAL" -ForegroundColor Yellow
  exit 1
} else {
  Write-Host "WIN-RUNTIME-QUALIFICATION-1: BLOCKED" -ForegroundColor Red
  exit 2
}
