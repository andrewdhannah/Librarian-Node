<#
.SYNOPSIS
  Show operator status summary for the Librarian Runtime Node.

.DESCRIPTION
  Reports: service state, port 9130 listener, rust-router and llama-server
  process state, and paths to recent relevant log files.

  Does NOT modify system state. Does NOT require elevation for read-only checks.

.EXAMPLE
  .\scripts\operations\runtime-status.ps1
#>

$ErrorActionPreference = "Continue"
$RepoRoot = "G:\OpenWork\librarian-runtime-node"
$ServiceName = "LibrarianRunTimeNode"
$RouterPort = if ($env:ROUTER_PORT) { [int]$env:ROUTER_PORT } else { 9130 }

Write-Host "=== Librarian Runtime Node Status ===" -ForegroundColor Cyan
Write-Host ""

# --- Service state ---
$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if (-not $svc) {
    Write-Host "[SERVICE]  Not found: $ServiceName" -ForegroundColor Red
} else {
    $color = if ($svc.Status -eq 'Running') { 'Green' } elseif ($svc.Status -eq 'Stopped') { 'Yellow' } else { 'Gray' }
    Write-Host "[SERVICE]  $ServiceName" -ForegroundColor White -NoNewline
    Write-Host "  Status: " -NoNewline
    Write-Host "$($svc.Status)" -ForegroundColor $color -NoNewline
    Write-Host " / StartType: $($svc.StartType)"
}
Write-Host ""

# --- Port 9130 listener ---
$portEntry = netstat -ano | Select-String ":9130.*LISTENING"
if ($portEntry) {
    $pidStr = ($portEntry -split '\s+')[-1]
    Write-Host "[PORT]     9130" -ForegroundColor White -NoNewline
    Write-Host "  LISTENING  (PID: $pidStr)" -ForegroundColor Green
    $proc = Get-Process -Id $pidStr -ErrorAction SilentlyContinue
    if ($proc) {
        Write-Host "         Process: $($proc.ProcessName) (PID $pidStr)" -ForegroundColor Gray
    }
} else {
    Write-Host "[PORT]     9130" -ForegroundColor White -NoNewline
    Write-Host "  no LISTENER" -ForegroundColor Yellow
}
Write-Host ""

# --- Process checks ---
$routerProcs = Get-Process -Name "rust-router" -ErrorAction SilentlyContinue
$backendProcs = Get-Process -Name "llama-server" -ErrorAction SilentlyContinue

if ($routerProcs) {
    Write-Host "[PROC]     rust-router.exe" -ForegroundColor White -NoNewline
    Write-Host "  Running  (PID: $($routerProcs.Id -join ', '))" -ForegroundColor Green
} else {
    Write-Host "[PROC]     rust-router.exe" -ForegroundColor White -NoNewline
    Write-Host "  not running" -ForegroundColor Gray
}

if ($backendProcs) {
    Write-Host "[PROC]     llama-server.exe" -ForegroundColor White -NoNewline
    Write-Host "  Running  (PID: $($backendProcs.Id -join ', '))" -ForegroundColor Green
} else {
    Write-Host "[PROC]     llama-server.exe" -ForegroundColor White -NoNewline
    Write-Host "  not running" -ForegroundColor Gray
}
Write-Host ""

# --- Log paths ---
$logDir = "$RepoRoot\logs"
if (Test-Path -LiteralPath $logDir) {
    Write-Host "[LOGS]     $logDir" -ForegroundColor White
    $recentLogs = Get-ChildItem -LiteralPath $logDir -Filter "*.log" | Sort-Object LastWriteTime -Descending | Select-Object -First 5
    if ($recentLogs) {
        foreach ($lf in $recentLogs) {
            $age = [math]::Round(((Get-Date) - $lf.LastWriteTime).TotalMinutes, 0)
            Write-Host "           $($lf.Name)  (${age}m ago, $('{0:N0}' -f $lf.Length) bytes)" -ForegroundColor Gray
        }
    } else {
        Write-Host "           (no .log files found)" -ForegroundColor Gray
    }
} else {
    Write-Host "[LOGS]     $logDir" -ForegroundColor White -NoNewline
    Write-Host "  (directory not found)" -ForegroundColor Yellow
}
Write-Host ""

# --- Router log ---
$routerLog = "$RepoRoot\logs\rust-router-service.log"
if (Test-Path -LiteralPath $routerLog) {
    $lastMod = (Get-Item -LiteralPath $routerLog).LastWriteTime
    $age = [math]::Round(((Get-Date) - $lastMod).TotalMinutes, 0)
    Write-Host "[ROUTER LOG] $routerLog" -ForegroundColor White
    Write-Host "            Last modified: ${age}m ago" -ForegroundColor Gray
} else {
    Write-Host "[ROUTER LOG] $routerLog  (not found)" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "=== End Status ===" -ForegroundColor Cyan
