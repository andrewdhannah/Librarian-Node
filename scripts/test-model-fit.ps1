param(
    [string]$ModelFile,
    [string]$Alias,
    [int]$Ngl,
    [int]$Port = 9120,
    [int]$Context = 1024,
    [string]$ServerPath = "G:\openwork\librarian-runtime-node\runtime\llama.cpp\llama-server.exe",
    [string]$LogDir = "G:\openwork\thelibrarian\fixtures\windows-runtime-node\model-fit\logs",
    [string]$EvidenceDir = "G:\openwork\thelibrarian\fixtures\windows-runtime-node\model-fit\evidence"
)

$ErrorActionPreference = 'Continue'
$result = @{
    model_file = $ModelFile
    alias = $Alias
    ngl = $Ngl
    context = $Context
    port = $Port
    started = $false
    health_ok = $false
    health_model = $null
    models_id = $null
    chat_ok = $false
    chat_content = $null
    finish_reason = $null
    process_alive_after = $false
    error = $null
}

$logTag = "$Alias-ngl$Ngl"
$logFile = "$LogDir\$logTag.log"
$evidenceFile = "$EvidenceDir\$logTag.json"
$startupTimeoutSec = 180

# Kill any existing server on our port
Get-Process -Name llama-server* -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1
try { $tcp = New-Object System.Net.Sockets.TcpClient; $tcp.Connect("127.0.0.1", $Port); $tcp.Close(); Start-Sleep 2 } catch {}

Write-Output "[$logTag] Starting: $ModelFile at ngl=$Ngl..."

# Start server
$modelPath = "G:\llama.cpp\models\$ModelFile"
if (-not (Test-Path $modelPath)) {
    $result.error = "Model file not found: $modelPath"
    $result | ConvertTo-Json -Depth 5 | Set-Content $evidenceFile
    return $result
}

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $ServerPath
$psi.Arguments = "-m `"$modelPath`" -p $Port -c $Context -ngl $Ngl -n 512 --alias `"$Alias`""
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $false
$psi.RedirectStandardError = $false
$psi.WindowStyle = [System.Diagnostics.ProcessWindowStyle]::Hidden

$p = $null
try {
    $p = [System.Diagnostics.Process]::Start($psi)
    Write-Output "[$logTag] PID=$($p.Id)"
} catch {
    $result.error = "Launch failed: $_"
    $result | ConvertTo-Json -Depth 5 | Set-Content $evidenceFile
    return $result
}

# Wait for health (up to timeout)
$startTime = (Get-Date)
for ($i = 0; $i -lt ($startupTimeoutSec / 3); $i++) {
    Start-Sleep -Seconds 3
    $elapsed = [math]::Round((New-TimeSpan -Start $startTime -End (Get-Date)).TotalSeconds)
    
    if ($p.HasExited) {
        $result.error = "Process exited (code $($p.ExitCode)) before health after ${elapsed}s"
        Write-Output "[$logTag] FAILED: $($result.error)"
        $result | ConvertTo-Json -Depth 5 | Set-Content $evidenceFile
        return $result
    }
    
    try {
        $h = Invoke-RestMethod "http://localhost:$Port/health" -TimeoutSec 3 -ErrorAction Stop
        if ($h.status -eq 'ok') {
            $result.started = $true
            $result.health_ok = $true
            $result.health_model = $h.model
            Write-Output "[$logTag] HEALTH OK after ${elapsed}s: model=$($h.model)"
            break
        }
    } catch {}
}

if (-not $result.health_ok) {
    $result.error = "Timed out after ${startupTimeoutSec}s — no health response"
    Write-Output "[$logTag] FAILED: $($result.error)"
    $result | ConvertTo-Json -Depth 5 | Set-Content $evidenceFile
    if (-not $p.HasExited) { $p.Kill() }
    return $result
}

# Query /v1/models
Start-Sleep -Seconds 1
try {
    $m = Invoke-RestMethod "http://localhost:$Port/v1/models" -TimeoutSec 5
    if ($m.data -and $m.data[0]) { $result.models_id = $m.data[0].id }
    Write-Output "[$logTag] /v1/models: id=$($result.models_id)"
} catch {
    $result.error = "/v1/models failed: $_"
    Write-Output "[$logTag] /v1/models FAILED: $_"
}

# Tiny chat completion
Start-Sleep -Seconds 1
try {
    $body = @{
        messages = @(@{role="user";content="Reply with OK only."})
        max_tokens = 8
        temperature = 0
    } | ConvertTo-Json
    $chat = Invoke-RestMethod "http://localhost:$Port/v1/chat/completions" -Method Post -Body $body -ContentType "application/json" -TimeoutSec 120
    if ($chat.choices -and $chat.choices[0]) {
        $result.chat_ok = $chat.choices[0].message.content -notlike "*ERROR*"
        $result.chat_content = $chat.choices[0].message.content
        $result.finish_reason = $chat.choices[0].finish_reason
        $result.model = $chat.model
    }
    Write-Output "[$logTag] Chat: content='$($result.chat_content)' finish='$($result.finish_reason)'"
} catch {
    $result.error = "Chat failed: $_"
    Write-Output "[$logTag] Chat FAILED: $_"
}

# Check process alive after test
$result.process_alive_after = (-not $p.HasExited)
Write-Output "[$logTag] Process alive after test: $($result.process_alive_after)"

# Kill process
if (-not $p.HasExited) { $p.Kill(); Start-Sleep -Seconds 1 }
Write-Output "[$logTag] Done."

# Save evidence
$result | ConvertTo-Json -Depth 5 | Set-Content $evidenceFile

return $result
