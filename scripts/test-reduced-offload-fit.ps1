# scripts/test-reduced-offload-fit.ps1
# =============================================================================
# Reduced-Offload Fit Test for RX 570 Runtime (REDUCED-OFFLOAD-FIT-1)
# =============================================================================
# Tests OOM profiles (llama-3.2, qwen3, gemma-3) at descending ngl values
# to find stable GPU offload configurations.
#
# IMPORTANT: The router reads model-profiles.json at startup only. To change
# ngl/context for a test, we MUST stop the router, update the config, then
# restart the router. Each test cell = stop-router + change-config + start
# + select-profile + health-wait + chat-test + cleanup.
#
# Usage:
#   .\scripts\test-reduced-offload-fit.ps1
#
# Config: Original config is saved to .bak and restored at the end.
# =============================================================================

$RouterUrl = "http://127.0.0.1:9130"
$ConfigPath = "G:\openwork\librarian-runtime-node\config\model-profiles.json"
$BackupFile = "$ConfigPath.reduced-offload-bak.json"
$ResultLog = "reduced-offload-fit-results.log"
$RouterStdout = "router-stdout.log"
$RouterStderr = "router-stderr.log"
$ProfilesToTest = @("llama-3.2", "qwen3", "gemma-3")
$NglLadder = @(80, 60, 40, 20, 0)
$HigherContexts = @(2048, 3072, 4096)
$BaseContext = 1024

# === Logging ===

function Log-Result {
    param($profile, $ngl, $context, $status, $detail)
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $line = "[$timestamp] $profile`tngl=$ngl`tctx=$context`t$status`t$detail"
    Write-Host "  LOG: $line" -ForegroundColor DarkGray
    Add-Content -Path $ResultLog -Value $line
}

function Write-ResultTable {
    param($title, $results)
    Write-Host ""
    Write-Host "=== $title ===" -ForegroundColor Cyan
    if ($results.Count -eq 0) {
        Write-Host "  (no results)" -ForegroundColor DarkGray
        return
    }
    Write-Host ("{0,-10} {1,-8} {2,-10} {3}" -f "Profile", "ngl", "Context", "Status")
    Write-Host ("{0,-10} {1,-8} {2,-10} {3}" -f ("-"*8), ("-"*6), ("-"*8), ("-"*25))
    foreach ($r in $results) {
        $color = "Yellow"
        if ($r.status -eq "PASS") { $color = "Green" }
        elseif ($r.status -eq "OOM" -or $r.status -eq "FAIL") { $color = "Red" }
        Write-Host ("{0,-10} {1,-8} {2,-10} {3}" -f $r.profile, $r.ngl, $r.context, $r.status) -ForegroundColor $color
    }
    Write-Host ""
}

# === Router lifecycle ===

function Start-Router {
    Write-Host "  Starting router..." -NoNewline
    $proc = Start-Process python -ArgumentList "-u router/router.py --port 9130" -PassThru `
        -RedirectStandardOutput $RouterStdout -RedirectStandardError $RouterStderr -NoNewWindow
    $ready = $false
    for ($i = 0; $i -lt 30; $i++) {
        try {
            $null = Invoke-RestMethod -Uri "$RouterUrl/backend/status" -Method Get -TimeoutSec 2
            $ready = $true
            break
        } catch { Start-Sleep -Seconds 1 }
    }
    if (-not $ready) {
        Write-Host " FAILED" -ForegroundColor Red
        return $null
    }
    Write-Host " PID=$($proc.Id)" -ForegroundColor Green
    return $proc
}

function Stop-Router {
    param($routerProc)
    if (-not $routerProc) { return }
    $rpid = $routerProc.Id
    $p = Get-Process -Id $rpid -ErrorAction SilentlyContinue
    if ($p) { Stop-Process -Id $rpid -Force -ErrorAction SilentlyContinue }
}

function Kill-AllBackends {
    Get-Process -Name "llama-server*" -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 1
}

# === Config ===

function Backup-Config {
    Copy-Item -Path $ConfigPath -Destination $BackupFile -Force
    Write-Host "  Config backed up to $BackupFile" -ForegroundColor DarkGray
}

function Restore-Config {
    if (Test-Path $BackupFile) {
        Copy-Item -Path $BackupFile -Destination $ConfigPath -Force
    }
}

function Set-ProfileConfig {
    param($profileAlias, $newNgl, $newContext)
    $json = Get-Content $ConfigPath -Raw | ConvertFrom-Json
    $profile = $json.profiles | Where-Object { $_.alias -eq $profileAlias }
    if (-not $profile) { return $false }
    $profile.ngl = $newNgl
    $profile.context = $newContext
    $profile.launch_command = $profile.launch_command -replace "-ngl \d+", "-ngl $newNgl"
    $profile.launch_command = $profile.launch_command -replace "-c \d+", "-c $newContext"
    $json | ConvertTo-Json -Depth 10 | Set-Content $ConfigPath
    return $true
}

# === Test cell ===

function Test-Cell {
    param($profileAlias, $ngl, $context)
    Write-Host ""
    Write-Host "  ----- Cell: $profileAlias  ngl=$ngl  context=$context -----" -ForegroundColor Cyan

    # Config change + restart router (only way to pick up new ngl/context)
    Write-Host "  Applying ngl=$ngl, context=$context to config..." -NoNewline
    if (-not (Set-ProfileConfig $profileAlias $ngl $context)) {
        Write-Host " FAIL (profile not found)" -ForegroundColor Red
        return @{ status = "SKIP"; detail = "Profile not found" }
    }
    Write-Host " done" -ForegroundColor Gray

    # Start router (picks up new config)
    $rp = Start-Router
    if (-not $rp) {
        return @{ status = "FAIL"; detail = "Router failed to start" }
    }

    # Select the profile (launches backend)
    try {
        $selectBody = @{ profile = $profileAlias } | ConvertTo-Json
        $selectResp = Invoke-RestMethod -Uri "$RouterUrl/backend/select" -Method Post `
            -Body $selectBody -ContentType "application/json" -TimeoutSec 10
    } catch {
        Stop-Router -routerProc $rp
        Kill-AllBackends
        return @{ status = "FAIL"; detail = "Select exception: $($_.Exception.Message)" }
    }

    # Wait for healthy state
    $healthy = $false
    $oomDetected = $false
    $failDetail = ""

    for ($i = 0; $i -lt 90; $i++) {
        try {
            $statusResp = Invoke-RestMethod -Uri "$RouterUrl/backend/status" -Method Get -TimeoutSec 3
            $state = $statusResp.profiles.$profileAlias.state
            if ($state -eq "healthy") {
                $healthy = $true
                break
            }
        } catch { }

        # Check backend log for OOM between polls
        $logTail = Get-Content "backend_$profileAlias.log" -Tail 5 -ErrorAction SilentlyContinue
        if ($logTail -match "out of memory|OOM|CUDA OOM|VK_ERROR|ErrorOutOfDeviceMemory|VK_ERROR_OUT_OF_DEVICE_MEMORY") {
            $oomDetected = $true
            $failDetail = "OOM detected in backend log"
            break
        }
        Start-Sleep -Seconds 2
    }

    if ($oomDetected) {
        Stop-Router -routerProc $rp
        Kill-AllBackends
        return @{ status = "OOM"; detail = $failDetail }
    }

    if (-not $healthy) {
        # Final OOM check
        $logTail = Get-Content "backend_$profileAlias.log" -Tail 15 -ErrorAction SilentlyContinue
        if ($logTail -match "out of memory|OOM|CUDA OOM|VK_ERROR|ErrorOutOfDeviceMemory") {
            Stop-Router -routerProc $rp
            Kill-AllBackends
            return @{ status = "OOM"; detail = "OOM detected post-timeout" }
        }
        Stop-Router -routerProc $rp
        Kill-AllBackends
        return @{ status = "FAIL"; detail = "Timed out waiting for healthy state" }
    }

    # Health OK. Test chat.
    Start-Sleep -Seconds 2
    try {
        $chatBody = @{
            profile   = $profileAlias
            messages  = @(@{ role = "user"; content = "Reply with OK only." })
        } | ConvertTo-Json
        $chatResp = Invoke-RestMethod -Uri "$RouterUrl/backend/chat" -Method Post `
            -Body $chatBody -ContentType "application/json" -TimeoutSec 120

        if ($chatResp.status -eq "ok") {
            $result = @{ status = "PASS"; detail = "Chat returned OK" }
        } else {
            $result = @{ status = "FAIL"; detail = "Chat status=$($chatResp.status)" }
        }
    } catch {
        $result = @{ status = "FAIL"; detail = "Chat exception: $($_.Exception.Message)" }
    }

    # Cleanup
    Stop-Router -routerProc $rp
    Kill-AllBackends
    return $result
}

# === Main ===

Write-Host ""
Write-Host ("=" * 70) -ForegroundColor Cyan
Write-Host "  REDUCED-OFFLOAD-FIT-1 - RX 570 Reduced GPU Offload Fit Test"
Write-Host "  Testing: $($ProfilesToTest -join ', ')"
Write-Host "  ngl ladder: $($NglLadder -join ', ')"
Write-Host "  Base context: $BaseContext"
Write-Host ("  Date: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')")
Write-Host ("=" * 70) -ForegroundColor Cyan

# Kill any leftover processes before starting
Kill-AllBackends

# Backup original config
Backup-Config

# Init result log
"Reduced-Offload Fit Test - $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" | Out-File -FilePath $ResultLog
"Profiles: $($ProfilesToTest -join ', ')" | Add-Content -Path $ResultLog
"ngl ladder: $($NglLadder -join ', ')" | Add-Content -Path $ResultLog
("=" * 60) | Add-Content -Path $ResultLog

$allResults = @{}
$higherCtxResults = @()

# === Phase 1: Find stable ngl at context 1024 ===
foreach ($profileAlias in $ProfilesToTest) {
    Write-Host ""
    Write-Host ("=" * 60) -ForegroundColor Cyan
    Write-Host "  Phase 1: $profileAlias at context $BaseContext" -ForegroundColor Cyan
    Write-Host ("=" * 60) -ForegroundColor Cyan

    $cells = @()
    $stableNgl = $null

    foreach ($ngl in $NglLadder) {
        Write-Host ""

        # Restore config back to original before each cell
        Restore-Config

        $r = Test-Cell $profileAlias $ngl $BaseContext
        $r.profile = $profileAlias
        $r.ngl = $ngl
        $r.context = $BaseContext
        $cells += $r

        $color = "Yellow"
        if ($r.status -eq "PASS") { $color = "Green"; $stableNgl = $ngl; break }
        elseif ($r.status -eq "OOM") { $color = "Red"; Write-Host "  (descending...)" -ForegroundColor DarkYellow; continue }
        elseif ($r.status -eq "FAIL") { $color = "Red"; Write-Host "  (stopping - non-OOM failure)" -ForegroundColor DarkYellow; break }

        Log-Result $profileAlias $ngl $BaseContext $r.status $r.detail
    }

    if ($stableNgl) {
        $summary = "Highest stable ngl at context $BaseContext is ngl=$stableNgl"
        Write-Host ""
        Write-Host "  *** $summary ***" -ForegroundColor Green
        Log-Result $profileAlias $stableNgl $BaseContext "STABLE" $summary
    } else {
        $summary = "NO STABLE ngl found at context $BaseContext"
        Write-Host ""
        Write-Host "  *** $summary ***" -ForegroundColor Red
        Log-Result $profileAlias "N/A" $BaseContext "UNSTABLE" $summary
    }

    $allResults[$profileAlias] = @{
        cells     = $cells
        stableNgl = $stableNgl
        summary   = $summary
    }

    Write-ResultTable -title "Results: $profileAlias at context $BaseContext" -results $cells
}

# === Phase 2: Test higher contexts at stable ngl ===
foreach ($profileAlias in $ProfilesToTest) {
    $info = $allResults[$profileAlias]
    $stableNgl = $info.stableNgl
    if (-not $stableNgl) {
        Write-Host "Skipping higher-context tests for $profileAlias (no stable ngl at 1024)." -ForegroundColor DarkYellow
        continue
    }

    Write-Host ""
    Write-Host ("=" * 60) -ForegroundColor Magenta
    Write-Host "  Phase 2: $profileAlias at ngl=$stableNgl, testing higher contexts" -ForegroundColor Magenta
    Write-Host ("=" * 60) -ForegroundColor Magenta

    foreach ($ctx in $HigherContexts) {
        Restore-Config

        $r = Test-Cell $profileAlias $stableNgl $ctx
        $r.profile = $profileAlias
        $r.ngl = $stableNgl
        $r.context = $ctx
        $higherCtxResults += $r

        $color = "Yellow"
        if ($r.status -eq "PASS") { $color = "Green" }
        elseif ($r.status -eq "OOM") { $color = "Red" }
        Write-Host "  Higher-context: ngl=$stableNgl ctx=$ctx -> $($r.status)" -ForegroundColor $color
        Log-Result $profileAlias $stableNgl $ctx $r.status $r.detail
    }
}

# === Restore config to original ===
Restore-Config
Write-Host ""
Write-Host "Config restored to original." -ForegroundColor DarkGray

# === Print results ===
Write-Host ""
Write-Host ("=" * 70) -ForegroundColor Cyan
Write-Host "  FINAL RESULTS" -ForegroundColor Cyan
Write-Host ("=" * 70) -ForegroundColor Cyan

Write-Host ""
Write-Host "--- Phase 1: ngl at context $BaseContext ---" -ForegroundColor Cyan
foreach ($profileAlias in $ProfilesToTest) {
    $info = $allResults[$profileAlias]
    $scolor = if ($info.stableNgl) { "Green" } else { "Red" }
    Write-Host ("  {0,-12} {1}" -f ($profileAlias + ":"), $info.summary) -ForegroundColor $scolor
}

if ($higherCtxResults.Count -gt 0) {
    Write-ResultTable -title "Phase 2: Higher-Context Results" -results $higherCtxResults
}

Write-Host ""
Write-Host "Full log: $ResultLog"
Write-Host ("=" * 70) -ForegroundColor Cyan

# Final cleanup: ensure no orphans
Kill-AllBackends
Write-Host "Orphan check: no llama-server processes remaining." -ForegroundColor Green
