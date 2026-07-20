<#
.SYNOPSIS
    Collect Phase 0 evidence for the Windows Runtime Node.
.DESCRIPTION
    This script collects runtime evidence for Phase 0 of the Windows Runtime Node
    planning sprint. It is strictly observational — no modifications are made.
.NOTES
    Run from the repository root directory.
#>

$ErrorActionPreference = "Stop"
$evidenceDir = "evidence\phase0"

Write-Host "=== Phase 0 Evidence Collection ===" -ForegroundColor Cyan
Write-Host "Date: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
Write-Host "Machine: $env:COMPUTERNAME"
Write-Host ""

# 1. Rust toolchain
Write-Host "--- Rust Version ---" -ForegroundColor Yellow
$rustVersion = rustc --version 2>$null
$cargoVersion = cargo --version 2>$null
$rustVersion | Out-File -FilePath "$evidenceDir\rust-version.md" -Encoding UTF8
$cargoVersion | Out-File -FilePath "$evidenceDir\rust-version.md" -Encoding UTF8 -Append
Write-Host "Rust: $rustVersion"

# 2. Process list
Write-Host "--- Process List ---" -ForegroundColor Yellow
Get-Process | Where-Object { $_.ProcessName -match "librarian|llama|rust|python|nssm" } | 
    Format-Table Id, ProcessName, CPU, PM, StartTime -AutoSize | 
    Out-File -FilePath "$evidenceDir\process-list.md" -Encoding UTF8

# 3. Network listeners
Write-Host "--- Network Listeners ---" -ForegroundColor Yellow
Get-NetTCPConnection | Where-Object { $_.State -eq "Listen" } | 
    Select-Object LocalAddress, LocalPort, OwningProcess | 
    Out-File -FilePath "$evidenceDir\network-listeners.md" -Encoding UTF8

# 4. Hardware
Write-Host "--- Hardware ---" -ForegroundColor Yellow
Get-WmiObject Win32_VideoController | 
    Format-Table Name, AdapterRAM, DriverVersion -AutoSize | 
    Out-File -FilePath "$evidenceDir\gpu.md" -Encoding UTF8

Get-ComputerInfo | 
    Select-Object CsName, CsTotalPhysicalMemory, WindowsVersion | 
    Out-File -FilePath "$evidenceDir\hardware.md" -Encoding UTF8

# 5. Database status
Write-Host "--- Database Status ---" -ForegroundColor Yellow
$dbFiles = Get-ChildItem -Path "G:\openwork\librarian-runtime-node\data\" -Filter "*.db" -Recurse -ErrorAction SilentlyContinue
if ($dbFiles) {
    $dbFiles | Format-Table FullName, Length, LastWriteTime -AutoSize | 
        Out-File -FilePath "$evidenceDir\database-status.md" -Encoding UTF8
} else {
    "No database files found." | Out-File -FilePath "$evidenceDir\database-status.md" -Encoding UTF8
}

# 6. Runtime config
Write-Host "--- Runtime Config ---" -ForegroundColor Yellow
$configFiles = Get-ChildItem -Path "G:\openwork\librarian-runtime-node\config\" -Recurse -ErrorAction SilentlyContinue
if ($configFiles) {
    $configFiles | Format-Table FullName, Length, LastWriteTime -AutoType | 
        Out-File -FilePath "$evidenceDir\runtime-config.md" -Encoding UTF8
} else {
    "No config files found." | Out-File -FilePath "$evidenceDir\runtime-config.md" -Encoding UTF8
}

Write-Host ""
Write-Host "=== Collection Complete ===" -ForegroundColor Green
Write-Host "Evidence saved to: $evidenceDir"