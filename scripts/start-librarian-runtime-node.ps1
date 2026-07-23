<#
.SYNOPSIS
  Librarian Runtime Node Service Launcher (WIN-RUST-SERVICE-SWAP-1)
  Used by the Windows Service (via NSSM) to start the router.

.DESCRIPTION
  Primary: Rust router (rust-router.exe)
  Fallback: Python router (router.py)

  The Rust router is the primary service path. If it fails to start
  (non-zero exit, port conflict, missing binary, config error), the
  script falls back to the Python router. The fallback preserves
  operational continuity during the transition.

  Environment variables used by the Rust router:
    ROUTER_PORT          — HTTP port (default 9130)
    LOG_PATH             — structured log file path
    EVIDENCE_PATH        — evidence output directory
    BACKEND_BINARY_PATH  — llama-server.exe path
    HEALTH_POLL_INTERVAL_SECS — background health poll (default 5)
    HEALTH_TIMEOUT_SECS  — backend health wait (default 180)

.NOTES
  Sprint: WIN-RUST-SERVICE-SWAP-1
  Authoritative source: config/model-profiles.json
  Authority: advisory_only
#>

$WorkDir = "G:\OpenWork\librarian-runtime-node"
$Port = 9130

Set-Location -LiteralPath $WorkDir

# --- Paths ---
$RustRouter     = "$WorkDir\rust-router\target\release\rust-router.exe"
$PythonExe      = "C:\Python314\python.exe"
$RouterScript   = "$WorkDir\router\router.py"
$RustLogPath    = "$WorkDir\logs\rust-router-service.log"
$StartupLog     = "$WorkDir\logs\service-router-startup.log"

# --- Environment variables for Rust router ---
$env:ROUTER_PORT                 = "$Port"
$env:LOG_PATH                    = $RustLogPath
$env:EVIDENCE_PATH               = "$WorkDir\fixtures\windows-runtime-node\router-impl"
$env:BACKEND_BINARY_PATH         = "$WorkDir\runtime\llama.cpp\llama-server.exe"
$env:HEALTH_POLL_INTERVAL_SECS   = "5"
$env:HEALTH_TIMEOUT_SECS         = "180"

# ============================================================================
# Phase 1: Rust Router (Primary)
# ============================================================================
if (Test-Path -LiteralPath $RustRouter) {
    $msg = "[WIN-RUST-SERVICE-SWAP-1] Starting Rust router (primary) on port $Port..."
    Write-Host $msg -ForegroundColor Cyan
    $msg | Out-File -FilePath $StartupLog -Encoding utf8 -Append

    # Launch Rust router in foreground. PowerShell blocks until the process
    # exits. This keeps NSSM attached to the service process.
    & $RustRouter --port $Port
    $exitCode = $LASTEXITCODE

    if ($exitCode -eq 0) {
        # Clean exit (shutdown requested by NSSM) — propagate exit code
        $msg = "[WIN-RUST-SERVICE-SWAP-1] Rust router exited cleanly (code 0)."
        Write-Host $msg -ForegroundColor Green
        $msg | Out-File -FilePath $StartupLog -Encoding utf8 -Append
        exit 0
    }

    # Rust router exited with error — log and fall through to Python
    $msg = "[WIN-RUST-SERVICE-SWAP-1] Rust router exited with code $exitCode. Falling back to Python router."
    Write-Warning $msg
    $msg | Out-File -FilePath $StartupLog -Encoding utf8 -Append
} else {
    $msg = "[WIN-RUST-SERVICE-SWAP-1] Rust router binary not found at '$RustRouter'. Falling back to Python router."
    Write-Warning $msg
    $msg | Out-File -FilePath $StartupLog -Encoding utf8 -Append
}

# ============================================================================
# Phase 2: Python Router (Fallback)
# ============================================================================
$msg = "[WIN-RUST-SERVICE-SWAP-1] Starting Python router (fallback) on port $Port..."
Write-Host $msg -ForegroundColor Yellow
$msg | Out-File -FilePath $StartupLog -Encoding utf8 -Append

if (Test-Path -LiteralPath $PythonExe) {
    & $PythonExe -u $RouterScript --port $Port
    exit $LASTEXITCODE
} else {
    $msg = "[WIN-RUST-SERVICE-SWAP-1] CRITICAL: Python interpreter not found at '$PythonExe'. Cannot start router."
    Write-Error $msg
    $msg | Out-File -FilePath $StartupLog -Encoding utf8 -Append
    exit 1
}
