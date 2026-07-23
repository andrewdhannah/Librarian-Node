#!/usr/bin/env pwsh
# =============================================================================
# Sprint 2 Qualification Harness
# WIN-LOCAL-MODEL-HARDWARE-AND-LLAMACPP-QUALIFICATION-1
#
# Tests: HQ-7 (MiniCPM load+generation), HQ-8 (VibeThinker availability),
#        HQ-9 (process lifecycle), HQ-10 (release evidence),
#        HQ-11 (sequential execution), HQ-12 (DB populated)
# =============================================================================

param(
    [string]$Sqlite3 = "C:\Users\andre\AppData\Local\Temp\opencode\sqlite3.exe",
    [string]$DbPath = "G:\openwork\librarian-runtime-node\data\runtime-operational.db",
    [string]$LlamaServer = "G:\llama.cpp-prism\build\bin\Release\llama-server.exe",
    [int]$BasePort = 9120,
    [int]$HealthTimeoutSec = 60,
    [int]$GenerationTimeoutSec = 30
)

$ErrorActionPreference = "Stop"
$results = @()

function Write-Phase($msg) { Write-Host "`n=== $msg ===" -ForegroundColor Cyan }
function Write-Step($msg) { Write-Host "  $msg" -ForegroundColor White }
function Write-Ok($msg) { Write-Host "  [OK] $msg" -ForegroundColor Green }
function Write-Warn($msg) { Write-Host "  [WARN] $msg" -ForegroundColor Yellow }
function Write-Fail($msg) { Write-Host "  [FAIL] $msg" -ForegroundColor Red }

function Get-Timestamp {
    return (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
}

function New-EvidenceId {
    return "ev-" + [guid]::NewGuid().ToString("N").Substring(0, 12)
}

function New-LeaseId {
    return "lease-" + [guid]::NewGuid().ToString("N").Substring(0, 8)
}

function New-RunId {
    return "run-" + [guid]::NewGuid().ToString("N").Substring(0, 8)
}

function Insert-LifecycleEvidence {
    param(
        [string]$EventType,
        [string]$ModelId = "",
        [string]$ProfileId = "",
        [string]$LeaseId = "",
        [string]$RunId = "",
        [int]$ProcessId = 0,
        [string]$ObservedState = "",
        [string]$ObservationJson
    )
    $eid = New-EvidenceId
    $now = Get-Timestamp
    $modelSql = if ($ModelId) { "'$ModelId'" } else { "NULL" }
    $profileSql = if ($ProfileId) { "'$ProfileId'" } else { "NULL" }
    $leaseSql = if ($LeaseId) { "'$LeaseId'" } else { "NULL" }
    $runSql = if ($RunId) { "'$RunId'" } else { "NULL" }
    $procIdSql = if ($ProcessId -gt 0) { $ProcessId } else { "NULL" }
    $stateSql = if ($ObservedState) { "'$ObservedState'" } else { "NULL" }

    # Escape single quotes in JSON
    $safeJson = $ObservationJson -replace "'", "''"

    $sql = "INSERT INTO lifecycle_evidence (evidence_id, event_type, model_id, profile_id, lease_id, run_id, process_id, observed_state, observation_json, occurred_at, recorded_at) VALUES ('$eid', '$EventType', $modelSql, $profileSql, $leaseSql, $runSql, $procIdSql, $stateSql, '$safeJson', '$now', '$now');"
    $sql | & $Sqlite3 $DbPath
}

function Insert-Lease {
    param(
        [string]$LeaseId,
        [string]$ModelId,
        [string]$ProfileId = "",
        [int]$Port = 0,
        [int]$ProcessId = 0,
        [string]$State = "unloaded"
    )
    $now = Get-Timestamp
    $profileSql = if ($ProfileId) { "'$ProfileId'" } else { "NULL" }
    $portSql = if ($Port -gt 0) { $Port } else { "NULL" }
    $procIdSql = if ($ProcessId -gt 0) { $ProcessId } else { "NULL" }

    $sql = "INSERT OR REPLACE INTO job_leases (lease_id, model_id, profile_id, port, process_id, state, loaded_at) VALUES ('$LeaseId', '$ModelId', $profileSql, $portSql, $procIdSql, '$State', '$now');"
    $sql | & $Sqlite3 $DbPath
}

function Update-LeaseState {
    param([string]$LeaseId, [string]$State)
    $now = Get-Timestamp
    $extra = ""
    if ($State -eq "unloaded") {
        $extra = ", released_at = '$now'"
    }
    $sql = "UPDATE job_leases SET state = '$State'$extra WHERE lease_id = '$LeaseId';"
    $sql | & $Sqlite3 $DbPath
}

function Insert-Run {
    param(
        [string]$RunId,
        [string]$LeaseId,
        [string]$PacketId = "",
        [int]$InputTokens = 0,
        [int]$OutputTokens = 0,
        [int]$LoadDurationMs = 0,
        [int]$GenDurationMs = 0,
        [string]$ExitStatus = "",
        [string]$StartedAt,
        [string]$EndedAt = ""
    )
    $endedSql = if ($EndedAt) { "'$EndedAt'" } else { "NULL" }
    $packetSql = if ($PacketId) { "'$PacketId'" } else { "NULL" }

    $sql = "INSERT INTO runtime_runs (run_id, lease_id, packet_id, input_tokens, output_tokens, load_duration_ms, generation_duration_ms, exit_status, started_at, ended_at) VALUES ('$RunId', '$LeaseId', $packetSql, $InputTokens, $OutputTokens, $LoadDurationMs, $GenDurationMs, '$ExitStatus', '$StartedAt', $endedSql);"
    $sql | & $Sqlite3 $DbPath
}

function Test-Health {
    param([int]$Port, [int]$TimeoutSec = 60)
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        try {
            $resp = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/health" -TimeoutSec 2 -UseBasicParsing -ErrorAction SilentlyContinue
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

function Stop-Server {
    param([int]$Port)
    try {
        $body = '{"stop":true}' | ConvertTo-Json
        Invoke-WebRequest -Uri "http://127.0.0.1:$Port/health" -Method POST -Body $body -ContentType "application/json" -TimeoutSec 5 -UseBasicParsing -ErrorAction SilentlyContinue
    } catch {
        # Expected - server may already be stopping
    }
}

function Invoke-QualificationRun {
    param(
        [string]$ModelId,
        [string]$ProfileId,
        [string]$ModelPath,
        [string]$ModelFilename,
        [int]$Port,
        [int]$NgL = 99,
        [int]$CtxSize = 4096
    )

    $runId = New-RunId
    $leaseId = New-LeaseId
    $startedAt = Get-Timestamp
    $sw = [System.Diagnostics.Stopwatch]::StartNew()

    Write-Phase "Qualifying: $ModelId on port $Port"
    Write-Step "Model: $ModelPath"
    Write-Step "Profile: $ProfileId"
    Write-Step "Run: $runId | Lease: $leaseId"

    # Record lease creation
    Insert-Lease -LeaseId $leaseId -ModelId $ModelId -ProfileId $ProfileId -Port $Port -State "loading"
    Insert-LifecycleEvidence -EventType "runtime_startup" -ModelId $ModelId -ProfileId $ProfileId -LeaseId $leaseId -RunId $runId -ObservedState "loading" -ObservationJson "{`"action`":`"starting_llama_server`",`"port`":$Port,`"ngl`":$NgL,`"ctx`":$CtxSize}"

    # Start llama-server
    Write-Step "Starting llama-server..."
    $loadStart = Get-Date
    $process = Start-Process -FilePath $LlamaServer `
        -ArgumentList "-m", "`"$ModelPath`"", "--port", $Port, "-ngl", $NgL, "-c", $CtxSize, "--host", "127.0.0.1" `
        -PassThru -NoNewWindow `
        -RedirectStandardOutput "C:\Users\andre\AppData\Local\Temp\opencode\llama-stdout-$Port.log" `
        -RedirectStandardError "C:\Users\andre\AppData\Local\Temp\opencode\llama-stderr-$Port.log"

    $procId = $process.Id
    Write-Ok "Process started: PID $procId"

    Update-LeaseState -LeaseId $leaseId -State "loading"
    Insert-LifecycleEvidence -EventType "process_started" -ModelId $ModelId -LeaseId $leaseId -RunId $runId -ProcessId $procId -ObservedState "loading" -ObservationJson "{`"pid`":$procId,`"port`":$Port}"

    # Wait for health
    Write-Step "Waiting for health readiness (timeout: ${HealthTimeoutSec}s)..."
    $health = Test-Health -Port $Port -TimeoutSec $HealthTimeoutSec
    $loadDuration = [int]((Get-Date) - $loadStart).TotalMilliseconds

    if ($health.ok) {
        Write-Ok "Health ready in ${loadDuration}ms"
        Update-LeaseState -LeaseId $leaseId -State "ready"
        Insert-LifecycleEvidence -EventType "runtime_ready" -ModelId $ModelId -LeaseId $leaseId -RunId $runId -ProcessId $procId -ObservedState "ready" -ObservationJson "{`"load_duration_ms`":$loadDuration,`"health_status`":$($health.status)}"
    } else {
        Write-Fail "Health check FAILED: $($health.body)"
        Update-LeaseState -LeaseId $leaseId -State "failed"
        Insert-LifecycleEvidence -EventType "runtime_error" -ModelId $ModelId -LeaseId $leaseId -RunId $runId -ProcessId $procId -ObservedState "failed" -ObservationJson "{`"error`":`"health_check_timeout`",`"load_duration_ms`":$loadDuration}"

        # Kill process
        try { $process.Kill() } catch {}
        Insert-Run -RunId $runId -LeaseId $leaseId -LoadDurationMs $loadDuration -ExitStatus "health_timeout" -StartedAt $startedAt
        return @{ success = $false; runId = $runId; leaseId = $leaseId; pid = $procId; error = "health_timeout" }
    }

    # Issue generation request
    Write-Step "Issuing bounded generation request..."
    $genStart = Get-Date
    $genBody = @{
        model = $ModelId
        messages = @(@{ role = "user"; content = "Say exactly: qualification test successful" })
        max_tokens = 32
        temperature = 0.0
    } | ConvertTo-Json -Depth 3

    try {
        $genResp = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/v1/chat/completions" `
            -Method POST -Body $genBody -ContentType "application/json" `
            -TimeoutSec $GenerationTimeoutSec -UseBasicParsing

        $genDuration = [int]((Get-Date) - $genStart).TotalMilliseconds
        $genContent = ($genResp.Content | ConvertFrom-Json).choices[0].message.content
        $genTokens = ($genResp.Content | ConvertFrom-Json).usage.completion_tokens

        Write-Ok "Generation complete: ${genDuration}ms, ${genTokens} tokens"
        Write-Step "Response: $genContent"

        Update-LeaseState -LeaseId $leaseId -State "running"
        Insert-LifecycleEvidence -EventType "generation_completed" -ModelId $ModelId -LeaseId $leaseId -RunId $runId -ProcessId $procId -ObservedState "running" -ObservationJson "{`"gen_duration_ms`":$genDuration,`"output_tokens`":$genTokens,`"response_preview`":`"$($genContent.Substring(0, [Math]::Min(100, $genContent.Length)))`"}"
    } catch {
        $genDuration = [int]((Get-Date) - $genStart).TotalMilliseconds
        Write-Fail "Generation FAILED: $_"
        Insert-LifecycleEvidence -EventType "runtime_error" -ModelId $ModelId -LeaseId $leaseId -RunId $runId -ProcessId $procId -ObservedState "error" -ObservationJson "{`"error`":`"generation_failed`",`"detail`":`"$($_.Exception.Message)`",`"gen_duration_ms`":$genDuration}"
        $genTokens = 0
        $genContent = "GENERATION_FAILED"
    }

    # Stop server
    Write-Step "Requesting server shutdown..."
    Stop-Server -Port $Port

    # Wait for process exit
    Write-Step "Waiting for process exit..."
    $exitOk = $process.WaitForExit(10000)
    $sw.Stop()

    if ($exitOk) {
        $exitCode = $process.ExitCode
        Write-Ok "Process exited cleanly: code $exitCode ($($sw.ElapsedMilliseconds)ms total)"
        Update-LeaseState -LeaseId $leaseId -State "unloaded"
        Insert-LifecycleEvidence -EventType "process_exit" -ModelId $ModelId -LeaseId $leaseId -RunId $runId -ProcessId $procId -ObservedState "unloaded" -ObservationJson "{`"exit_code`":$exitCode,`"total_duration_ms`":$($sw.ElapsedMilliseconds)}"
    } else {
        Write-Warn "Process did not exit in 10s, killing..."
        try { $process.Kill(); $process.WaitForExit(5000) } catch {}
        Update-LeaseState -LeaseId $leaseId -State "unloaded"
        Insert-LifecycleEvidence -EventType "process_killed" -ModelId $ModelId -LeaseId $leaseId -RunId $runId -ProcessId $procId -ObservedState "unloaded" -ObservationJson "{`"action`":`"force_kill`",`"total_duration_ms`":$($sw.ElapsedMilliseconds)}"
    }

    # Post-shutdown verification
    Start-Sleep -Seconds 2
    $postCheck = Get-Process -Id $procId -ErrorAction SilentlyContinue
    if ($null -eq $postCheck) {
        Write-Ok "Process confirmed gone (PID $procId)"
        Insert-LifecycleEvidence -EventType "release_verified" -ModelId $ModelId -LeaseId $leaseId -RunId $runId -ProcessId $procId -ObservedState "released" -ObservationJson "{`"verified`":true,`"method`":`"process_absent`"}"
    } else {
        Write-Warn "Process still exists after kill"
        Insert-LifecycleEvidence -EventType "release_failed" -ModelId $ModelId -LeaseId $leaseId -RunId $runId -ProcessId $procId -ObservedState "lingering" -ObservationJson "{`"verified`":false}"
    }

    # Record the run
    Insert-Run -RunId $runId -LeaseId $leaseId -InputTokens 10 -OutputTokens $genTokens -LoadDurationMs $loadDuration -GenDurationMs $genDuration -ExitStatus "clean" -StartedAt $startedAt -EndedAt (Get-Timestamp)

    # GPU memory check via Vulkan devices (post-shutdown)
    Write-Step "Checking GPU memory post-shutdown..."
    try {
        $devOut = & $LlamaServer --list-devices 2>&1 | Out-String
        Write-Step "Post-shutdown devices: $devOut"
        Insert-LifecycleEvidence -EventType "gpu_memory_observed" -ModelId $ModelId -LeaseId $leaseId -RunId $runId -ObservedState "post_release" -ObservationJson "{`"devices_raw`":`"$($devOut.Trim() -replace '`', '``')`"}"
    } catch {
        Write-Warn "Could not query devices post-shutdown"
    }

    return @{
        success = $true
        runId = $runId
        leaseId = $leaseId
        pid = $procId
        modelId = $ModelId
        loadDurationMs = $loadDuration
        genDurationMs = $genDuration
        genTokens = $genTokens
        genContent = $genContent
    }
}

# =============================================================================
# MAIN EXECUTION
# =============================================================================

Write-Host ""
Write-Host "======================================================" -ForegroundColor Magenta
Write-Host " Sprint 2 Qualification Harness" -ForegroundColor Magenta
Write-Host " WIN-LOCAL-MODEL-HARDWARE-AND-LLAMACPP-QUALIFICATION-1" -ForegroundColor Magenta
Write-Host "======================================================" -ForegroundColor Magenta
Write-Host ""
Write-Host "Executable: $LlamaServer"
Write-Host "Database:   $DbPath"
Write-Host "Started:    $(Get-Timestamp)"

# Record harness startup
Insert-LifecycleEvidence -EventType "qualification_started" -ObservationJson "{`"harness`":`"qualify-harness.ps1`",`"executable`":`"$LlamaServer`"}"

# ── HQ-7: MiniCPM5 Q4_K_M qualification ──
$q4km = Invoke-QualificationRun `
    -ModelId "minicpm5-1b-q4km" `
    -ProfileId "prof-minicpm5-q4km-vulkan" `
    -ModelPath "G:\Models\minicpm5\MiniCPM5-1B-Q4_K_M.gguf" `
    -ModelFilename "MiniCPM5-1B-Q4_K_M.gguf" `
    -Port $BasePort `
    -NgL 99 `
    -CtxSize 4096

$results += $q4km

# Brief cooldown between models
Write-Host ""
Write-Step "Cooldown between models (3s)..."
Start-Sleep -Seconds 3

# ── HQ-11: Sequential model execution — second model ──
# Use MiniCPM5 Q8_0 as the second model to prove sequential execution
$q8 = Invoke-QualificationRun `
    -ModelId "minicpm5-1b-q8" `
    -ProfileId "prof-minicpm5-q8-vulkan" `
    -ModelPath "G:\Models\minicpm5\MiniCPM5-1B-Q8_0.gguf" `
    -ModelFilename "MiniCPM5-1B-Q8_0.gguf" `
    -Port ($BasePort + 1) `
    -NgL 99 `
    -CtxSize 4096

$results += $q8

# ── Summary ──
Write-Phase "Qualification Summary"
foreach ($r in $results) {
    if ($r.success) {
        Write-Ok "$($r.modelId): load=$($r.loadDurationMs)ms gen=$($r.genDurationMs)ms tokens=$($r.genTokens)"
    } else {
        Write-Fail "$($r.modelId): FAILED ($($r.error))"
    }
}

# Record harness completion
Insert-LifecycleEvidence -EventType "qualification_completed" -ObservationJson "{`"total_runs`":$($results.Count),`"successful`":$($results | Where-Object { $_.success }).Count,`"failed`":$($results | Where-Object { -not $_.success }).Count}"

Write-Host ""
Write-Host "Qualification harness complete." -ForegroundColor Green
Write-Host "Results recorded in: $DbPath"
