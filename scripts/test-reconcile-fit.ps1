# scripts/test-reconcile-fit.ps1
# =============================================================================
# FIT-EVIDENCE-RECONCILE-1 — Reconcile phi-4 and qwen-coder evidence using
# the correct restart-per-config-change method.
#
# The router reads model-profiles.json at startup only. Config changes while
# the router is running have no effect on in-memory profile data. This script
# stops the router, changes config, restarts, tests, and cleans up for each
# test cell — matching the method in test-reduced-offload-fit.ps1.
# =============================================================================

$RouterUrl = "http://127.0.0.1:9130"
$ConfigPath = "G:\openwork\librarian-runtime-node\config\model-profiles.json"
$BackupFile = "$ConfigPath.reconcile-bak.json"
$ResultLog = "reconcile-fit-results.log"
$RouterStdout = "router-stdout-reconcile.log"
$RouterStderr = "router-stderr-reconcile.log"
$EvidenceDir = "G:\openwork\librarian-runtime-node\fixtures\windows-runtime-node\model-fit\evidence"

# === Helpers ===

function Log {
    param($msg)
    $ts = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    Write-Host "[$ts] $msg"
    Add-Content -Path $ResultLog -Value "[$ts] $msg"
}

function Ensure-CleanSlate {
    Get-Process -Name "llama-server*" -ErrorAction SilentlyContinue | Stop-Process -Force
    Get-Process -Name "python*" -ErrorAction SilentlyContinue | Where-Object { $_.CommandLine -match "router" } | Stop-Process -Force
    Start-Sleep -Seconds 2
}

function Backup-Config {
    Copy-Item -Path $ConfigPath -Destination $BackupFile -Force
    Log "Config backed up to $BackupFile"
}

function Restore-Config {
    if (Test-Path $BackupFile) {
        Copy-Item -Path $BackupFile -Destination $ConfigPath -Force
        Log "Config restored from backup."
    }
}

function Set-ProfileConfig {
    param($alias, $newNgl, $newContext)
    $json = Get-Content $ConfigPath -Raw | ConvertFrom-Json
    $profile = $json.profiles | Where-Object { $_.alias -eq $alias }
    if (-not $profile) { return $false }
    $profile.ngl = $newNgl
    $profile.context = $newContext
    $profile.launch_command = $profile.launch_command -replace "-ngl \d+", "-ngl $newNgl"
    $profile.launch_command = $profile.launch_command -replace "-c \d+", "-c $newContext"
    $json | ConvertTo-Json -Depth 10 | Set-Content $ConfigPath
    return $true
}

function Start-RouterAndWait {
    Log "  Starting router..."
    $proc = Start-Process python -ArgumentList "-u router/router.py --port 9130" -PassThru `
        -RedirectStandardOutput $RouterStdout -RedirectStandardError $RouterStderr -NoNewWindow
    for ($i = 0; $i -lt 30; $i++) {
        try {
            $null = Invoke-RestMethod -Uri "$RouterUrl/backend/status" -Method Get -TimeoutSec 2
            Log "  Router ready (PID=$($proc.Id))."
            return $proc
        } catch { Start-Sleep -Seconds 1 }
    }
    Log "  Router FAILED to start."
    return $null
}

function Stop-Router {
    param($proc)
    if (-not $proc) { return }
    $p = Get-Process -Id $proc.Id -ErrorAction SilentlyContinue
    if ($p) { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue; Log "  Router stopped." }
}

# === Test Cell ===

function Test-Cell {
    param($alias, $ngl, $context)
    Log "=== CELL: $alias ngl=$ngl context=$context ==="

    # 1. Clean slate
    Ensure-CleanSlate

    # 2. Apply config
    if (-not (Set-ProfileConfig $alias $ngl $context)) {
        Log "  FAIL: Profile $alias not found in config."
        return "SKIP"
    }
    Log "  Config set: $alias ngl=$ngl context=$context"

    # 3. Start router (picks up new config)
    $rp = Start-RouterAndWait
    if (-not $rp) { return "FAIL" }

    # 4. Select profile (launches backend)
    try {
        $body = @{ profile = $alias } | ConvertTo-Json
        $resp = Invoke-RestMethod -Uri "$RouterUrl/backend/select" -Method Post `
            -Body $body -ContentType "application/json" -TimeoutSec 10
        Log "  Select response: status=$($resp.status)"
    } catch {
        Log "  FAIL: Select exception: $($_.Exception.Message)"
        Stop-Router $rp; Ensure-CleanSlate
        return "FAIL"
    }

    # 5. Wait for healthy
    $healthy = $false
    for ($i = 0; $i -lt 60; $i++) {
        try {
            $statusResp = Invoke-RestMethod -Uri "$RouterUrl/backend/status" -Method Get -TimeoutSec 3
            $state = $statusResp.profiles.$alias.state
            if ($state -eq "healthy") {
                $healthy = $true
                Log "  Backend healthy after ~$($i*2)s."
                break
            }
            if ($state -eq "error") {
                Log "  Backend state=error. Possible OOM."
                break
            }
        } catch { }
        Start-Sleep -Seconds 2
    }

    if (-not $healthy) {
        Log "  FAIL: Backend did not reach healthy state."
        Stop-Router $rp; Ensure-CleanSlate
        return "FAIL"
    }

    # 6. Chat test
    Start-Sleep -Seconds 2
    try {
        $chatBody = @{
            profile  = $alias
            messages = @(@{ role = "user"; content = "Reply with OK only." })
        } | ConvertTo-Json
        $chatResp = Invoke-RestMethod -Uri "$RouterUrl/backend/chat" -Method Post `
            -Body $chatBody -ContentType "application/json" -TimeoutSec 120
        if ($chatResp.status -eq "ok") {
            Log "  CHAT PASS: response='$($chatResp.content)'"
            $result = "PASS"
        } else {
            Log "  CHAT FAIL: status=$($chatResp.status), content=$($chatResp.content)"
            $result = "FAIL"
        }
    } catch {
        Log "  CHAT FAIL: exception: $($_.Exception.Message)"
        $result = "FAIL"
    }

    # 7. Cleanup
    Stop-Router $rp
    Ensure-CleanSlate
    return $result
}

# === Main ===

Log "=============================================="
Log "FIT-EVIDENCE-RECONCILE-1"
Log "phi-4 and qwen-coder at ngl=99, context=4096"
Log "Method: restart-per-config-change (correct)"
Log "=============================================="

# Backup original config
Backup-Config

# Ensure clean start
Ensure-CleanSlate

# --- phi-4, ngl=99, context=4096 ---
$r1 = Test-Cell "phi-4" 99 4096
Log "phi-4 ngl=99 ctx=4096 => $r1"

# --- qwen-coder, ngl=99, context=4096 ---
$r2 = Test-Cell "qwen-coder" 99 4096
Log "qwen-coder ngl=99 ctx=4096 => $r2"

# Restore config
Restore-Config

# Final cleanup
Ensure-CleanSlate

Log "=============================================="
Log "RESULTS: phi-4 => $r1, qwen-coder => $r2"
Log "=============================================="

if ($r1 -eq "PASS" -and $r2 -eq "PASS") {
    Log "BOTH PASSED — evidence reconciled."
} else {
    Log "SOME FAILURES — review above."
}
