# scripts/test-reconcile-optional.ps1
# Optional: test phi-4 and qwen-coder at context=2048 (ngl=99)
# Same restart-per-config method.

$RouterUrl = "http://127.0.0.1:9130"
$ConfigPath = "G:\openwork\librarian-runtime-node\config\model-profiles.json"
$BackupFile = "$ConfigPath.reconcile-2048-bak.json"

function Clean-Slate {
    Get-Process -Name "llama-server*" -ErrorAction SilentlyContinue | Stop-Process -Force
    Get-Process -Name "python*" -ErrorAction SilentlyContinue | Where-Object { $_.CommandLine -match "router" } | Stop-Process -Force
    Start-Sleep -Seconds 2
}

function Test-Cell($alias, $ngl, $ctx) {
    Write-Host "=== CELL: $alias ngl=$ngl context=$ctx ==="

    # Change config
    $json = Get-Content $ConfigPath -Raw | ConvertFrom-Json
    $profile = $json.profiles | Where-Object { $_.alias -eq $alias }
    $profile.ngl = $ngl
    $profile.context = $ctx
    $profile.launch_command = $profile.launch_command -replace "-ngl \d+", "-ngl $ngl"
    $profile.launch_command = $profile.launch_command -replace "-c \d+", "-c $ctx"
    $json | ConvertTo-Json -Depth 10 | Set-Content $ConfigPath
    Write-Host "  Config applied: $alias ngl=$ngl ctx=$ctx"

    # Start router (picks up new config)
    $rp = Start-Process python -ArgumentList "-u router/router.py --port 9130" -PassThru `
        -RedirectStandardOutput "router-stdout-2048.log" -RedirectStandardError "router-stderr-2048.log" -NoNewWindow
    $ready = $false
    for ($i = 0; $i -lt 30; $i++) {
        try {
            $null = Invoke-RestMethod -Uri "$RouterUrl/backend/status" -Method Get -TimeoutSec 2
            $ready = $true
            break
        } catch { Start-Sleep -Seconds 1 }
    }
    if (-not $ready) { Write-Host "  FAIL: Router not ready"; return "FAIL" }
    Write-Host "  Router ready (PID=$($rp.Id))"

    # Select profile
    $body = @{ profile = $alias } | ConvertTo-Json
    try {
        $resp = Invoke-RestMethod -Uri "$RouterUrl/backend/select" -Method Post -Body $body -ContentType "application/json" -TimeoutSec 10
        Write-Host "  Select: status=$($resp.status)"
    } catch {
        Write-Host "  FAIL: Select exception: $($_.Exception.Message)"
        Stop-Process -Id $rp.Id -Force; Clean-Slate; return "FAIL"
    }

    # Wait for healthy
    $healthy = $false
    for ($i = 0; $i -lt 60; $i++) {
        try {
            $s = Invoke-RestMethod -Uri "$RouterUrl/backend/status" -Method Get -TimeoutSec 3
            if ($s.profiles.$alias.state -eq "healthy") { $healthy = $true; break }
            if ($s.profiles.$alias.state -eq "error") { break }
        } catch { }
        Start-Sleep -Seconds 2
    }
    if (-not $healthy) {
        Write-Host "  FAIL: Not healthy"
        Stop-Process -Id $rp.Id -Force; Clean-Slate; return "FAIL"
    }
    Write-Host "  Backend healthy"

    # Chat test
    Start-Sleep -Seconds 2
    try {
        $chatBody = @{ profile = $alias; messages = @(@{ role = "user"; content = "Reply with OK only." }) } | ConvertTo-Json
        $chatResp = Invoke-RestMethod -Uri "$RouterUrl/backend/chat" -Method Post -Body $chatBody -ContentType "application/json" -TimeoutSec 120
        if ($chatResp.status -eq "ok") {
            Write-Host "  PASS: chat returned '$($chatResp.content)'"
            $result = "PASS"
        } else {
            Write-Host "  FAIL: chat status=$($chatResp.status), content=$($chatResp.content)"
            $result = "FAIL"
        }
    } catch {
        Write-Host "  FAIL: chat exception: $($_.Exception.Message)"
        $result = "FAIL"
    }

    Stop-Process -Id $rp.Id -Force -ErrorAction SilentlyContinue
    Clean-Slate
    return $result
}

# Main
Write-Host "=== OPTIONAL 2048 CONTEXT TESTS ==="

Clean-Slate
Copy-Item $ConfigPath $BackupFile -Force

$r1 = Test-Cell "phi-4" 99 2048
Write-Host "phi-4 ngl=99 ctx=2048 => $r1"

$r2 = Test-Cell "qwen-coder" 99 2048
Write-Host "qwen-coder ngl=99 ctx=2048 => $r2"

Copy-Item $BackupFile $ConfigPath -Force
Remove-Item $BackupFile -Force

Clean-Slate

Write-Host ""
Write-Host "=== RESULTS ==="
Write-Host "phi-4 ngl=99 ctx=2048: $r1"
Write-Host "qwen-coder ngl=99 ctx=2048: $r2"
