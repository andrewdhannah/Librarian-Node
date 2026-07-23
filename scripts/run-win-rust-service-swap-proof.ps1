<#
.SYNOPSIS
  Admin-elevated runner for WIN-RUST-SERVICE-SWAP-1 proof.
  This script REQUIRES admin privileges (to start/stop the NSSM service).

.DESCRIPTION
  Run this from an elevated PowerShell prompt:
    Start-Process powershell -Verb RunAs -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File scripts\run-win-rust-service-swap-proof.ps1"

  Or simply right-click PowerShell and "Run as Administrator", then:
    cd G:\OpenWork\librarian-runtime-node
    .\scripts\run-win-rust-service-swap-proof.ps1

  This script:
    1. Verifies prerequisites (service, NSSM, Rust binary)
    2. Ensures service is stopped and no stale processes exist
    3. Runs test-win-rust-service-swap.ps1 (the automated proof)
    4. Captures logs and evidence
    5. Reports pass/fail

.NOTES
  Sprint: WIN-RUST-SERVICE-SWAP-1
  Authority: advisory_only
#>

$BaseDir = "G:\OpenWork\librarian-runtime-node"
$ProofScript = "$BaseDir\scripts\test-win-rust-service-swap.ps1"
$NssmExe = "$BaseDir\runtime\bin\nssm.exe"
$ServiceName = "LibrarianRunTimeNode"
$EvidenceDir = "$BaseDir\fixtures\windows-runtime-node\router-impl"
$StartupLog = "$BaseDir\logs\service-router-startup.log"

# Verify admin
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
$isAdmin = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

Write-Host "=== WIN-RUST-SERVICE-SWAP-1 Admin Proof Runner ===" -ForegroundColor Cyan
Write-Host "Base directory: $BaseDir" -ForegroundColor DarkGray
Write-Host ""
Write-Host "Running as: $($identity.Name)" -ForegroundColor DarkGray
if (-not $isAdmin) {
    Write-Host ""
    Write-Host "ERROR: This script requires administrator privileges." -ForegroundColor Red
    Write-Host "Please run from an elevated PowerShell prompt:" -ForegroundColor Yellow
    Write-Host "  Start-Process powershell -Verb RunAs -ArgumentList '-NoProfile -ExecutionPolicy Bypass -File scripts\run-win-rust-service-swap-proof.ps1'" -ForegroundColor Yellow
    exit 1
}
Write-Host "Admin check: OK" -ForegroundColor Green
Write-Host ""

# Step 1: Clean slate
Write-Host "--- Pre-flight cleanup ---" -ForegroundColor Cyan

# Kill any stale rust-router or llama-server processes
$staleRust = Get-Process -Name "rust-router" -ErrorAction SilentlyContinue
if ($staleRust) {
    Write-Host "Stopping stale rust-router processes..." -ForegroundColor Yellow
    $staleRust | Stop-Process -Force
    Start-Sleep -Seconds 1
}

$staleLlama = Get-Process -Name "llama-server" -ErrorAction SilentlyContinue
if ($staleLlama) {
    Write-Host "Stopping stale llama-server processes..." -ForegroundColor Yellow
    $staleLlama | Stop-Process -Force
    Start-Sleep -Seconds 1
}

# Ensure service is stopped
$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($svc -and ($svc.Status -eq "Running")) {
    Write-Host "Stopping service $ServiceName..." -ForegroundColor Yellow
    Stop-Service -Name $ServiceName -Force
    Start-Sleep -Seconds 3
} else {
    Write-Host "Service is stopped." -ForegroundColor Green
}

# Clean logs
Write-Host "Cleaning old logs..." -ForegroundColor DarkGray
Remove-Item -Path "$BaseDir\logs\service-router-startup.log" -ErrorAction SilentlyContinue
Remove-Item -Path "$BaseDir\logs\rust-router-service.log" -ErrorAction SilentlyContinue
Remove-Item -Path "$BaseDir\logs\service-stdout.log" -ErrorAction SilentlyContinue
Remove-Item -Path "$BaseDir\logs\service-stderr.log" -ErrorAction SilentlyContinue

Write-Host "Ready." -ForegroundColor Green
Write-Host ""

# Step 2: Show NSSM config
Write-Host "--- Current NSSM Service Configuration ---" -ForegroundColor Cyan
Write-Host ("  Application:  " + (& $NssmExe get $ServiceName Application 2>$null))
Write-Host ("  Parameters:   " + (& $NssmExe get $ServiceName AppParameters 2>$null))
Write-Host ("  Directory:    " + (& $NssmExe get $ServiceName AppDirectory 2>$null))
Write-Host ("  Stdout:       " + (& $NssmExe get $ServiceName AppStdout 2>$null))
Write-Host ("  Stderr:       " + (& $NssmExe get $ServiceName AppStderr 2>$null))
Write-Host ("  Start type:   " + (& $NssmExe get $ServiceName Start 2>$null))
Write-Host ""

Write-Host "--- Service Launcher Script ---" -ForegroundColor Cyan
Get-Content "$BaseDir\scripts\start-librarian-runtime-node.ps1" | ForEach-Object { Write-Host "  $_" }
Write-Host ""

# Step 3: Run the proof
Write-Host "--- Running Automated Proof ---" -ForegroundColor Cyan
Write-Host ""

$startTime = Get-Date
& $ProofScript
$exitCode = $LASTEXITCODE
$elapsed = (Get-Date) - $startTime

Write-Host ""
Write-Host ("Proof completed in " + $elapsed.TotalSeconds.ToString("F1") + " seconds.") -ForegroundColor DarkGray
Write-Host ""

# Step 4: Collect results
Write-Host "--- Evidence Collection ---" -ForegroundColor Cyan

$cutoff = (Get-Date).AddMinutes(-30)
$evidenceFiles = Get-ChildItem -Path $EvidenceDir -Filter "*.json" -ErrorAction SilentlyContinue |
    Where-Object { $_.LastWriteTime -gt $cutoff }
if ($evidenceFiles) {
    Write-Host ("Evidence files written: " + $evidenceFiles.Count) -ForegroundColor Green
} else {
    Write-Host "No recent evidence files found." -ForegroundColor Yellow
}

$logFiles = Get-ChildItem -Path "$BaseDir\logs" -Filter "*.log" -ErrorAction SilentlyContinue |
    Where-Object { $_.LastWriteTime -gt $cutoff -and $_.Length -gt 0 }
if ($logFiles) {
    Write-Host "Log files:"
    $logFiles | ForEach-Object { Write-Host ("    " + $_.Name + " (" + $_.Length + " bytes)") -ForegroundColor DarkGray }
}

Write-Host ""

# Step 5: Cleanup check
Write-Host "--- Post-proof Orphan Check ---" -ForegroundColor Cyan
$orphansLlama = Get-Process -Name "llama-server" -ErrorAction SilentlyContinue
$orphansRust = Get-Process -Name "rust-router" -ErrorAction SilentlyContinue
if ($orphansLlama) { Write-Host ("WARNING: " + $orphansLlama.Count + " orphan llama-server processes") -ForegroundColor Yellow }
if ($orphansRust) { Write-Host ("WARNING: " + $orphansRust.Count + " orphan rust-router processes") -ForegroundColor Yellow }
if (-not $orphansLlama -and -not $orphansRust) {
    Write-Host "No orphan processes detected." -ForegroundColor Green
}

Write-Host ("")
Write-Host ("=== FINAL: " + $(if ($exitCode -eq 0) { "PASSED" } else { "FAILED" }) + " ===") -ForegroundColor $(if ($exitCode -eq 0) { "Green" } else { "Red" })
if ($exitCode -eq 0) {
    Write-Host "The Rust router is now the primary NSSM service path." -ForegroundColor Green
    Write-Host "Python router is retained as fallback." -ForegroundColor Green
}
exit $exitCode
