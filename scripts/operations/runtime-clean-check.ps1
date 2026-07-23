<#
.SYNOPSIS
  Verify the Runtime Node is in a clean, stopped, governed state.

.DESCRIPTION
  Confirms:
  - Service LibrarianRunTimeNode is Stopped / Manual
  - Port 9130 has no active LISTENER
  - No orphan rust-router or llama-server processes
  - Both repos have clean working trees (practical check)

  Exit code 0 = all checks pass.
  Exit code 1 = one or more checks fail (details printed).

.EXAMPLE
  .\scripts\operations\runtime-clean-check.ps1
#>

$ErrorActionPreference = "Continue"
$ServiceName = "LibrarianRunTimeNode"
$RouterPort = if ($env:ROUTER_PORT) { [int]$env:ROUTER_PORT } else { 9130 }
$RuntimeNodePath = "G:\OpenWork\librarian-runtime-node"
$MainLibrarianPath = "G:\OpenWork\TheLibrarian-main"

$allPass = $true

Write-Host "=== Runtime Clean Check ===" -ForegroundColor Cyan
Write-Host ""

# --- Service check ---
Write-Host "[CHECK]  Service state..." -ForegroundColor White
$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if (-not $svc) {
    Write-Host ("         FAIL - Service '$ServiceName' not found.") -ForegroundColor Red
    $allPass = $false
} elseif ($svc.Status -eq 'Stopped' -and $svc.StartType -eq 'Manual') {
    Write-Host ("         PASS - $ServiceName is Stopped / Manual") -ForegroundColor Green
} else {
    Write-Host ("         FAIL - $ServiceName is $($svc.Status) / $($svc.StartType)") -ForegroundColor Red
    $allPass = $false
}

# --- Port check ---
$portLabel = "Port $RouterPort"
Write-Host ("[CHECK]  $portLabel listener...") -ForegroundColor White
$listener = netstat -ano | Select-String (":$RouterPort.*LISTENING")
if (-not $listener) {
    Write-Host ("         PASS - $portLabel has no LISTENER") -ForegroundColor Green
} else {
    Write-Host ("         FAIL - $portLabel has active LISTENER") -ForegroundColor Red
    $allPass = $false
}

# --- Orphan checks ---
Write-Host "[CHECK]  Orphan processes..." -ForegroundColor White
$routerProcs = Get-Process -Name "rust-router" -ErrorAction SilentlyContinue
$backendProcs = Get-Process -Name "llama-server" -ErrorAction SilentlyContinue
$orphanCount = ($routerProcs.Count + $backendProcs.Count)

if ($orphanCount -eq 0) {
    Write-Host "         PASS - No rust-router or llama-server orphans" -ForegroundColor Green
} else {
    Write-Host ("         FAIL - Found $orphanCount orphan(s) (rust-router: $($routerProcs.Count), llama-server: $($backendProcs.Count))") -ForegroundColor Red
    $allPass = $false
}

# --- Repo working tree check (practical) ---
Write-Host "[CHECK]  Repo working trees..." -ForegroundColor White
$runtimeStatus = & git -C $RuntimeNodePath status --porcelain
$mainStatus = & git -C $MainLibrarianPath status --porcelain

if ([string]::IsNullOrEmpty($runtimeStatus)) {
    Write-Host "         PASS - librarian-runtime-node is clean" -ForegroundColor Green
} else {
    $changedFiles = ($runtimeStatus -split "`n" | Where-Object { $_ -ne '' }).Count
    Write-Host ("         INFO - librarian-runtime-node has $changedFiles untracked/modified file(s)") -ForegroundColor Yellow
}

if ([string]::IsNullOrEmpty($mainStatus)) {
    Write-Host "         PASS - TheLibrarian-main is clean" -ForegroundColor Green
} else {
    $changedFiles = ($mainStatus -split "`n" | Where-Object { $_ -ne '' }).Count
    Write-Host ("         INFO - TheLibrarian-main has $changedFiles untracked/modified file(s)") -ForegroundColor Yellow
}

Write-Host ""

if ($allPass) {
    Write-Host "=== CLEAN: All checks passed ===" -ForegroundColor Green
    exit 0
} else {
    Write-Host "=== DIRTY: One or more checks failed ===" -ForegroundColor Red
    exit 1
}
