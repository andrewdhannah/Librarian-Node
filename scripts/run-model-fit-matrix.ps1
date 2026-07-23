# WIN-MODEL-FIT-1: Systematic Model Fit Matrix Runner
# Tests each model at each ngl level, records results to CSV and evidence JSON.

$ErrorActionPreference = 'Continue'
$modelsDir = "G:\llama.cpp\models"
$serverPath = "G:\openwork\librarian-runtime-node\runtime\llama.cpp\llama-server.exe"
$logDir = "G:\openwork\thelibrarian\fixtures\windows-runtime-node\model-fit\logs"
$evidenceDir = "G:\openwork\thelibrarian\fixtures\windows-runtime-node\model-fit\evidence"
$csvPath = "G:\openwork\thelibrarian\fixtures\windows-runtime-node\model-fit\model-fit-matrix.csv"
$summaryPath = "G:\openwork\thelibrarian\fixtures\windows-runtime-node\model-fit\model-fit-summary.md"

if (-not (Test-Path $logDir)) { New-Item -ItemType Directory -Path $logDir -Force | Out-Null }
if (-not (Test-Path $evidenceDir)) { New-Item -ItemType Directory -Path $evidenceDir -Force | Out-Null }

# Define test models: name, file, alias
$testModels = @(
    @{name='qwen-coder'; file='qwen2.5-coder-1.5b-instruct-q8_0.gguf'}
    @{name='phi-4'; file='microsoft_Phi-4-mini-instruct-Q4_K_M.gguf'}
    @{name='llama-3.2'; file='Llama-3.2-3B-Instruct-Q5_K_M.gguf'}
    @{name='qwen3'; file='Qwen_Qwen3-4B-Q4_K_M.gguf'}
    @{name='gemma-3'; file='gemma-3-4b-it-Q4_K_M.gguf'}
)

$nglValues = @(20, 40, 60, 80, 99)
$port = 9120
$context = 1024

# CSV header
$csv = @()
$csv += "model,alias,model_file,model_size_gb,ngl,startup_result,health_ok,health_model,models_id,chat_ok,chat_content,finish_reason,process_alive,error"

$totalCells = $testModels.Count * $nglValues.Count
$cellNum = 0

foreach ($model in $testModels) {
    $modelPath = Join-Path $modelsDir $model.file
    $modelSize = if (Test-Path $modelPath) { [math]::Round((Get-Item $modelPath).Length / 1GB, 2) } else { 0 }
    
    foreach ($ngl in $nglValues) {
        $cellNum++
        Write-Output ""
        Write-Output "======================================================================"
        Write-Output "  CELL $cellNum/$totalCells : $($model.name) @ ngl=$ngl"
        Write-Output "======================================================================"
        
        $logTag = "$($model.name)-ngl$ngl"
        
        # Kill any existing server
        Get-Process -Name llama-server* -ErrorAction SilentlyContinue | Stop-Process -Force
        Start-Sleep -Seconds 2
        # Wait for port to free
        for ($retry=0; $retry -lt 10; $retry++) {
            try { $t = New-Object System.Net.Sockets.TcpClient; $t.Connect("127.0.0.1", $port); $t.Close(); Start-Sleep 2 } catch { break }
        }
        
        # Start server
        $startTime = Get-Date
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = $serverPath
        $psi.Arguments = "-m `"$modelPath`" -p $port -c $context -ngl $ngl -n 512 --alias `"$($model.name)`""
        $psi.UseShellExecute = $false
        $psi.RedirectStandardOutput = $false
        $psi.RedirectStandardError = $false
        $psi.WindowStyle = [System.Diagnostics.ProcessWindowStyle]::Hidden
        
        $p = $null
        $startupResult = 'not_tried'
        $healthOk = $false
        $healthModel = $null
        $modelsId = $null
        $chatOk = $false
        $chatContent = $null
        $finishReason = $null
        $processAlive = $false
        $errorMsg = $null
        
        try {
            $p = [System.Diagnostics.Process]::Start($psi)
            Write-Output "  PID=$($p.Id)"
        } catch {
            $errorMsg = "Launch failed: $_"
            Write-Output "  $errorMsg"
            # Record row
            $elapsed = [math]::Round((New-TimeSpan -Start $startTime -End (Get-Date)).TotalSeconds)
            $row = "$($model.name),$($model.name),$($model.file),$modelSize,$ngl,launch_failed,false,,,false,,,false,$errorMsg"
            $csv += $row
            continue
        }
        
        # Wait for health (up to 180s)
        $foundHealth = $false
        for ($i = 0; $i -lt 60; $i++) {
            Start-Sleep -Seconds 3
            $elapsed = [math]::Round((New-TimeSpan -Start $startTime -End (Get-Date)).TotalSeconds)
            
            if ($p.HasExited) {
                $startupResult = 'crashed'
                $errorMsg = "Process exited (code $($p.ExitCode)) before health after ${elapsed}s"
                Write-Output "  CRASHED after ${elapsed}s (code $($p.ExitCode))"
                break
            }
            
            try {
                $h = Invoke-RestMethod "http://localhost:$port/health" -TimeoutSec 3 -ErrorAction Stop
                if ($h.status -eq 'ok') {
                    $startupResult = 'healthy'
                    $healthOk = $true
                    $healthModel = $h.model
                    $foundHealth = $true
                    Write-Output "  HEALTH OK after ${elapsed}s: model=$($h.model)"
                    break
                }
            } catch {}
        }
        
        if (-not $foundHealth -and -not $errorMsg) {
            $startupResult = 'timeout'
            $errorMsg = "No health response within 180s"
            Write-Output "  TIMEOUT (no health after 180s)"
            if (-not $p.HasExited) { $p.Kill(); Start-Sleep -Seconds 1 }
        }
        
        if ($foundHealth) {
            # /v1/models
            try {
                $m = Invoke-RestMethod "http://localhost:$port/v1/models" -TimeoutSec 5
                if ($m.data -and $m.data[0]) { $modelsId = $m.data[0].id }
                Write-Output "  /v1/models: id=$modelsId"
            } catch {
                $errorMsg = "/v1/models failed: $_"
                Write-Output "  /v1/models FAILED: $_"
            }
            
            # Chat
            Start-Sleep -Seconds 1
            try {
                $body = @{
                    messages = @(@{role="user";content="Reply with OK only."})
                    max_tokens = 8
                    temperature = 0
                } | ConvertTo-Json
                $chat = Invoke-RestMethod "http://localhost:$port/v1/chat/completions" -Method Post -Body $body -ContentType "application/json" -TimeoutSec 120
                if ($chat.choices -and $chat.choices[0]) {
                    $content = $chat.choices[0].message.content
                    $chatOk = $content -notlike "*ERROR*"
                    $chatContent = $content
                    $finishReason = $chat.choices[0].finish_reason
                }
                Write-Output "  Chat: '$($chatContent.Substring(0, [Math]::Min(80, $chatContent.Length)))' finish=$finishReason"
            } catch {
                $errorMsg = "Chat failed: $_"
                Write-Output "  Chat FAILED: $_"
            }
        }
        
        # Check process alive
        $processAlive = (-not $p.HasExited)
        Write-Output "  Process alive: $processAlive"
        
        # Kill
        if (-not $p.HasExited) { $p.Kill(); Start-Sleep -Seconds 1 }
        
        # Save evidence JSON
        $evidence = @{
            model = $model.name
            alias = $model.name
            model_file = $model.file
            model_size_gb = $modelSize
            ngl = $ngl
            context = $context
            port = $port
            startup_result = $startupResult
            health_ok = $healthOk
            health_model = $healthModel
            models_id = $modelsId
            chat_ok = $chatOk
            chat_content = $chatContent
            finish_reason = $finishReason
            process_alive_after = $processAlive
            error = $errorMsg
            startup_time_s = [math]::Round((New-TimeSpan -Start $startTime -End (Get-Date)).TotalSeconds)
        }
        $evidence | ConvertTo-Json -Depth 5 | Set-Content "$evidenceDir\$logTag.json"
        
        # CSV row
        $escContent = if ($chatContent) { $chatContent.Replace('"', '""') } else { '' }
        $escError = if ($errorMsg) { $errorMsg.Replace('"', '""') } else { '' }
        $row = "$($model.name),$($model.name),$($model.file),$modelSize,$ngl,$startupResult,$healthOk,$healthModel,$modelsId,$chatOk,`"$escContent`",$finishReason,$processAlive,`"$escError`""
        $csv += $row
    }
}

# Write CSV
$csv -join "`n" | Set-Content $csvPath -Encoding ASCII
Write-Output ""
Write-Output "=============================================================================="
Write-Output "  CSV written to: $csvPath"
Write-Output "  Total cells: $cellNum"
Write-Output "=============================================================================="
