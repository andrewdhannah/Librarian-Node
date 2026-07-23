# scripts/test-context-fit.ps1
# =============================================================================
# test-context-fit.ps1 — Context Limit Verifier for RX 570 Runtime
# =============================================================================
#
# ⚠ DEPRECATED ⚠
# This script modifies config/model-profiles.json while the router is running.
# The router reads the config at startup only and caches profiles in memory.
# Changing the config file at runtime has NO EFFECT on the in-memory profile
# data. Backend restarts triggered by this script use STALE profile values.
#
# Use scripts/test-reduced-offload-fit.ps1 or scripts/test-reconcile-fit.ps1
# instead — both correctly stop the router, change config, and restart the
# router to pick up new values.
#
# Kept for reference only. Do not use for producing evidence.
# =============================================================================

$RouterUrl = "http://127.0.0.1:9130"
$ConfigPath = "G:\openwork\librarian-runtime-node\config\model-profiles.json"
$BackupPath = "$ConfigPath.bak"
$Profiles = @("phi-4", "qwen-coder", "llama-3.2", "qwen3", "gemma-3")
$Contexts = @(1024, 2048, 3072, 4096)

function Log-Result($profile, $context, $status, $detail) {
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    "[$timestamp] Profile: $profile, Context: $context, Status: $status, Detail: $detail" | Out-File -FilePath "context-fit-results.log" -Append
}

# 1. Backup config
Copy-Item -Path $ConfigPath -Destination $BackupPath -Force
Write-Host "Config backed up to $BackupPath"

try {
    # 2. Start Router Manually (with logging)
    Write-Host "Starting Router manually..."
    $routerProc = Start-Process python -ArgumentList "router/router.py --port 9130" -PassThru -RedirectStandardOutput "router-stdout.log" -RedirectStandardError "router-stderr.log" -WindowStyle Hidden
    
    # Wait for router to be listening on 9130
    $routerReady = $false
    for ($i=0; $i -lt 15; $i++) {
        try {
            $resp = Invoke-RestMethod -Uri "$RouterUrl/backend/status" -Method Get -TimeoutSec 2
            $routerReady = $true
            break
        } catch {
            Start-Sleep -Seconds 1
        }
    }

    if (-not $routerReady) {
        Write-Host "Router failed to start on port 9130. Check router-debug.log" -ForegroundColor Red
        throw "Router not ready"
    }
    Write-Host "Router is ready."

    foreach ($profileAlias in $Profiles) {
        Write-Host "`nTesting Profile: $profileAlias" -ForegroundColor Cyan
        
        foreach ($ctx in $Contexts) {
            Write-Host "  Testing Context: $ctx..." -NoNewline
            
            # Update config
            $config = Get-Content $ConfigPath -Raw | ConvertFrom-Json
            $profile = $config.profiles | Where-Object { $_.alias -eq $profileAlias }
            
            if (-not $profile) {
                Write-Host " NOT FOUND" -ForegroundColor Red
                continue
            }

            $profile.context = $ctx
            $profile.launch_command = $profile.launch_command -replace "-c \d+", "-c $ctx"
            
            $config | ConvertTo-Json -Depth 10 | Set-Content $ConfigPath

            try {
                # Restart backend
                $restartBody = @{ profile = $profileAlias } | ConvertTo-Json
                $restartResp = Invoke-RestMethod -Uri "$RouterUrl/backend/restart" -Method Post -Body $restartBody -ContentType "application/json"
                
                # Wait for health
                $healthy = $false
                for ($i=0; $i -lt 30; $i++) {
                    try {
                        $statusResp = Invoke-RestMethod -Uri "$RouterUrl/backend/status" -Method Get -TimeoutSec 2
                        # Use a more robust way to access the profile state in a PSCustomObject
                        $profileStatus = $statusResp.profiles | Get-Member -MemberType NoteProperty | Where-Object { $_.Name -eq $profileAlias }
                        if ($profileStatus) {
                            $state = $statusResp.profiles."$profileAlias".state
                            if ($state -eq "healthy") {
                                $healthy = $true
                                break
                            }
                        }
                    } catch { }
                    Start-Sleep -Seconds 2
                }


                if (-not $healthy) {
                    Write-Host " FAIL (Health Timeout)" -ForegroundColor Red
                    Log-Result $profileAlias $ctx "FAIL" "Backend failed to reach healthy state"
                    continue
                }

                # Test Chat
                $chatBody = @{ 
                    profile = $profileAlias; 
                    messages = @(@{ role = "user"; content = "Reply with OK only." }) 
                } | ConvertTo-Json
                
                $chatResp = Invoke-RestMethod -Uri "$RouterUrl/backend/chat" -Method Post -Body $chatBody -ContentType "application/json"
                
                if ($chatResp.status -eq "ok" -and $chatResp.content -match "OK") {
                    Write-Host " PASS" -ForegroundColor Green
                    Log-Result $profileAlias $ctx "PASS" "Response: $($chatResp.content)"
                } else {
                    Write-Host " FAIL (Chat Error)" -ForegroundColor Red
                    Log-Result $profileAlias $ctx "FAIL" "Chat response invalid: $($chatResp.content)"
                }
            } catch {
                Write-Host " FAIL (Exception)" -ForegroundColor Red
                Log-Result $profileAlias $ctx "FAIL" $_.Exception.Message
            }
        }
    }
} catch {
    Write-Host "Critical Error: $($_.Exception.Message)" -ForegroundColor Red
} finally {
    # 3. Cleanup
    Write-Host "`nCleaning up..." -ForegroundColor Yellow
    if ($routerProc) { Stop-Process -Id $routerProc.Id -Force -ErrorAction SilentlyContinue }
    Get-Process llama-server -ErrorAction SilentlyContinue | Stop-Process -Force
    Move-Item -Path $BackupPath -Destination $ConfigPath -Force
    Write-Host "Router and backends stopped, config restored."
}
