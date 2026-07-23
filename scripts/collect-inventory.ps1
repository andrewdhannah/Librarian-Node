<#
.SYNOPSIS
  Collect hardware, network, and runtime evidence from the local Windows node
  and write into the Librarian repo's fixtures directory.

.DESCRIPTION
  This script gathers system information and writes immutable snapshot files
  into G:\openwork\thelibrarian\fixtures\windows-runtime-node\.

  Run after any hardware/network change to refresh the evidence.

.EXAMPLE
  .\scripts\collect-inventory.ps1
#>

$fixturesRoot = "G:\openwork\thelibrarian\fixtures\windows-runtime-node"
$timestamp = (Get-Date).ToString("yyyy-MM-ddTHH:mm:ssZ")

Write-Host "Collecting inventory → $fixturesRoot" -ForegroundColor Cyan

# --- Hardware ---
$hwDir = Join-Path $fixturesRoot "hardware"
if (-not (Test-Path $hwDir)) { New-Item -ItemType Directory -Path $hwDir -Force | Out-Null }

# systeminfo
Write-Host "  systeminfo..."
systeminfo | Out-File -FilePath (Join-Path $hwDir "systeminfo.txt") -Encoding utf8

# CPU
Write-Host "  CPU..."
Get-CimInstance Win32_Processor | Format-List * | Out-File -FilePath (Join-Path $hwDir "cpu.txt") -Encoding utf8

# Memory
Write-Host "  Memory..."
Get-CimInstance Win32_PhysicalMemory | Format-List * | Out-File -FilePath (Join-Path $hwDir "memory.txt") -Encoding utf8

# GPU
Write-Host "  GPU..."
Get-CimInstance Win32_VideoController | Format-List * | Out-File -FilePath (Join-Path $hwDir "gpu.txt") -Encoding utf8

# nvidia-smi
Write-Host "  nvidia-smi..."
$nvidiaPath = Join-Path $hwDir "nvidia-smi.txt"
try {
  nvidia-smi 2>$null | Out-File -FilePath $nvidiaPath -Encoding utf8
  if (-not (Get-Item $nvidiaPath).Length) { throw "nvidia-smi produced no output" }
} catch {
  "nvidia-smi not available (no NVIDIA GPU or driver missing)" | Out-File -FilePath $nvidiaPath -Encoding utf8
}

# vulkaninfo
Write-Host "  vulkaninfo..."
$vkPath = Join-Path $hwDir "vulkaninfo-summary.txt"
try {
  vulkaninfo --summary 2>$null | Out-File -FilePath $vkPath -Encoding utf8
  if (-not (Get-Item $vkPath).Length) { throw "vulkaninfo produced no output" }
} catch {
  "vulkaninfo not available" | Out-File -FilePath $vkPath -Encoding utf8
}

# --- Network ---
$netDir = Join-Path $fixturesRoot "network"
if (-not (Test-Path $netDir)) { New-Item -ItemType Directory -Path $netDir -Force | Out-Null }

# ipconfig
Write-Host "  ipconfig..."
ipconfig /all | Out-File -FilePath (Join-Path $netDir "ipconfig.txt") -Encoding utf8

# LAN addresses
Write-Host "  LAN addresses..."
Get-NetIPAddress -AddressFamily IPv4 | Select-Object IPAddress, InterfaceAlias, InterfaceIndex |
  Format-Table -AutoSize | Out-File -FilePath (Join-Path $netDir "lan-address.txt") -Encoding utf8

# Listening ports
Write-Host "  Ports..."
netstat -ano | Select-String "LISTENING" | Out-File -FilePath (Join-Path $netDir "ports.txt") -Encoding utf8

Write-Host "Inventory collection complete." -ForegroundColor Green
