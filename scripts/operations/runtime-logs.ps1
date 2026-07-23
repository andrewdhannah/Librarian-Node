<#
.SYNOPSIS
  Locate and display recent Librarian Runtime Node logs.

.DESCRIPTION
  Finds the most recent log files in the logs/ directory and displays
  their paths, sizes, and modification times. With -Tail, prints the
  last N lines of the most recent relevant log.

  Does NOT create noisy log artifacts.
  Does NOT expose secrets (only prints log content, does not search).

.PARAMETER Tail
  Optional: number of recent lines to print from the most relevant log.
  Default: 0 (just list paths).

.PARAMETER Name
  Optional: specific log filename to tail (e.g. "rust-router-service.log").
  Used with -Tail.

.EXAMPLE
  .\scripts\operations\runtime-logs.ps1
  .\scripts\operations\runtime-logs.ps1 -Tail 20
  .\scripts\operations\runtime-logs.ps1 -Name rust-router-service.log -Tail 50
#>

param(
    [int]$Tail = 0,
    [string]$Name = ""
)

$ErrorActionPreference = "Continue"
$RepoRoot = "G:\OpenWork\librarian-runtime-node"
$logDir = Join-Path -Path $RepoRoot -ChildPath "logs"
$serviceLog = Join-Path -Path $logDir -ChildPath "service-router-startup.log"
$routerLog = Join-Path -Path $logDir -ChildPath "rust-router-service.log"

Write-Host "=== Runtime Logs ===" -ForegroundColor Cyan
Write-Host ""

$logDirExists = Test-Path -LiteralPath $logDir
if (-not $logDirExists) {
    Write-Host ("[LOGS]  Log directory not found: " + $logDir) -ForegroundColor Yellow
    exit 0
}

$logFiles = Get-ChildItem -LiteralPath $logDir -Filter "*.log" | Sort-Object LastWriteTime -Descending
if ($logFiles.Count -eq 0) {
    Write-Host ("[LOGS]  No .log files found in " + $logDir) -ForegroundColor Yellow
    exit 0
}

Write-Host ("Log directory: " + $logDir) -ForegroundColor White
Write-Host ("Total log files: " + $logFiles.Count) -ForegroundColor White
Write-Host ""

# Known relevant logs (always shown first)
if (Test-Path -LiteralPath $serviceLog) {
    $item = Get-Item -LiteralPath $serviceLog
    $ageMin = [math]::Round(((Get-Date) - $item.LastWriteTime).TotalMinutes, 0)
    Write-Host "[SERVICE STARTUP]  " -ForegroundColor White -NoNewline
    Write-Host $serviceLog -ForegroundColor Gray
    Write-Host ("   Modified: " + $ageMin + "m ago, Size: " + $item.Length + " bytes") -ForegroundColor Gray
}
if (Test-Path -LiteralPath $routerLog) {
    $item = Get-Item -LiteralPath $routerLog
    $ageMin = [math]::Round(((Get-Date) - $item.LastWriteTime).TotalMinutes, 0)
    Write-Host "[ROUTER]           " -ForegroundColor White -NoNewline
    Write-Host $routerLog -ForegroundColor Gray
    Write-Host ("   Modified: " + $ageMin + "m ago, Size: " + $item.Length + " bytes") -ForegroundColor Gray
}
Write-Host ""

# Recent logs
Write-Host "Recent log files (top 10 by modification time):" -ForegroundColor White
$idx = 0
foreach ($lf in $logFiles) {
    $idx += 1
    if ($idx -gt 10) { break }
    $ageMin = [math]::Round(((Get-Date) - $lf.LastWriteTime).TotalMinutes, 0)
    $sizeStr = "{0:N0}" -f $lf.Length
    Write-Host ("   " + $lf.Name + "  - " + $ageMin + "m ago, " + $sizeStr + " bytes") -ForegroundColor Gray
}
Write-Host ""

# Tail mode
if ($Tail -gt 0) {
    $targetLog = ""
    if ($Name -ne "") {
        $targetLog = Join-Path -Path $logDir -ChildPath $Name
        $logExists = Test-Path -LiteralPath $targetLog
        if (-not $logExists) {
            Write-Host ("[ERROR]  Log file not found: " + $targetLog) -ForegroundColor Red
            exit 1
        }
    } else {
        # Auto-select most recent among known service/router logs
        $candidates = @($routerLog, $serviceLog)
        foreach ($candidate in $candidates) {
            if (Test-Path -LiteralPath $candidate) {
                $targetLog = $candidate
                break
            }
        }
        if (($targetLog -eq "") -and ($logFiles.Count -gt 0)) {
            $targetLog = $logFiles[0].FullName
        }
    }

    if ($targetLog -ne "") {
        Write-Host ("=== Last " + $Tail + " lines of: " + $targetLog + " ===") -ForegroundColor Cyan
        Write-Host ""
        Get-Content -LiteralPath $targetLog -Tail $Tail
        Write-Host ""
        Write-Host "=== End of log ===" -ForegroundColor Cyan
    } else {
        Write-Host "[ERROR]  No log file found to tail." -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "=== End Logs ===" -ForegroundColor Cyan
