<#
.SYNOPSIS
  Parity test: verify Rust router endpoints match Python router behavior.

.DESCRIPTION
  Starts both the Python and Rust routers on different ports, queries identical
  endpoints, and compares response shapes. Reports any discrepancies.

.EXAMPLE
  .\scripts\test-rust-router-parity.ps1
  .\scripts\test-rust-router-parity.ps1 -SkipPython
#>

param(
  [int]$RustPort = 9130,
  [int]$PythonPort = 9131,
  [switch]$SkipPython,
  [string]$RouterDir = "G:\OpenWork\librarian-runtime-node\rust-router",
  [string]$PythonRouter = "G:\OpenWork\librarian-runtime-node\router\router.py"
)

$RustUrl = "http://127.0.0.1:$RustPort"
$PythonUrl = "http://127.0.0.1:$PythonPort"
$RustBinary = "$RouterDir\target\release\rust-router.exe"

$Discrepancies = @()

function Start-Router {
  param([string]$Name, [string]$Exe, [string]$Args)
  Write-Host "  Starting $Name..." -NoNewline
  $proc = Start-Process -FilePath $Exe -ArgumentList $Args -NoNewWindow -PassThru
  Start-Sleep -Milliseconds 2000
  Write-Host " PID=$($proc.Id)" -ForegroundColor Cyan
  return $proc
}

function Wait-For-Health {
  param([string]$Url, [int]$TimeoutSec = 10)
  $deadline = (Get-Date).AddSeconds($TimeoutSec)
  while ((Get-Date) -lt $deadline) {
    try {
      $r = Invoke-WebRequest -Uri "$Url/health" -TimeoutSec 2 -ErrorAction Stop
      if ($r.StatusCode -eq 200) { return $true }
    } catch {}
    Start-Sleep -Milliseconds 500
  }
  return $false
}

function Get-Json {
  param([string]$Url, [string]$Path)
  try {
    $r = Invoke-WebRequest -Uri "$Url$Path" -TimeoutSec 10 -ErrorAction Stop
    return @{ StatusCode = [int]$r.StatusCode; Body = $r.Content | ConvertFrom-Json }
  } catch {
    return @{ StatusCode = $_.Exception.Response.StatusCode.value__; Body = $null; Error = $_.Exception.Message }
  }
}

function Compare-Shape {
  param([string]$Test, [object]$Rust, [object]$Python)
  $allOk = $true
  if (-not $SkipPython) {
    # Compare status codes
    if ($Rust.StatusCode -ne $Python.StatusCode) {
      Write-Host "    MISMATCH StatusCode: Rust=$($Rust.StatusCode) Python=$($Python.StatusCode)" -ForegroundColor Red
      $allOk = $false
    }
    # Compare top-level keys
    if (($null -ne $Rust.Body) -and ($null -ne $Python.Body)) {
      $rustKeys = $Rust.Body.PSObject.Properties.Name | Sort-Object
      $pyKeys = $Python.Body.PSObject.Properties.Name | Sort-Object
      $keysOnlyInRust = Compare-Object $rustKeys $pyKeys | Where-Object { $_.SideIndicator -eq '<=' }
      $keysOnlyInPython = Compare-Object $rustKeys $pyKeys | Where-Object { $_.SideIndicator -eq '=>' }
      if ($keysOnlyInRust) {
        Write-Host "    KEYS ONLY IN RUST: $($keysOnlyInRust.InputObject -join ', ')" -ForegroundColor Yellow
      }
      if ($keysOnlyInPython) {
        Write-Host "    KEYS ONLY IN PYTHON: $($keysOnlyInPython.InputObject -join ', ')" -ForegroundColor Yellow
      }
    }
  }
  if ($allOk) {
    Write-Host "    OK" -ForegroundColor Green
  } else {
    $script:Discrepancies += $Test
  }
}

# --- Main ---
Write-Host "=== Rust / Python Router Parity Test ===" -ForegroundColor Cyan
Write-Host ""

$procs = @()

# Start Rust router
$rustProc = Start-Router -Name "rust-router" -Exe $RustBinary -Args "--port $RustPort"
$procs += $rustProc
if (-not (Wait-For-Health -Url $RustUrl)) {
  Write-Host "FAIL: Rust router did not become healthy" -ForegroundColor Red
  $procs | ForEach-Object { if (-not $_.HasExited) { $_.Kill() } }
  exit 1
}
Write-Host "  Rust router healthy at $RustUrl" -ForegroundColor Green

# Start Python router (if not skipped)
$pyProc = $null
if (-not $SkipPython) {
  $pyProc = Start-Router -Name "python-router" -Exe "python" -Args "$PythonRouter --port $PythonPort"
  $procs += $pyProc
  if (-not (Wait-For-Health -Url $PythonUrl)) {
    Write-Host "WARN: Python router did not become healthy (skipping comparisons)" -ForegroundColor Yellow
    $SkipPython = $true
  } else {
    Write-Host "  Python router healthy at $PythonUrl" -ForegroundColor Green
  }
}

Write-Host ""

# --- Test: GET /backend/profiles ---
Write-Host "--- GET /backend/profiles ---" -ForegroundColor Cyan
$rustResult = Get-Json -Url $RustUrl -Path "/backend/profiles"
Write-Host "  Rust: profiles=$($rustResult.Body.profiles.Count)" -NoNewline
if ($SkipPython) { Write-Host "" } else {
  $pyResult = Get-Json -Url $PythonUrl -Path "/backend/profiles"
  Write-Host " Python: profiles=$($pyResult.Body.profiles.Count)"
  Compare-Shape -Test "GET /backend/profiles" -Rust $rustResult -Python $pyResult
}

# --- Test: GET /backend/status ---
Write-Host "--- GET /backend/status ---" -ForegroundColor Cyan
$rustResult = Get-Json -Url $RustUrl -Path "/backend/status"
Write-Host "  Rust: status=$($rustResult.Body.status)" -NoNewline
if ($SkipPython) { Write-Host "" } else {
  $pyResult = Get-Json -Url $PythonUrl -Path "/backend/status"
  Write-Host " Python: status=$($pyResult.Body.status)"
  Compare-Shape -Test "GET /backend/status" -Rust $rustResult -Python $pyResult
}

# --- Test: GET /health ---
Write-Host "--- GET /health ---" -ForegroundColor Cyan
$rustResult = Get-Json -Url $RustUrl -Path "/health"
Write-Host "  Rust: status=$($rustResult.Body.status)" -NoNewline
if ($SkipPython) { Write-Host "" } else {
  $pyResult = Get-Json -Url $PythonUrl -Path "/health"
  Write-Host " Python: status=$($pyResult.Body.status)"
  Compare-Shape -Test "GET /health" -Rust $rustResult -Python $pyResult
}

# --- Test: GET /backend/health ---
Write-Host "--- GET /backend/health ---" -ForegroundColor Cyan
$rustResult = Get-Json -Url $RustUrl -Path "/backend/health"
Write-Host "  Rust: status=$($rustResult.Body.status)" -NoNewline
if ($SkipPython) { Write-Host "" } else {
  $pyResult = Get-Json -Url $PythonUrl -Path "/backend/health"
  Write-Host " Python: status=$($pyResult.Body.status)"
  Compare-Shape -Test "GET /backend/health" -Rust $rustResult -Python $pyResult
}

# --- Test: POST /backend/select (invalid) ---
Write-Host "--- POST /backend/select (invalid profile) ---" -ForegroundColor Cyan
$badBody = @{ profile = "__nonexistent__" } | ConvertTo-Json -Compress
try {
  $r = Invoke-WebRequest -Uri "$RustUrl/backend/select" -Method POST -Body $badBody -ContentType "application/json" -TimeoutSec 10
  $rustResult = @{ StatusCode = [int]$r.StatusCode; Body = $null }
} catch {
  $rustResult = @{ StatusCode = [int]$_.Exception.Response.StatusCode; Body = $null }
}
Write-Host "  Rust: status_code=$($rustResult.StatusCode)" -NoNewline
if (-not $SkipPython) {
  try {
    $r = Invoke-WebRequest -Uri "$PythonUrl/backend/select" -Method POST -Body $badBody -ContentType "application/json" -TimeoutSec 10
    $pyResult = @{ StatusCode = [int]$r.StatusCode; Body = $null }
  } catch {
    $pyResult = @{ StatusCode = [int]$_.Exception.Response.StatusCode; Body = $null }
  }
  Write-Host " Python: status_code=$($pyResult.StatusCode)"
  if ($rustResult.StatusCode -eq $pyResult.StatusCode) {
    Write-Host "    OK" -ForegroundColor Green
  } else {
    Write-Host "    MISMATCH" -ForegroundColor Red
    $script:Discrepancies += "POST /backend/select (invalid)"
  }
} else { Write-Host "" }

# --- Summary ---
Write-Host ""
Write-Host "=== Results ===" -ForegroundColor Cyan
if ($Discrepancies.Count -eq 0) {
  Write-Host "All parity checks passed!" -ForegroundColor Green
} else {
  Write-Host "$($Discrepancies.Count) discrepancy(s) found:" -ForegroundColor Red
  $Discrepancies | ForEach-Object { Write-Host "  - $_" -ForegroundColor Yellow }
}

# --- Cleanup ---
Write-Host ""
Write-Host "Stopping routers..." -ForegroundColor Yellow
$procs | ForEach-Object {
  if ($_ -and !$_.HasExited) {
    $_.Kill()
    Write-Host "  PID $($_.Id) stopped"
  }
}
Write-Host "Done." -ForegroundColor Green

if ($Discrepancies.Count -gt 0) { exit 1 } else { exit 0 }
