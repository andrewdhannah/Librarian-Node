<#
.SYNOPSIS
  Stop the Librarian Runtime Node and clean up governed processes.

.DESCRIPTION
  Stops the LibrarianRunTimeNode Windows service, then verifies:
  - Service reaches Stopped state (waits up to 30s)
  - Port 9130 has no active LISTENER
  - No orphan llama-server or rust-router processes remain

  Uses Stop-Service first, then Stop-Process for any remaining
  orphan governed processes that the service did not clean up.

  Does NOT kill unrelated processes.

  Requires Administrator privileges to stop the service.

.EXAMPLE
  .\scripts\operations\runtime-stop.ps1
#>

$ErrorActionPreference = "Continue"
$ServiceName = "LibrarianRunTimeNode"
$RouterPort = if ($env:ROUTER_PORT) { [int]$env:ROUTER_PORT } else { 9130 }

Write-Host "=== Runtime Stop ===" -ForegroundColor Cyan
Write-Host ""

# --- Elevation check ---
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")
if (-not $isAdmin) {
    Write-Host "[ERROR]  Administrator privileges required to stop a Windows service." -ForegroundColor Red
    Write-Host "         Please run PowerShell as Administrator, then retry." -ForegroundColor Yellow
    Write-Host "         Example: Start-Process powershell -Verb RunAs" -ForegroundColor Yellow
    exit 1
}

# --- Stop service ---
$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if (-not $svc) {
    Write-Host ("[SKIP]  Service '$ServiceName' not found.") -ForegroundColor Yellow
} elseif ($svc.Status -eq 'Stopped') {
    Write-Host ("[SKIP]  Service '$ServiceName' is already Stopped.") -ForegroundColor Yellow
} else {
    Write-Host ("Stopping service '$ServiceName'...") -ForegroundColor White
    Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
    if (-not $?) {
        Write-Host "[WARN]  Stop-Service failed. Attempting process-level cleanup." -ForegroundColor Yellow
    }

    # Wait for Stopped state (up to 30s)
    $waitSeconds = 30
    $svc = Get-Service -Name $ServiceName
    while ($svc.Status -ne 'Stopped' -and $waitSeconds -gt 0) {
        Start-Sleep -Seconds 1
        $svc = Get-Service -Name $ServiceName
        $waitSeconds -= 1
    }
    if ($svc.Status -eq 'Stopped') {
        Write-Host ("[OK]   Service '$ServiceName' is Stopped.") -ForegroundColor Green
    } else {
        Write-Host ("[WARN] Service '$ServiceName' did not reach Stopped within 30s (Status: " + $svc.Status + ").") -ForegroundColor Yellow
    }
}

# --- Orphan cleanup: rust-router ---
$routerProcs = Get-Process -Name "rust-router" -ErrorAction SilentlyContinue
if ($routerProcs) {
    Write-Host ("Cleaning " + $routerProcs.Count + " orphan rust-router process(es)...") -ForegroundColor Yellow
    foreach ($p in $routerProcs) {
        Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    }
    Start-Sleep -Seconds 2
    $remaining = Get-Process -Name "rust-router" -ErrorAction SilentlyContinue
    if (-not $remaining) {
        Write-Host "[OK]   rust-router orphan(s) cleaned." -ForegroundColor Green
    } else {
        Write-Host ("[WARN] " + $remaining.Count + " rust-router process(es) remain.") -ForegroundColor Yellow
    }
} else {
    Write-Host "[OK]   No rust-router orphan found." -ForegroundColor Gray
}

# --- Orphan cleanup: llama-server ---
$backendProcs = Get-Process -Name "llama-server" -ErrorAction SilentlyContinue
if ($backendProcs) {
    Write-Host ("Cleaning " + $backendProcs.Count + " orphan llama-server process(es)...") -ForegroundColor Yellow
    foreach ($p in $backendProcs) {
        Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    }
    Start-Sleep -Seconds 2
    $remaining = Get-Process -Name "llama-server" -ErrorAction SilentlyContinue
    if (-not $remaining) {
        Write-Host "[OK]   llama-server orphan(s) cleaned." -ForegroundColor Green
    } else {
        Write-Host ("[WARN] " + $remaining.Count + " llama-server process(es) remain.") -ForegroundColor Yellow
    }
} else {
    Write-Host "[OK]   No llama-server orphan found." -ForegroundColor Gray
}

Write-Host ""

# --- Verify port 9130 ---
Start-Sleep -Seconds 2
$portEntry = netstat -ano | Select-String (":$RouterPort.*LISTENING")
if (-not $portEntry) {
    Write-Host ("[OK]   Port $RouterPort has no LISTENER.") -ForegroundColor Green
} else {
    $listeningCount = ($portEntry | Where-Object { $_ -match "LISTENING" }).Count
    if ($listeningCount -gt 0) {
        Write-Host ("[WARN] " + $listeningCount + " LISTENING entry/entries still present on port $RouterPort.") -ForegroundColor Yellow
    } else {
        Write-Host ("[OK]   Port $RouterPort has no active LISTENER (residual TIME_WAIT only).") -ForegroundColor Green
    }
}

Write-Host ""
Write-Host "=== Stop Complete ===" -ForegroundColor Cyan
