<#
.SYNOPSIS
  Start the Librarian Runtime Node Windows service.

.DESCRIPTION
  Starts the LibrarianRunTimeNode Windows service and verifies:
  - Service reaches Running state (waits up to 30s)
  - Port 9130 listener appears (waits up to 30s)

  Does NOT select a model/profile. Service starts the router;
  router waits for /backend/select.

  Does NOT require ROUTER_AUTH_TOKEN unless the service environment
  already requires it.

  Requires Administrator privileges to start the service.

.EXAMPLE
  .\scripts\operations\runtime-start.ps1
#>

$ErrorActionPreference = "Stop"
$ServiceName = "LibrarianRunTimeNode"
$RouterPort = if ($env:ROUTER_PORT) { [int]$env:ROUTER_PORT } else { 9130 }

Write-Host "=== Runtime Start ===" -ForegroundColor Cyan
Write-Host ""

# --- Elevation check ---
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")
if (-not $isAdmin) {
    Write-Host "[ERROR]  Administrator privileges required to start a Windows service." -ForegroundColor Red
    Write-Host "         Please run PowerShell as Administrator, then retry." -ForegroundColor Yellow
    Write-Host "         Example: Start-Process powershell -Verb RunAs" -ForegroundColor Yellow
    exit 1
}

# --- Pre-check: service exists ---
$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if (-not $svc) {
    Write-Host ("[ERROR]  Service '$ServiceName' not found.") -ForegroundColor Red
    exit 1
}

# --- Start if not already running ---
if ($svc.Status -eq 'Running') {
    Write-Host ("[SKIP]  Service '$ServiceName' is already Running.") -ForegroundColor Yellow
} else {
    Write-Host ("Starting service '$ServiceName'...") -ForegroundColor White
    Start-Service -Name $ServiceName -ErrorAction Stop
    Write-Host "[OK]   Start-Service succeeded." -ForegroundColor Green

    # Wait for Running state (up to 30s)
    $waitSeconds = 30
    $svc = Get-Service -Name $ServiceName
    while ($svc.Status -ne 'Running' -and $waitSeconds -gt 0) {
        Start-Sleep -Seconds 1
        $svc = Get-Service -Name $ServiceName
        $waitSeconds -= 1
    }
    if ($svc.Status -ne 'Running') {
        Write-Host ("[ERROR]  Service '$ServiceName' did not reach Running state within 30s.") -ForegroundColor Red
        exit 1
    }
    Write-Host ("[OK]   Service '$ServiceName' is Running.") -ForegroundColor Green
}

# --- Verify port 9130 listener (up to 30s) ---
Write-Host ("Waiting for port $RouterPort listener...") -ForegroundColor White
$waitSeconds = 30
$listenerFound = $false
$portEntry = $null
while ($waitSeconds -gt 0) {
    $portEntry = netstat -ano | Select-String (":$RouterPort.*LISTENING")
    if ($portEntry) {
        $listenerFound = $true
        break
    }
    Start-Sleep -Seconds 1
    $waitSeconds -= 1
}

if ($listenerFound) {
    $pidStr = ($portEntry -split '\s+')[-1]
    Write-Host ("[OK]   Port $RouterPort is LISTENING (PID: " + $pidStr + ")") -ForegroundColor Green
} else {
    Write-Host ("[WARN] Port $RouterPort listener not detected within 30s.") -ForegroundColor Yellow
    Write-Host "       Service may still be starting or router may need config." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "[NOTE] No model/profile selected." -ForegroundColor Cyan
Write-Host "       Use POST /backend/select to activate a profile, or" -ForegroundColor Cyan
Write-Host "       run '.\scripts\operations\runtime-status.ps1' to inspect." -ForegroundColor Cyan
Write-Host ""
Write-Host "=== Start Complete ===" -ForegroundColor Cyan
