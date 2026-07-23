#Requires -Version 5.1
<#
.SYNOPSIS
    MAC/WIN-ROUTER-CONTEXT-MEASURE-1 — PowerShell measurement wrapper.

.DESCRIPTION
    Runs the Python measurement harness and collects Windows-specific system info.
    Produces machine-readable JSON results and calibrated hardware profiles.

    DO NOT modify production router behavior.
    DO NOT implement live context routing.
    DO NOT modify model execution.

.SPRINT
    MAC/WIN-ROUTER-CONTEXT-MEASURE-1
#>

param(
    [string]$RepoRoot = "G:\OpenWork\librarian-runtime-node",
    [string]$PythonScript = "scripts\measurements\measure-router-context.py"
)

$ErrorActionPreference = "Continue"
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"

Write-Host "=" * 70
Write-Host "MAC/WIN-ROUTER-CONTEXT-MEASURE-1 — PowerShell Measurement Wrapper"
Write-Host "=" * 70
Write-Host "Timestamp: $timestamp"
Write-Host "Repo root: $RepoRoot"
Write-Host ""

# ------------------------------------------------------------------
# 1. Pre-measurement state checks
# ------------------------------------------------------------------
Write-Host "[Pre] Collecting system state..."

$gitStatus = git -C $RepoRoot status --short 2>&1
$gitHead = git -C $RepoRoot rev-parse --short HEAD 2>&1

$serviceState = Get-Service -Name "LibrarianRunTimeNode" -ErrorAction SilentlyContinue |
    Select-Object Name, Status, StartType

$routerProcesses = Get-Process -Name "rust-router", "llama-server", "python-router" -ErrorAction SilentlyContinue
$processCount = if ($routerProcesses) { $routerProcesses.Count } else { 0 }

Write-Host "  Git HEAD: $gitHead"
Write-Host "  Git status: $(if ($gitStatus) { $gitStatus } else { '(clean)' })"
Write-Host "  Service: $($serviceState.Name) - $($serviceState.Status) / $($serviceState.StartType)"
Write-Host "  Router processes: $processCount"
Write-Host ""

# ------------------------------------------------------------------
# 2. Collect Windows-specific system info
# ------------------------------------------------------------------
Write-Host "[1/4] Collecting Windows system info..."

$sysInfo = @{
    timestamp = $timestamp
    platform = "Windows"
    os_version = (Get-CimInstance Win32_OperatingSystem).Caption
    os_build = (Get-CimInstance Win32_OperatingSystem).BuildNumber
    processor = (Get-CimInstance Win32_Processor).Name
    ram_total_gb = [math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB, 2)
    gpu_name = (Get-CimInstance Win32_VideoController | Select-Object -First 1).Name
    gpu_vram_gb = [math]::Round((Get-CimInstance Win32_VideoController | Select-Object -First 1).AdapterRAM / 1GB, 2)
    git_head = "$gitHead"
    service_status = "$($serviceState.Status)"
    service_start_type = "$($serviceState.StartType)"
    router_process_count = $processCount
}

try {
    $disk = Get-CimInstance Win32_LogicalDisk -Filter "DeviceID='G:'" -ErrorAction SilentlyContinue
    if ($disk) {
        $sysInfo.disk_total_gb = [math]::Round($disk.Size / 1GB, 2)
        $sysInfo.disk_free_gb = [math]::Round($disk.FreeSpace / 1GB, 2)
    }
} catch { }

Write-Host "  OS: $($sysInfo.os_version)"
Write-Host "  CPU: $($sysInfo.processor)"
Write-Host "  RAM: $($sysInfo.ram_total_gb) GB"
Write-Host "  GPU: $($sysInfo.gpu_name) ($($sysInfo.gpu_vram_gb) GB)"
Write-Host ""

# ------------------------------------------------------------------
# 3. Run Python measurement harness
# ------------------------------------------------------------------
Write-Host "[2/4] Running Python measurement harness..."

$pythonScript = Join-Path $RepoRoot $PythonScript
if (Test-Path $pythonScript) {
    $env:PYTHONIOENCODING = "utf-8"
    $pythonOutput = & python $pythonScript 2>&1
    Write-Host $pythonOutput
    Write-Host ""
    Write-Host "  Python measurement complete."
} else {
    Write-Host "  WARNING: Python script not found at $pythonScript"
    Write-Host "  Running PowerShell-only measurements..."
}
Write-Host ""

# ------------------------------------------------------------------
# 4. Collect additional PowerShell measurements
# ------------------------------------------------------------------
Write-Host "[3/4] Running PowerShell-only measurements..."

# Measure file I/O directly via PowerShell
$testDir = Join-Path $RepoRoot "temp\ps-measurements"
if (-not (Test-Path $testDir)) { New-Item -ItemType Directory -Path $testDir -Force | Out-Null }

$psMeasurements = @()

# Small payload write/read
$smallPayload = "x" * (429 * 4)  # ~429 tokens
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$smallPath = Join-Path $testDir "small-payload.txt"
for ($i = 0; $i -lt 50; $i++) {
    [System.IO.File]::WriteAllText($smallPath, $smallPayload)
}
$sw.Stop()
$smallWriteMs = [math]::Round($sw.ElapsedMilliseconds / 50, 4)

$sw.Restart()
for ($i = 0; $i -lt 50; $i++) {
    [System.IO.File]::ReadAllText($smallPath)
}
$sw.Stop()
$smallReadMs = [math]::Round($sw.ElapsedMilliseconds / 50, 4)

$psMeasurements += @{
    operation = "file_write_small_429tok"
    median_ms = $smallWriteMs
    iterations = 50
}
$psMeasurements += @{
    operation = "file_read_small_429tok"
    median_ms = $smallReadMs
    iterations = 50
}

# Large payload write/read
$largePayload = "x" * (128000 * 4)  # ~128K tokens
$largePath = Join-Path $testDir "large-payload.txt"
$sw.Restart()
for ($i = 0; $i -lt 10; $i++) {
    [System.IO.File]::WriteAllText($largePath, $largePayload)
}
$sw.Stop()
$largeWriteMs = [math]::Round($sw.ElapsedMilliseconds / 10, 4)

$sw.Restart()
for ($i = 0; $i -lt 10; $i++) {
    [System.IO.File]::ReadAllText($largePath)
}
$sw.Stop()
$largeReadMs = [math]::Round($sw.ElapsedMilliseconds / 10, 4)

$psMeasurements += @{
    operation = "file_write_large_128ktok"
    median_ms = $largeWriteMs
    iterations = 10
}
$psMeasurements += @{
    operation = "file_read_large_128ktok"
    median_ms = $largeReadMs
    iterations = 10
}

# JSON parse/serialize
$jsonPayload = @{ context = "x" * (8000 * 4); tokens = 8000 } | ConvertTo-Json
$sw.Restart()
for ($i = 0; $i -lt 50; $i++) {
    $jsonPayload | ConvertFrom-Json
}
$sw.Stop()
$jsonParseMs = [math]::Round($sw.ElapsedMilliseconds / 50, 4)

$sw.Restart()
for ($i = 0; $i -lt 50; $i++) {
    @{ data = $jsonPayload } | ConvertTo-Json -Compress
}
$sw.Stop()
$jsonSerializeMs = [math]::Round($sw.ElapsedMilliseconds / 50, 4)

$psMeasurements += @{
    operation = "json_parse_8k_context"
    median_ms = $jsonParseMs
    iterations = 50
}
$psMeasurements += @{
    operation = "json_serialize_8k_context"
    median_ms = $jsonSerializeMs
    iterations = 50
}

# Network connection attempt
$sw.Restart()
try {
    $tcp = New-Object System.Net.Sockets.TcpClient
    $tcp.Connect("localhost", 8080)
    $tcp.Close()
} catch { }
$sw.Stop()
$networkConnectMs = [math]::Round($sw.ElapsedMilliseconds, 4)

$psMeasurements += @{
    operation = "tcp_connect_localhost_8080"
    median_ms = $networkConnectMs
    iterations = 1
    notes = "Connection refused expected (router stopped)"
}

# Cleanup
Remove-Item -Path $testDir -Recurse -Force -ErrorAction SilentlyContinue

Write-Host "  PowerShell measurements complete: $($psMeasurements.Count) measurements"
Write-Host ""

# ------------------------------------------------------------------
# 5. Append PowerShell results to JSON
# ------------------------------------------------------------------
Write-Host "[4/4] Appending PowerShell results to JSON..."

$resultsPath = Join-Path $RepoRoot "reports\router-context-measure-results.json"
if (Test-Path $resultsPath) {
    $existing = Get-Content $resultsPath -Raw | ConvertFrom-Json
    $existing.results | Add-Member -NotePropertyName "powershell_measurements" -NotePropertyValue $psMeasurements -Force
    $existing.system_info | Add-Member -NotePropertyName "powershell_sysinfo" -NotePropertyValue $sysInfo -Force
    $existing | ConvertTo-Json -Depth 10 | Set-Content $resultsPath -Encoding UTF8
    Write-Host "  Results appended to: $resultsPath"
} else {
    Write-Host "  WARNING: Results file not found. Python script may not have run."
}

# ------------------------------------------------------------------
# 6. Final state verification
# ------------------------------------------------------------------
Write-Host ""
Write-Host "=" * 70
Write-Host "Final State Verification"
Write-Host "=" * 70

$finalGitStatus = git -C $RepoRoot status --short 2>&1
$finalGitHead = git -C $RepoRoot rev-parse --short HEAD 2>&1
$finalService = Get-Service -Name "LibrarianRunTimeNode" -ErrorAction SilentlyContinue |
    Select-Object Name, Status, StartType
$finalProcesses = Get-Process -Name "rust-router", "llama-server", "python-router" -ErrorAction SilentlyContinue
$finalProcessCount = if ($finalProcesses) { $finalProcesses.Count } else { 0 }

Write-Host "Git HEAD: $finalGitHead"
Write-Host "Git status: $(if ($finalGitStatus) { $finalGitStatus } else { '(clean)' })"
Write-Host "Service: $($finalService.Name) - $($finalService.Status) / $($finalService.StartType)"
Write-Host "Router processes: $finalProcessCount"

# Check ports are free
$portsUsed = @()
foreach ($port in @(8080, 9120, 9121, 9122, 9123, 9124)) {
    $conn = Get-NetTCPConnection -LocalPort $port -ErrorAction SilentlyContinue
    if ($conn) { $portsUsed += $port }
}
if ($portsUsed.Count -gt 0) {
    Write-Host "WARNING: Ports still in use: $($portsUsed -join ', ')"
} else {
    Write-Host "All test ports are free."
}

Write-Host ""
Write-Host "Measurement complete."
