<#
.SYNOPSIS
  Start the local llama.cpp runtime.

.DESCRIPTION
  Reads config from config\runtime-node.local.json and launches llama.cpp.
  Default endpoint: http://localhost:8080

.EXAMPLE
  .\scripts\start-runtime.ps1
#>

$configPath = "G:\openwork\librarian-runtime-node\config\runtime-node.local.json"

if (-not (Test-Path $configPath)) {
  Write-Error "Config not found at $configPath. Copy runtime-node.example.json first."
  exit 1
}

$config = Get-Content $configPath | ConvertFrom-Json

if (-not $config.llama.binary_path -or -not (Test-Path $config.llama.binary_path)) {
  Write-Error "llama.cpp binary not found at '$($config.llama.binary_path)'. Check config."
  exit 1
}

if (-not $config.llama.model_path -or -not (Test-Path $config.llama.model_path)) {
  Write-Error "Model not found at '$($config.llama.model_path)'. Check config."
  exit 1
}

$args = @(
  "--model", $config.llama.model_path
  "--host", $config.llama.host
  "--port", [string]$config.llama.port
  "--n-gpu-layers", [string]$config.llama.n_gpu_layers
  "--ctx-size", [string]$config.llama.ctx_size
  "--threads", [string]$config.llama.threads
)

if ($config.llama.extra_args) {
  $args += $config.llama.extra_args
}

Write-Host "Starting llama.cpp..." -ForegroundColor Cyan
Write-Host "  Binary: $($config.llama.binary_path)"
Write-Host "  Model:  $($config.llama.model_path)"
Write-Host "  Endpoint: http://$($config.llama.host):$($config.llama.port)"
Write-Host ""

$logFile = "G:\openwork\librarian-runtime-node\logs\runtime-$(Get-Date -Format 'yyyyMMdd-HHmmss').log"

# Start as background job
$job = Start-Job -Name "llama.cpp" -ScriptBlock {
  param($bin, $model, $host, $port, $gpuLayers, $ctxSize, $threads, $logFile)
  & $bin --model $model --host $host --port $port --n-gpu-layers $gpuLayers --ctx-size $ctxSize --threads $threads *>&1 |
    ForEach-Object { $_ | Out-File -FilePath $logFile -Encoding utf8 -Append }
} -ArgumentList $config.llama.binary_path, $config.llama.model_path, $config.llama.host,
                           $config.llama.port, $config.llama.n_gpu_layers, $config.llama.ctx_size,
                           $config.llama.threads, $logFile

Write-Host "llama.cpp started as background job (Id=$($job.Id)). Log: $logFile" -ForegroundColor Green
Write-Host "Run health-check.ps1 to verify."
