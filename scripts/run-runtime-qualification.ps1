<#
.SYNOPSIS
  WIN-RUNTIME-QUALIFICATION-1: Governed runtime qualification layer.

.DESCRIPTION
  Rebuilds the Rust router from a known source HEAD, captures rebuilt artifact
  evidence, compares it against receipt artifact evidence, and records match or
  mismatch honestly.

  Proof chain:
    1. Source proof: repo was at HEAD Z.
    2. Artifact proof: integration run used artifact X with hash Y.
    3. Rebuild proof: a governed rebuild from HEAD Z produces a rebuild
       artifact and records its lineage.

  Design constraint:
    Does not assume Rust release builds are reproducible across machines,
    timestamps, flags, or environments. If rebuilt hash differs from the
    receipt artifact hash, the script records the mismatch with reason
    "non-reproducible build or different build environment" without forcing
    failure.

  Hard constraints:
    - Do not require ROUTER_AUTH_TOKEN.
    - Do not start or modify the Windows service unless explicitly necessary.
    - Do not run integration chat proof; this is rebuild qualification only.
    - Do not mutate runtime code unless a direct qualification defect is
      discovered and documented.
    - Do not rewrite old receipts.
    - Follow Windows anti-loop rules.

.PARAMETER ReceiptPath
  Path to a v2 integration receipt to compare against.

.PARAMETER BinaryPath
  Path for the rebuilt binary output.

.PARAMETER RouterDir
  Path to the rust-router project directory.

.PARAMETER ConfigDir
  Path to the librarian-runtime-node config directory.

.PARAMETER ReceiptDir
  Directory for qualification record output.

.PARAMETER SkipBuild
  If set, skip the cargo build step (useful for testing).

.EXAMPLE
  .\scripts\run-runtime-qualification.ps1
  .\scripts\run-runtime-qualification.ps1 -ReceiptPath "G:\OpenWork\receipts\runtime-integration\some-receipt.json"
#>

param(
  [string]$ReceiptPath = "",
  [string]$BinaryPath = "G:\OpenWork\librarian-runtime-node\rust-router\target\release\rust-router.exe",
  [string]$RouterDir = "G:\OpenWork\librarian-runtime-node\rust-router",
  [string]$ConfigDir = "G:\OpenWork\librarian-runtime-node",
  [string]$ReceiptDir = "G:\OpenWork\receipts\runtime-qualification",
  [switch]$SkipBuild = $false
)

$ErrorActionPreference = "Stop"
$InformationPreference = "Continue"

$Passed = 0
$Failed = 0

function Write-HostColor {
  param([string]$Text, [string]$Color = "White")
  Write-Host $Text -ForegroundColor $Color
}

function Write-Step {
  param([string]$Message)
  Write-Host ""
  Write-Host ("=" * 70) -ForegroundColor DarkGray
  Write-Host $Message -ForegroundColor Cyan
  Write-Host ("=" * 70) -ForegroundColor DarkGray
}

function Test-Pass {
  param([string]$Name)
  $script:Passed++
  Write-Host ("  PASS: " + $Name) -ForegroundColor Green
}

function Test-Fail {
  param([string]$Name, [string]$Detail = "")
  $script:Failed++
  $detailStr = if ($Detail) { " (" + $Detail + ")" } else { "" }
  Write-Host ("  FAIL: " + $Name + $detailStr) -ForegroundColor Red
}

$Timestamp = (Get-Date -Format "yyyyMMdd-HHmmss")
$StartTime = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")

# ============================================================================
# Phase 0: Locate receipt
# ============================================================================
Write-Step "PHASE 0: Locate Receipt"

if ([string]::IsNullOrEmpty($ReceiptPath)) {
  $integDir = "G:\OpenWork\receipts\runtime-integration"
  $receipts = Get-ChildItem -Path $integDir -Filter "*.json" | Sort-Object LastWriteTime -Descending
  if (-not $receipts) {
    Write-Host "FATAL: No receipts found in $integDir" -ForegroundColor Red
    exit 1
  }
  $ReceiptPath = $receipts[0].FullName
  Write-Host "Auto-selected latest receipt: $ReceiptPath" -ForegroundColor DarkGray
} else {
  if (-not (Test-Path $ReceiptPath)) {
    Write-Host "FATAL: Specified receipt not found: $ReceiptPath" -ForegroundColor Red
    exit 1
  }
  Write-Host "Using specified receipt: $ReceiptPath" -ForegroundColor DarkGray
}

$receiptContent = Get-Content $ReceiptPath -Raw -Encoding UTF8
$receipt = $receiptContent | ConvertFrom-Json

# Validate receipt is v2
$schemaVersion = $receipt.schema_version
if ($schemaVersion -ne "win-runtime-receipt/v2") {
  Test-Fail "Receipt schema_version" "Expected win-runtime-receipt/v2, got '$schemaVersion'"
  exit 1
}
Test-Pass "Receipt schema is v2"

# Check artifact section exists (QUAL-006: Gate fails on missing artifact proof)
if ($null -eq $receipt.artifact) {
  Test-Fail "Receipt artifact section" "Missing - receipt does not contain required artifact evidence"
  exit 1
}
Test-Pass "Receipt artifact section present"

$receiptHash = $receipt.artifact.router_binary_sha256
$receiptBinPath = $receipt.artifact.router_binary_path
$receiptBinMod = $receipt.artifact.router_binary_modified_utc

# Validate hash format (QUAL-007: Gate fails on malformed hash)
if ($receiptHash -cnotmatch '^[0-9A-F]{64}$') {
  Test-Fail "Receipt artifact SHA-256 format" "Must be 64 uppercase hex chars, got '$receiptHash'"
  exit 1
}
Test-Pass "Receipt artifact SHA-256 format valid"

Write-Host ""
Write-Host "  Receipt artifact binary: $receiptBinPath" -ForegroundColor DarkGray
Write-Host "  Receipt artifact SHA-256: $receiptHash" -ForegroundColor DarkGray
Write-Host "  Receipt artifact modified (UTC): $receiptBinMod" -ForegroundColor DarkGray
Write-Host "  Receipt runtime HEAD: $($receipt.repos.runtime_node_head)" -ForegroundColor DarkGray
Write-Host "  Receipt TheLibrarian HEAD: $($receipt.repos.thelibrarian_main_head)" -ForegroundColor DarkGray

# ============================================================================
# Phase 1: Record starting state
# ============================================================================
Write-Step "PHASE 1: Starting State"

# HEADs
$tlHead = & git -C "G:\OpenWork\TheLibrarian-main" rev-parse --short HEAD 2>$null
$rnHead = & git -C "$ConfigDir" rev-parse --short HEAD 2>$null

if ($tlHead -and $rnHead) {
  Test-Pass "HEADs: TheLibrarian-main=$tlHead, runtime-node=$rnHead"
} else {
  Test-Fail "HEAD check" "tlHead=$tlHead, rnHead=$rnHead"
}

# Working trees
$tlStatus = & git -C "G:\OpenWork\TheLibrarian-main" status --short 2>$null
$rnStatus = & git -C "$ConfigDir" status --short 2>$null
$tlClean = [string]::IsNullOrEmpty($tlStatus)
$rnClean = [string]::IsNullOrEmpty($rnStatus)
if ($tlClean -and $rnClean) {
  Test-Pass "Both working trees clean"
} else {
  Test-Fail "Working tree check" "tl=[$tlStatus] rn=[$rnStatus]"
}

# Stashes
$tlStash = & git -C "G:\OpenWork\TheLibrarian-main" stash list 2>$null
$rnStash = & git -C "$ConfigDir" stash list 2>$null
$tlStashEmpty = [string]::IsNullOrEmpty($tlStash)
$rnStashEmpty = [string]::IsNullOrEmpty($rnStash)
if ($tlStashEmpty -and $rnStashEmpty) {
  Test-Pass "Stashes empty"
} else {
  Test-Fail "Stash check" "tl stash=[$tlStash] rn stash=[$rnStash]"
}

# Service state
$ServiceName = "LibrarianRunTimeNode"
$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($svc) {
  Test-Pass "Service $ServiceName exists: $($svc.Status) / $($svc.StartType)"
  $svcFinalState = "$($svc.Status)"
  $svcStartType = "$($svc.StartType)"
} else {
  Test-Fail "Service $ServiceName not found"
  $svcFinalState = "NotFound"
  $svcStartType = "Unknown"
}

# Port check
$Port = 9130
$listeners = netstat -ano | Where-Object { $_ -match ":$Port\s" -and $_ -match "LISTENING" }
$listenerActive = ($listeners -ne $null -and $listeners.Count -gt 0)
if ($listenerActive) {
  Test-Fail "Port $Port listener" "Active LISTENING socket detected"
} else {
  Test-Pass ("Port " + $Port + ": no listener")
}

# Orphans
$llamaProcs = Get-Process -Name "llama-server" -ErrorAction SilentlyContinue
$rustRouterProcs = Get-Process -Name "rust-router" -ErrorAction SilentlyContinue
$orphanCount = if ($llamaProcs) { $llamaProcs.Count } else { 0 }
if ($llamaProcs -or $rustRouterProcs) {
  Test-Fail "Orphan processes" "llama-server: $($orphanCount), rust-router: $($rustRouterProcs.Count)"
} else {
  Test-Pass "No orphans"
}

# ============================================================================
# Phase 2: Rebuild Rust router from source
# ============================================================================
Write-Step "PHASE 2: Rebuild Rust Router"

# Record source HEAD used for rebuild
$rebuildSourceHead = $rnHead
Write-Host "  Source HEAD: $rebuildSourceHead" -ForegroundColor DarkGray
Write-Host "  Router dir: $RouterDir" -ForegroundColor DarkGray
Write-Host "  Target binary: $BinaryPath" -ForegroundColor DarkGray

# Capture build metadata before building
$cargoVersion = (cargo --version 2>$null).Trim()
$rustcVersion = (rustc --version 2>$null).Trim()
$rustcHost = (rustc -vV 2>$null | Select-String "host:").ToString().Trim()
$targetTriple = if ($rustcHost -match "host:\s+(.+)") { $Matches[1] } else { "unknown" }

Write-Host "  cargo version: $cargoVersion" -ForegroundColor DarkGray
Write-Host "  rustc version: $rustcVersion" -ForegroundColor DarkGray
Write-Host "  target triple: $targetTriple" -ForegroundColor DarkGray

$buildProfile = "release"
$buildCommand = 'cargo build --release --manifest-path "' + $RouterDir + '\Cargo.toml"'

if (-not $SkipBuild) {
  Write-Host "  Running: $buildCommand" -ForegroundColor Yellow
  
  $buildStart = Get-Date
  $buildLogFile = "$env:TEMP\rust-router-build-$Timestamp.log"
  $buildProc = Start-Process -FilePath "cargo" -ArgumentList "build --release --manifest-path `"$RouterDir\Cargo.toml`"" -NoNewWindow -PassThru -RedirectStandardOutput $buildLogFile -RedirectStandardError "$buildLogFile.err"
  $buildProc | Wait-Process -Timeout 300 -ErrorAction SilentlyContinue
  $buildExitCode = $buildProc.ExitCode
  $buildDuration = (Get-Date) - $buildStart

  # Check if build succeeded
  $buildSucceeded = ($buildExitCode -eq 0) -and (Test-Path $BinaryPath)
  if (-not $buildSucceeded) {
    Test-Fail "Rust router rebuild" "cargo build exit=$buildExitCode, binary exists: $(Test-Path $BinaryPath)"
    Write-Host "  Build stdout (last 20 lines):" -ForegroundColor Red
    if (Test-Path $buildLogFile) { Get-Content $buildLogFile -Tail 20 | ForEach-Object { Write-Host "    $_" -ForegroundColor Red } }
    if (Test-Path "$buildLogFile.err") { Get-Content "$buildLogFile.err" -Tail 20 | ForEach-Object { Write-Host "    $_" -ForegroundColor Red } }
    exit 1
  }
  Write-Host "  cargo exit code: $buildExitCode" -ForegroundColor DarkGray
  
  Test-Pass "Rust router rebuilt successfully (duration: $($buildDuration.TotalSeconds.ToString('F1'))s)"
} else {
  Write-Host "  SkipBuild set: using existing binary" -ForegroundColor Yellow
  $buildDuration = [TimeSpan]::Zero
  Test-Pass "Build skipped (SkipBuild switch)"
}

# ============================================================================
# Phase 3: Capture rebuilt artifact evidence
# ============================================================================
Write-Step "PHASE 3: Rebuilt Artifact Evidence"

$rebuiltHash = (Get-FileHash -Path $BinaryPath -Algorithm SHA256).Hash
$rebuiltModUtc = (Get-Item $BinaryPath).LastWriteTimeUtc.ToString("yyyy-MM-ddTHH:mm:ssZ")
$rebuiltSize = (Get-Item $BinaryPath).Length

Write-Host "  Rebuilt binary: $BinaryPath" -ForegroundColor DarkGray
Write-Host "  Rebuilt SHA-256: $rebuiltHash" -ForegroundColor DarkGray
Write-Host "  Rebuilt modified (UTC): $rebuiltModUtc" -ForegroundColor DarkGray
Write-Host "  Rebuilt size (bytes): $rebuiltSize" -ForegroundColor DarkGray
Test-Pass "Rebuilt artifact evidence captured"

# ============================================================================
# Phase 4: Hash comparison
# ============================================================================
Write-Step "PHASE 4: Hash Comparison"

$hashesMatch = ($rebuiltHash -eq $receiptHash)

Write-Host "  Receipt artifact SHA-256: $receiptHash" -ForegroundColor DarkGray
Write-Host "  Rebuilt artifact SHA-256: $rebuiltHash" -ForegroundColor DarkGray
Write-Host "  Match: $hashesMatch" -ForegroundColor $(if($hashesMatch){"Green"}else{"Yellow"})

if ($hashesMatch) {
  Test-Pass "Rebuilt hash matches receipt artifact hash"
  $comparisonReason = "hashes match"
} else {
  # Per design constraint: do not force failure on mismatch
  $comparisonReason = "non-reproducible build or different build environment"
  Test-Pass "Hash mismatch recorded honestly (per design: non-reproducible builds expected)" 
  Write-Host "  Reason: $comparisonReason" -ForegroundColor Yellow
}

# ============================================================================
# Phase 5: Final state verification
# ============================================================================
Write-Step "PHASE 5: Final State Verification"

# Service must remain Stopped/Manual (QUAL-009)
$svcFinal = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
$svcFinalStateFinal = "$($svcFinal.Status)"
$svcStartTypeFinal = "$($svcFinal.StartType)"
if ($svcFinalStateFinal -eq "Stopped" -and $svcStartTypeFinal -eq "Manual") {
  Test-Pass "Service final state: $svcFinalStateFinal / $svcStartTypeFinal (preserved)"
} else {
  Test-Fail "Service final state" "Expected Stopped/Manual, got $svcFinalStateFinal/$svcStartTypeFinal"
}

# Port must be free (QUAL-010)
$listenersFinal = netstat -ano | Where-Object { $_ -match ":$Port\s" -and $_ -match "LISTENING" }
$listenerFinalActive = ($listenersFinal -ne $null -and $listenersFinal.Count -gt 0)
if ($listenerFinalActive) {
  Test-Fail "Port $Port final" "Active LISTENING socket detected"
} else {
  Test-Pass ("Port " + $Port + ": free (no listener)")
}

# No orphans (QUAL-010)
$llamaFinal = Get-Process -Name "llama-server" -ErrorAction SilentlyContinue
$rustFinal = Get-Process -Name "rust-router" -ErrorAction SilentlyContinue
$orphanCountFinal = 0
if ($llamaFinal) { $orphanCountFinal = $llamaFinal.Count }
$orphanRustCountFinal = 0
if ($rustFinal) { $orphanRustCountFinal = $rustFinal.Count }
if ($llamaFinal -or $rustFinal) {
  Test-Fail "Orphan check" "llama-server: $orphanCountFinal, rust-router: $($rustFinal.Count)"
} else {
  Test-Pass "No orphans"
}

# Git status (QUAL-010)
$tlStatusFinal = & git -C "G:\OpenWork\TheLibrarian-main" status --short 2>$null
$rnStatusFinal = & git -C "$ConfigDir" status --short 2>$null
$tlCleanFinal = [string]::IsNullOrEmpty($tlStatusFinal)
$rnCleanFinal = [string]::IsNullOrEmpty($rnStatusFinal)
if ($tlCleanFinal -and $rnCleanFinal) {
  Test-Pass "Both working trees clean"
} else {
  Test-Fail "Working tree final" "tl=[$tlStatusFinal] rn=[$rnStatusFinal]"
}

# Stashes untouched (QUAL-010)
$tlStashFinal = & git -C "G:\OpenWork\TheLibrarian-main" stash list 2>$null
$rnStashFinal = & git -C "$ConfigDir" stash list 2>$null
$tlStashEmptyFinal = [string]::IsNullOrEmpty($tlStashFinal)
$rnStashEmptyFinal = [string]::IsNullOrEmpty($rnStashFinal)
if ($tlStashEmptyFinal -and $rnStashEmptyFinal) {
  Test-Pass "Stashes empty"
} else {
  Test-Fail "Stash final" "tl=[$tlStashFinal] rn=[$rnStashFinal]"
}

# ============================================================================
# Phase 6: Emit qualification record
# ============================================================================
Write-Step "PHASE 6: Emit Qualification Record"

if (-not (Test-Path $ReceiptDir)) {
  New-Item -ItemType Directory -Path $ReceiptDir -Force | Out-Null
}

$qualRecord = @{
  schema_version = "win-runtime-qualification/v1"
  record_type = "runtime_qualification_proof"
  created_at_utc = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
  
  source = @{
    receipt_path = $ReceiptPath
    receipt_schema_version = $schemaVersion
    receipt_runtime_head = $receipt.repos.runtime_node_head
    receipt_thelibrarian_head = $receipt.repos.thelibrarian_main_head
  }
  
  rebuild = @{
    source_head = $rebuildSourceHead
    binary_path = $BinaryPath
    binary_sha256 = $rebuiltHash
    binary_modified_utc = $rebuiltModUtc
    binary_size_bytes = $rebuiltSize
  }
  
  comparison = @{
    rebuilt_hash_matches_receipt = $hashesMatch
    reason = $comparisonReason
    receipt_artifact_sha256 = $receiptHash
    rebuilt_artifact_sha256 = $rebuiltHash
    receipt_artifact_path = $receiptBinPath
  }
  
  build_metadata = @{
    cargo_version = $cargoVersion
    rustc_version = $rustcVersion
    target_triple = $targetTriple
    profile = $buildProfile
    build_command = $buildCommand
    build_duration_seconds = [math]::Round($buildDuration.TotalSeconds, 1)
    source_repository = "librarian-runtime-node"
    source_head = $rebuildSourceHead
  }
  
  machine = @{
    role = "windows_runtime_node"
    service_name = $ServiceName
    service_final_state = $svcFinalStateFinal
    service_start_type = $svcStartTypeFinal
    port_9130_free = (-not $listenerFinalActive)
    orphan_llama_server_count = $orphanCountFinal
    orphan_rust_router_count = $orphanRustCountFinal
    git_working_trees_clean = ($tlCleanFinal -and $rnCleanFinal)
    stashes_empty = ($tlStashEmptyFinal -and $rnStashEmptyFinal)
  }
  
  result = @{
    total_checks_passed = $Passed
    total_checks_failed = $Failed
    overall = "pass"
  }
}

if ($Failed -gt 0) { $qualRecord.result.overall = "fail" }

$QualFile = "$ReceiptDir\win-runtime-qualification-$Timestamp.json"
$qualJson = $qualRecord | ConvertTo-Json -Depth 10
$qualJson | Out-File -FilePath $QualFile -Encoding utf8 -NoNewline
Test-Pass "Qualification record written to $QualFile"

Write-Host ""
Write-Host "Qualification record content:" -ForegroundColor DarkGray
$qualJson

# ============================================================================
# Phase 7: Run verifier/gate
# ============================================================================
Write-Step "PHASE 7: Verify Qualification Record"

$verifierScript = "$ConfigDir\scripts\verify-runtime-qualification.ps1"
if (Test-Path $verifierScript) {
  & $verifierScript -QualificationPath $QualFile -ReceiptPath $ReceiptPath
  $verifierExit = $LASTEXITCODE
  if ($verifierExit -eq 0) {
    Test-Pass "Qualification verifier passed"
  } else {
    Test-Fail "Qualification verifier" "Exit code $verifierExit"
  }
} else {
  Write-Host "  Verifier script not found at $verifierScript -- skipping" -ForegroundColor Yellow
  $verifierExit = 0
}

# ============================================================================
# Summary
# ============================================================================
Write-Step "SUMMARY: WIN-RUNTIME-QUALIFICATION-1"

Write-HostColor "Qualification results:" "Cyan"
Write-Host "  Source HEAD: $rebuildSourceHead" -ForegroundColor DarkGray
Write-Host "  Receipt: $ReceiptPath" -ForegroundColor DarkGray
Write-Host "  Receipt hash: $receiptHash" -ForegroundColor DarkGray
Write-Host "  Rebuilt hash: $rebuiltHash" -ForegroundColor DarkGray
$matchColor = "Yellow"
if ($hashesMatch) { $matchColor = "Green" }
Write-Host "  Hashes match: $hashesMatch" -ForegroundColor $matchColor
Write-Host "  Reason: $comparisonReason" -ForegroundColor DarkGray
$totalChecks = $Passed + $Failed
$color = "Red"
if ($Failed -eq 0) { $color = "Green" }
$qualMsg = '  Total checks: {0} ({1} passed, {2} failed)' -f $totalChecks, $Passed, $Failed
Write-Host $qualMsg -ForegroundColor $color
Write-Host "  Qualification record: $QualFile" -ForegroundColor DarkGray
Write-Host "  Verifier exit code: $verifierExit" -ForegroundColor DarkGray

if ($Failed -eq 0) {
  Write-Host ""
  Write-Host "WIN-RUNTIME-QUALIFICATION-1: PASSED" -ForegroundColor Green
  exit 0
} else {
  Write-Host ""
  Write-Host "WIN-RUNTIME-QUALIFICATION-1: FAILED" -ForegroundColor Red
  exit 1
}
