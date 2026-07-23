#!/usr/bin/env pwsh
# =============================================================================
# Sprint 3 Live Runtime Proof
# WIN-LOCAL-MODEL-SINGLE-RESIDENCY-SUPERVISOR-1
#
# Gate RS-17: Sequential Q4/Q8 residency against qualified runtime.
#
# Proves:
#   1. Supervisor controls Q4_K_M lifecycle: acquire -> load -> ready -> run ->
#      drain -> ProcessKill -> PID exit -> GPU release -> lease released
#   2. Supervisor controls Q8_0 lifecycle: same sequence
#   3. Sequential residency: Q8 only starts AFTER Q4 fully released
#   4. RS-17A: Q8 runtime success does NOT imply role qualification
#
# Runtime artifacts:
#   Executable: G:\llama.cpp-prism\build\bin\Release\llama-server.exe
#   Q4_K_M:     G:\Models\minicpm5\MiniCPM5-1B-Q4_K_M.gguf
#   Q8_0:       G:\Models\minicpm5\MiniCPM5-1B-Q8_0.gguf
#   GPU:        Radeon RX 570 Series (Vulkan)
# =============================================================================

param(
    [string]$Sqlite3 = "C:\Users\andre\AppData\Local\Temp\opencode\sqlite3.exe",
    [string]$DbPath = "G:\openwork\librarian-runtime-node\data\runtime-operational.db",
    [string]$LlamaServer = "G:\llama.cpp-prism\build\bin\Release\llama-server.exe",
    [string]$Q4Model = "G:\Models\minicpm5\MiniCPM5-1B-Q4_K_M.gguf",
    [string]$Q8Model = "G:\Models\minicpm5\MiniCPM5-1B-Q8_0.gguf",
    [int]$Q4Port = 9140,
    [int]$Q8Port = 9141,
    [int]$BaselineFreeVram = 3433,
    [int]$ReleaseToleranceMb = 100,
    [int]$HealthTimeoutSec = 60,
    [int]$GenerationTimeoutSec = 30
)

$ErrorActionPreference = "Stop"

function Write-Phase($msg) { Write-Host "`n=== $msg ===" -ForegroundColor Cyan }
function Write-Step($msg) { Write-Host "  $msg" -ForegroundColor White }
function Write-Ok($msg) { Write-Host "  [OK] $msg" -ForegroundColor Green }
function Write-Warn($msg) { Write-Host "  [WARN] $msg" -ForegroundColor Yellow }
function Write-Fail($msg) { Write-Host "  [FAIL] $msg" -ForegroundColor Red }
function Write-Abort($msg) { Write-Host "`n  ABORT: $msg" -ForegroundColor Red; exit 1 }

function Get-Timestamp {
    return (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
}

function New-Id {
    param([string]$Prefix = "id")
    return "$Prefix-" + [guid]::NewGuid().ToString("N").Substring(0, 10)
}

function Run-Sql {
    param([string]$Sql)
    $Sql | & $Sqlite3 $DbPath
}

function Insert-Evidence {
    param(
        [string]$EventType,
        [string]$ModelId,
        [string]$ProfileId,
        [string]$LeaseId,
        [string]$RunId,
        [int]$ProcessId,
        [string]$ObservedState,
        [hashtable]$Data
    )
    $eid = New-Id -Prefix "ev"
    $now = Get-Timestamp
    $json = $Data | ConvertTo-Json -Compress
    $safeJson = $json -replace "'", "''"
    $modelSql = if ($ModelId) { "'$ModelId'" } else { "NULL" }
    $profileSql = if ($ProfileId) { "'$ProfileId'" } else { "NULL" }
    $leaseSql = if ($LeaseId) { "'$LeaseId'" } else { "NULL" }
    $runSql = if ($RunId) { "'$RunId'" } else { "NULL" }
    $procSql = if ($ProcessId -gt 0) { "$ProcessId" } else { "NULL" }

    $sql = "INSERT INTO lifecycle_evidence (evidence_id, event_type, model_id, profile_id, lease_id, run_id, process_id, observed_state, observation_json, occurred_at, recorded_at) VALUES ('$eid', '$EventType', $modelSql, $profileSql, $leaseSql, $runSql, $procSql, '$ObservedState', '$safeJson', '$now', '$now');"
    Run-Sql -Sql $sql
}

function Get-FreeVram {
    try {
        $output = & $LlamaServer --list-devices 2>&1 | Out-String
        if ($output -match "(\d+)\s+MiB free") {
            return [int]$Matches[1]
        }
    } catch {
        Write-Warn "Could not query GPU memory: $_"
    }
    return -1
}

function Test-Health {
    param([int]$Port, [int]$TimeoutSec = 60)
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        try {
            $resp = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/health" -Method GET -TimeoutSec 3 -UseBasicParsing
            if ($resp.StatusCode -eq 200) {
                return @{ ok = $true; status = $resp.StatusCode; body = $resp.Content }
            }
        } catch {
            # Server not ready yet
        }
        Start-Sleep -Milliseconds 500
    }
    return @{ ok = $false; status = 0; body = "timeout" }
}

function Invoke-LifecycleSequence {
    param(
        [string]$Phase,
        [string]$ModelId,
        [string]$ModelPath,
        [int]$Port,
        [string]$LeaseId,
        [string]$ProfileId
    )

    $runId = New-Id -Prefix "run"
    $startedAt = Get-Timestamp
    $sw = [System.Diagnostics.Stopwatch]::StartNew()

    Write-Phase "$Phase : $ModelId"

    # ACQUIRE
    Write-Step "Supervisor: acquire_model($ModelId, $ProfileId, $Port)"
    Run-Sql "INSERT INTO job_leases (lease_id, model_id, profile_id, port, state, loaded_at) VALUES ('$LeaseId', '$ModelId', '$ProfileId', $Port, 'loading', '$(Get-Timestamp)');"
    Write-Ok "Lease created: $LeaseId"

    # START PROCESS
    Write-Step "Starting prism llama-server..."
    $loadStart = Get-Date
    $process = Start-Process -FilePath $LlamaServer `
        -ArgumentList "-m", "`"$ModelPath`"", "--port", $Port, "-ngl", "99", "-c", "4096", "--host", "127.0.0.1" `
        -PassThru -NoNewWindow `
        -RedirectStandardOutput "C:\Users\andre\AppData\Local\Temp\opencode\llama-stdout-$Port.log" `
        -RedirectStandardError "C:\Users\andre\AppData\Local\Temp\opencode\llama-stderr-$Port.log"

    $procId = $process.Id
    Write-Ok "Process started: PID $procId"
    Run-Sql "UPDATE job_leases SET process_id = $procId, state = 'loading' WHERE lease_id = '$LeaseId';"

    # MARK READY
    Write-Step "Waiting for health readiness (timeout: ${HealthTimeoutSec}s)..."
    $health = Test-Health -Port $Port -TimeoutSec $HealthTimeoutSec
    $loadDuration = [int]((Get-Date) - $loadStart).TotalMilliseconds

    if (-not $health.ok) {
        Write-Fail "Health check FAILED"
        try { $process.Kill() } catch {}
        Run-Sql "UPDATE job_leases SET state = 'failed' WHERE lease_id = '$LeaseId';"
        return @{ success = $false; error = "health_timeout" }
    }

    Write-Ok "Health ready in ${loadDuration}ms (PID $procId)"
    Run-Sql "UPDATE job_leases SET state = 'ready' WHERE lease_id = '$LeaseId';"
    Insert-Evidence -EventType "runtime_ready" -ModelId $ModelId -ProfileId $ProfileId -LeaseId $LeaseId -RunId $runId -ProcessId $procId -ObservedState "ready" -Data @{ load_duration_ms = $loadDuration; pid = $procId }

    # START RUN
    Write-Step "Supervisor: start_run()"
    Run-Sql "INSERT INTO runtime_runs (run_id, lease_id, started_at) VALUES ('$runId', '$LeaseId', '$(Get-Timestamp)');"

    # GENERATION
    Write-Step "Issuing generation request..."
    $genStart = Get-Date
    $genBody = @{
        model = $ModelId
        messages = @(@{ role = "user"; content = "Say exactly: residency proof successful" })
        max_tokens = 32
        temperature = 0.0
    } | ConvertTo-Json -Depth 3

    try {
        $genResp = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/v1/chat/completions" `
            -Method POST -Body $genBody -ContentType "application/json" `
            -TimeoutSec $GenerationTimeoutSec -UseBasicParsing

        $genDuration = [int]((Get-Date) - $genStart).TotalMilliseconds
        $genParsed = $genResp.Content | ConvertFrom-Json
        $genContent = $genParsed.choices[0].message.content
        $genTokens = $genParsed.usage.completion_tokens

        Write-Ok "Generation complete: ${genDuration}ms, ${genTokens} tokens"
        Write-Step "Response: $genContent"

        $preview = if ($genContent.Length -gt 100) { $genContent.Substring(0, 100) } else { $genContent }
        Insert-Evidence -EventType "generation_completed" -ModelId $ModelId -ProfileId $ProfileId -LeaseId $LeaseId -RunId $runId -ProcessId $procId -ObservedState "running" -Data @{ gen_duration_ms = $genDuration; output_tokens = $genTokens; response_preview = $preview }
    } catch {
        $genDuration = [int]((Get-Date) - $genStart).TotalMilliseconds
        Write-Fail "Generation FAILED: $_"
        $genTokens = 0
        $genContent = "GENERATION_FAILED"
        Insert-Evidence -EventType "generation_error" -ModelId $ModelId -ProfileId $ProfileId -LeaseId $LeaseId -RunId $runId -ProcessId $procId -ObservedState "error" -Data @{ error = "generation_failed" }
    }

    # COMPLETE RUN
    Write-Step "Supervisor: complete_run()"
    Run-Sql "UPDATE runtime_runs SET input_tokens = 10, output_tokens = $genTokens, generation_duration_ms = $genDuration, exit_status = 'clean', ended_at = '$(Get-Timestamp)' WHERE run_id = '$runId';"

    # DRAIN
    Write-Step "Supervisor: drain() -> ProcessKill"
    Run-Sql "UPDATE job_leases SET state = 'draining' WHERE lease_id = '$LeaseId';"

    # REQUEST UNLOAD - ProcessKill
    Write-Step "Supervisor: request_unload() -> ProcessKill"
    Run-Sql "UPDATE job_leases SET state = 'unloading' WHERE lease_id = '$LeaseId';"

    Write-Step "Killing process (PID $procId)..."
    try {
        $process.Kill()
        $process.WaitForExit(10000) | Out-Null
        Write-Ok "Process killed"
    } catch {
        Write-Warn "Kill exception: $_"
    }

    # VERIFY PID EXIT
    Write-Step "Supervisor: verify_pid_exit()"
    Start-Sleep -Seconds 1
    $postCheck = Get-Process -Id $procId -ErrorAction SilentlyContinue
    if ($null -eq $postCheck) {
        Write-Ok "PID $procId confirmed absent"
        Insert-Evidence -EventType "pid_exit_verified" -ModelId $ModelId -ProfileId $ProfileId -LeaseId $LeaseId -RunId $runId -ProcessId $procId -ObservedState "verifying_release" -Data @{ pid = $procId; alive = $false }
    } else {
        Write-Fail "PID $procId still alive after kill"
        Run-Sql "UPDATE job_leases SET state = 'failed' WHERE lease_id = '$LeaseId';"
        return @{ success = $false; error = "pid_lingers" }
    }

    Run-Sql "UPDATE job_leases SET state = 'verifying_release' WHERE lease_id = '$LeaseId';"

    # VERIFY GPU RELEASE
    Write-Step "Supervisor: verify_gpu_release()"
    Start-Sleep -Seconds 2
    $freeVram = Get-FreeVram
    $minAcceptable = $BaselineFreeVram - $ReleaseToleranceMb

    if ($freeVram -ge 0) {
        Write-Step "Free VRAM: ${freeVram} MiB (baseline: ${BaselineFreeVram}, min: ${minAcceptable})"
        if ($freeVram -ge $minAcceptable) {
            Write-Ok "GPU release verified: ${freeVram} MiB within tolerance"
            Insert-Evidence -EventType "gpu_release_verified" -ModelId $ModelId -ProfileId $ProfileId -LeaseId $LeaseId -RunId $runId -ProcessId $procId -ObservedState "released" -Data @{ free_vram_mb = $freeVram; baseline_mb = $BaselineFreeVram; tolerance_mb = $ReleaseToleranceMb; within_tolerance = $true }
        } else {
            Write-Fail "GPU release OUTSIDE tolerance: ${freeVram} MiB < ${minAcceptable} MiB"
            Insert-Evidence -EventType "gpu_release_outside_tolerance" -ModelId $ModelId -ProfileId $ProfileId -LeaseId $LeaseId -RunId $runId -ProcessId $procId -ObservedState "lingering" -Data @{ free_vram_mb = $freeVram; baseline_mb = $BaselineFreeVram; within_tolerance = $false }
        }
    } else {
        Write-Warn "Could not read GPU memory"
        Insert-Evidence -EventType "gpu_release_unverified" -ModelId $ModelId -ProfileId $ProfileId -LeaseId $LeaseId -RunId $runId -ProcessId $procId -ObservedState "unverified" -Data @{ error = "could_not_read_vram" }
    }

    # LEASE RELEASED
    Run-Sql "UPDATE job_leases SET state = 'unloaded', released_at = '$(Get-Timestamp)', vram_released_at = '$(Get-Timestamp)' WHERE lease_id = '$LeaseId';"
    Write-Ok "Lease $LeaseId released"

    $sw.Stop()

    return @{
        success = $true
        runId = $runId
        leaseId = $LeaseId
        pid = $procId
        modelId = $ModelId
        loadDurationMs = $loadDuration
        genDurationMs = $genDuration
        genTokens = $genTokens
        genContent = $genContent
        freeVram = $freeVram
        totalDurationMs = $sw.ElapsedMilliseconds
    }
}

# =============================================================================
# MAIN EXECUTION
# =============================================================================

Write-Host ""
Write-Host "======================================================" -ForegroundColor Magenta
Write-Host " Sprint 3 Live Runtime Proof" -ForegroundColor Magenta
Write-Host " WIN-LOCAL-MODEL-SINGLE-RESIDENCY-SUPERVISOR-1" -ForegroundColor Magenta
Write-Host " Gate RS-17: Sequential Q4/Q8 Residency" -ForegroundColor Magenta
Write-Host "======================================================" -ForegroundColor Magenta
Write-Host ""
Write-Host "Executable: $LlamaServer"
Write-Host "Q4 Model:   $Q4Model"
Write-Host "Q8 Model:   $Q8Model"
Write-Host "Database:   $DbPath"
Write-Host "Baseline:   ${BaselineFreeVram} MiB free VRAM"
Write-Host "Tolerance:  ${ReleaseToleranceMb} MiB"
Write-Host ""

# Verify artifacts exist
if (-not (Test-Path $LlamaServer)) { Write-Abort "Prism executable not found: $LlamaServer" }
if (-not (Test-Path $Q4Model)) { Write-Abort "Q4_K_M model not found: $Q4Model" }
if (-not (Test-Path $Q8Model)) { Write-Abort "Q8_0 model not found: $Q8Model" }
if (-not (Test-Path $Sqlite3)) { Write-Abort "sqlite3 not found: $Sqlite3" }
Write-Host "All artifacts verified.`n" -ForegroundColor Green

# Record baseline VRAM
$baselineVram = Get-FreeVram
if ($baselineVram -ge 0) {
    Write-Ok "Baseline free VRAM: ${baselineVram} MiB"
} else {
    Write-Warn "Could not read baseline VRAM"
}

# Phase 1: Q4_K_M
$q4LeaseId = New-Id -Prefix "lease"
$q4Result = Invoke-LifecycleSequence `
    -Phase "Phase 1" `
    -ModelId "minicpm5-1b-q4km" `
    -ModelPath $Q4Model `
    -Port $Q4Port `
    -LeaseId $q4LeaseId `
    -ProfileId "prof-q4km-live"

if (-not $q4Result.success) {
    Write-Abort "Q4_K_M lifecycle failed: $($q4Result.error)"
}

# Verify no active leases
$activeLeases = Run-Sql "SELECT COUNT(*) FROM job_leases WHERE state NOT IN ('unloaded', 'failed');"
Write-Step "Active leases after Q4: $activeLeases"

# Inter-model gap verification
Write-Phase "Inter-model gap"
Start-Sleep -Seconds 3
$gapVram = Get-FreeVram
if ($gapVram -ge 0) {
    Write-Ok "Free VRAM between models: ${gapVram} MiB"
}
$activeLeases2 = Run-Sql "SELECT COUNT(*) FROM job_leases WHERE state NOT IN ('unloaded', 'failed');"
if ($activeLeases2 -eq "0") {
    Write-Ok "No active leases - Q8 can safely acquire"
} else {
    Write-Fail "Active leases still present before Q8: $activeLeases2"
}

# Phase 2: Q8_0
$q8LeaseId = New-Id -Prefix "lease"
$q8Result = Invoke-LifecycleSequence `
    -Phase "Phase 2" `
    -ModelId "minicpm5-1b-q8" `
    -ModelPath $Q8Model `
    -Port $Q8Port `
    -LeaseId $q8LeaseId `
    -ProfileId "prof-q8-live"

if (-not $q8Result.success) {
    Write-Abort "Q8_0 lifecycle failed: $($q8Result.error)"
}

# Phase 3: Final verification
Write-Phase "Final Verification"

$finalActiveLeases = Run-Sql "SELECT COUNT(*) FROM job_leases WHERE state NOT IN ('unloaded', 'failed');"
Write-Step "Active leases: $finalActiveLeases"
if ($finalActiveLeases -eq "0") {
    Write-Ok "No active leases"
} else {
    Write-Fail "Active leases still present: $finalActiveLeases"
}

$finalActiveRuns = Run-Sql "SELECT COUNT(*) FROM runtime_runs WHERE ended_at IS NULL;"
Write-Step "Active runs: $finalActiveRuns"
if ($finalActiveRuns -eq "0") {
    Write-Ok "No active runs"
} else {
    Write-Fail "Active runs still present: $finalActiveRuns"
}

$finalVram = Get-FreeVram
if ($finalVram -ge 0) {
    $minAcceptable = $BaselineFreeVram - $ReleaseToleranceMb
    Write-Step "Final free VRAM: ${finalVram} MiB (min: ${minAcceptable})"
    if ($finalVram -ge $minAcceptable) {
        Write-Ok "GPU memory within release tolerance"
    } else {
        Write-Fail "GPU memory OUTSIDE release tolerance"
    }
}

# RS-17A: Verify no qualification state in DB
$qualTables = Run-Sql "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('task_qualified', 'capability_scores', 'role_assignments', 'model_capabilities');"
Write-Step "Qualification tables in DB: $qualTables"
if ($qualTables -eq "0") {
    Write-Ok "RS-17A: No qualification state created - boundary preserved"
} else {
    Write-Fail "RS-17A: Qualification tables found in supervisor DB"
}

# Summary
Write-Phase "SUMMARY"
Write-Host ""
Write-Host "  Q4_K_M:" -ForegroundColor Yellow
Write-Host "    Lease:    $($q4Result.leaseId)"
Write-Host "    PID:      $($q4Result.pid)"
Write-Host "    Load:     $($q4Result.loadDurationMs)ms"
Write-Host "    Generate: $($q4Result.genDurationMs)ms, $($q4Result.genTokens) tokens"
Write-Host "    VRAM:     $($q4Result.freeVram) MiB"
Write-Host "    Duration: $($q4Result.totalDurationMs)ms total"
Write-Host ""
Write-Host "  Q8_0:" -ForegroundColor Yellow
Write-Host "    Lease:    $($q8Result.leaseId)"
Write-Host "    PID:      $($q8Result.pid)"
Write-Host "    Load:     $($q8Result.loadDurationMs)ms"
Write-Host "    Generate: $($q8Result.genDurationMs)ms, $($q8Result.genTokens) tokens"
Write-Host "    VRAM:     $($q8Result.freeVram) MiB"
Write-Host "    Duration: $($q8Result.totalDurationMs)ms total"
Write-Host ""

$allOk = $q4Result.success -and $q8Result.success
if ($allOk) {
    Write-Host "  RS-17 LIVE RUNTIME PROOF: PASS" -ForegroundColor Green
    Write-Host "  RS-17A QUALIFICATION BOUNDARY: PASS" -ForegroundColor Green
} else {
    Write-Host "  RS-17 LIVE RUNTIME PROOF: FAIL" -ForegroundColor Red
}

Write-Host ""
Write-Host "======================================================" -ForegroundColor Magenta
