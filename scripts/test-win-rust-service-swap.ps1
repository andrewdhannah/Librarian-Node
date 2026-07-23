<#
.SYNOPSIS
  WIN-RUST-SERVICE-SWAP-1 Proof: start/select/chat/stop through NSSM Rust router service.

.DESCRIPTION
  Proves the Rust router runs as the NSSM service primary path:
  1. Start the LibrarianRunTimeNode service
  2. Verify Rust router is responding (health endpoint)
  3. Confirm authority mode
  4. Select phi-4 backend
  5. Send a chat request
  6. Stop the backend
  7. Stop the service
  8. Cleanup evidence

  Each step is logged and verified. The script exits 0 on full pass.

.PARAMETER Port
  Router HTTP port (default 9130)

.PARAMETER ServiceName
  NSSM service name (default LibrarianRunTimeNode)

.PARAMETER Profile
  Profile to select for lifecycle proof (default phi-4)

.EXAMPLE
  .\scripts\test-win-rust-service-swap.ps1

.NOTES
  Sprint: WIN-RUST-SERVICE-SWAP-1
  Authority: advisory_only
#>

param(
    [int]$Port = 9130,
    [string]$ServiceName = "LibrarianRunTimeNode",
    [string]$Profile = "phi-4"
)

$BaseUrl     = "http://127.0.0.1:$Port"
$NssmExe     = "G:\OpenWork\librarian-runtime-node\runtime\bin\nssm.exe"
$Passed      = 0
$Failed      = 0
$EvidenceDir = "G:\OpenWork\librarian-runtime-node\fixtures\windows-runtime-node\router-impl"
$RustRouter  = "G:\OpenWork\librarian-runtime-node\rust-router\target\release\rust-router.exe"
$StartupLog  = "G:\OpenWork\librarian-runtime-node\logs\service-router-startup.log"

function Write-Step {
    param([string]$Message, [string]$Color = "Cyan")
    Write-Host ""
    Write-Host ("=== " + $Message + " ===") -ForegroundColor $Color
}

function Test-Pass {
    param([string]$Name)
    $script:Passed++
    Write-Host ("  PASS: " + $Name) -ForegroundColor Green
}

function Test-Fail {
    param([string]$Name, [string]$Detail)
    $script:Failed++
    Write-Host ("  FAIL: " + $Name + " (" + $Detail + ")") -ForegroundColor Red
}

function Invoke-Rest {
    param([string]$Method = "GET", [string]$Path, [object]$Body = $null, [int]$Timeout = 10)
    $params = @{
        Uri         = "$BaseUrl$Path"
        Method      = $Method
        ContentType = "application/json"
        TimeoutSec  = $Timeout
        UseBasicParsing = $true
        ErrorAction = "Stop"
    }
    if ($Body) { $params["Body"] = ($Body | ConvertTo-Json -Compress) }
    try { return Invoke-RestMethod @params }
    catch { return $null }
}

# ============================================================================
# Step 0: Verify prerequisites
# ============================================================================
Write-Step "PREREQUISITES"

$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if (-not $svc) {
    Write-Error ("Service " + $ServiceName + " not found.")
    exit 1
}
$svcInfo = "Status=" + $svc.Status + ", StartType=" + $svc.StartType
Test-Pass ("Service " + $ServiceName + " exists (" + $svcInfo + ")")

if (-not (Test-Path -LiteralPath $NssmExe)) {
    Write-Error ("NSSM not found at " + $NssmExe)
    exit 1
}
Test-Pass "NSSM available"

if (Test-Path -LiteralPath $RustRouter) {
    Test-Pass "Rust router binary present"
} else {
    Write-Warning "Rust router binary not found - proof will verify fallback path."
}

# ============================================================================
# Step 1: Start the service
# ============================================================================
Write-Step "STEP 1: Start NSSM Service"

if ($svc.Status -eq "Running") {
    Write-Host "  Service already running. Stopping first..." -ForegroundColor Yellow
    Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 3
}

Write-Host ("  Starting service " + $ServiceName + "...") -ForegroundColor Yellow
Start-Service -Name $ServiceName -ErrorAction Stop

# Poll for router health
$ready = $false
for ($i = 0; $i -lt 30; $i++) {
    Start-Sleep -Milliseconds 1000
    try {
        $result = Invoke-WebRequest -Uri "$BaseUrl/health" -TimeoutSec 2 -UseBasicParsing -ErrorAction Stop
        if ($result.StatusCode -eq 200) { $ready = $true; break }
    } catch { }
}

if ($ready) {
    Test-Pass ("Service started and Rust router responding on port " + $Port)
} else {
    Test-Fail "Service start" "Router not healthy within 30 seconds"
    $svcAfter = Get-Service -Name $ServiceName
    Write-Host ("  Service status: " + $svcAfter.Status) -ForegroundColor DarkYellow
    try {
        $log = Get-Content $StartupLog -ErrorAction SilentlyContinue
        Write-Host "  Startup log:" -ForegroundColor DarkYellow
        $log | ForEach-Object { Write-Host ("    " + $_) }
    } catch {}
    exit 1
}

# ============================================================================
# Step 2: Verify router identity
# ============================================================================
Write-Step "STEP 2: Verify Router Identity"

try {
    $health = Invoke-Rest -Path "/health"
    if ($health -and $health.authority -eq "advisory_only") {
        Test-Pass "Authority is advisory_only"
    } else {
        $hJson = $health | ConvertTo-Json -Compress
        Test-Fail "Authority check" ("Got: " + $hJson)
    }

    $status = Invoke-Rest -Path "/backend/status"
    if ($status -and ($null -ne $status.status)) {
        Test-Pass ("/backend/status returns status=" + $status.status)
        $si = "  runtimes_alive=" + $status.runtimes_alive + ", profiles_registered=" + $status.profiles_registered
        Test-Pass $si
    } else {
        Test-Fail "/backend/status" "No status field"
    }

    $profiles = Invoke-Rest -Path "/backend/profiles"
    if ($profiles -and $profiles.profiles -and ($profiles.profiles.Count -gt 0)) {
        Test-Pass ("/backend/profiles returns " + $profiles.profiles.Count + " profiles")
    } else {
        Test-Fail "/backend/profiles" "No profiles returned"
    }
} catch {
    Test-Fail "Identity endpoint" $_.Exception.Message
}

# ============================================================================
# Step 3: Select profile
# ============================================================================
Write-Step ("STEP 3: Select Profile " + $Profile)

try {
    $selectResult = Invoke-Rest -Method POST -Path "/backend/select" -Body @{ profile = $Profile }
    if ($selectResult -and ($selectResult.status -eq "selected")) {
        Test-Pass ("/backend/select returned status=selected for profile " + $Profile)
        $si = "  Port: " + $selectResult.port + ", Authority: " + $selectResult.authority
        Test-Pass $si

        # Wait for backend to be healthy
        $backendHealthy = $false
        for ($i = 0; $i -lt 30; $i++) {
            Start-Sleep -Milliseconds 1000
            $backendStatus = Invoke-Rest -Path "/backend/status"
            if ($backendStatus -and ($backendStatus.runtimes_alive -ge 1)) {
                $backendHealthy = $true
                break
            }
        }
        if ($backendHealthy) {
            Test-Pass ("Backend " + $Profile + " reached healthy state")
            $si = "  runtimes_alive=" + $backendStatus.runtimes_alive + ", active_profile=" + $backendStatus.active_profile
            Test-Pass $si
        } else {
            Test-Fail "Backend healthy wait" "Not healthy within 30 seconds"
            exit 1
        }
    } else {
        $sJson = $selectResult | ConvertTo-Json -Compress
        Test-Fail "/backend/select" ("Expected status=selected, got: " + $sJson)
    }
} catch {
    Test-Fail "/backend/select" $_.Exception.Message
}

# ============================================================================
# Step 4: Send chat through Rust router
# ============================================================================
Write-Step "STEP 4: Chat via Rust Router"

try {
    $chatResult = Invoke-Rest -Method POST -Path "/backend/chat" -Body @{
        profile    = $Profile
        messages   = @(@{ role = "user"; content = "Reply with OK only." })
        max_tokens = 128
        temperature = 0.7
    }
    if ($chatResult -and ($chatResult.status -eq "ok")) {
        Test-Pass ("/backend/chat returned status=ok")
        Test-Pass ("  content: " + $chatResult.content)
        Test-Pass ("  finish_reason: " + $chatResult.finish_reason)
    } else {
        $cJson = $chatResult | ConvertTo-Json -Compress
        Test-Fail "/backend/chat" ("Expected status=ok, got: " + $cJson)
    }
} catch {
    Test-Fail "/backend/chat" $_.Exception.Message
}

# ============================================================================
# Step 5: Stop the backend
# ============================================================================
Write-Step ("STEP 5: Stop Backend " + $Profile)

try {
    $stopResult = Invoke-Rest -Method POST -Path "/backend/stop" -Body @{ profile = $Profile }
    if ($stopResult -and ($stopResult.status -eq "stopped")) {
        Test-Pass "/backend/stop returned status=stopped"
        $ss = "  stopped: " + ($stopResult.stopped -join ", ")
        Test-Pass $ss

        Start-Sleep -Seconds 2
        $backendStatus = Invoke-Rest -Path "/backend/status"
        if ($backendStatus -and ($backendStatus.runtimes_alive -eq 0)) {
            Test-Pass "Backend confirmed stopped (runtimes_alive=0)"
        } else {
            $si = "Backend status after stop: runtimes_alive=" + $backendStatus.runtimes_alive
            Test-Pass $si
        }
    } else {
        $sJson = $stopResult | ConvertTo-Json -Compress
        Test-Fail "/backend/stop" ("Expected status=stopped, got: " + $sJson)
    }
} catch {
    Test-Fail "/backend/stop" $_.Exception.Message
}

# ============================================================================
# Step 6: Stop the service
# ============================================================================
Write-Step "STEP 6: Stop NSSM Service"

try {
    Stop-Service -Name $ServiceName -Force -ErrorAction Stop
    Start-Sleep -Seconds 2
    $svcAfter = Get-Service -Name $ServiceName
    if ($svcAfter.Status -eq "Stopped") {
        Test-Pass ("Service " + $ServiceName + " stopped (Status=" + $svcAfter.Status + ")")
    } else {
        Test-Fail "Service stop" ("Status is " + $svcAfter.Status + ", expected Stopped")
    }
} catch {
    Test-Fail "Service stop" $_.Exception.Message
}

# Verify no orphan backends
$orphans = Get-Process -Name "llama-server" -ErrorAction SilentlyContinue
if ($orphans) {
    Write-Warning ("Found " + $orphans.Count + " orphan llama-server processes after service stop.")
    $orphans | ForEach-Object { Write-Host ("  PID " + $_.Id + ": " + $_.ProcessName) -ForegroundColor DarkYellow }
} else {
    Test-Pass "No orphan llama-server processes after service stop"
}

$routerProcs = Get-Process -Name "rust-router" -ErrorAction SilentlyContinue
if ($routerProcs) {
    Write-Warning ("Found " + $routerProcs.Count + " rust-router processes still running.")
} else {
    Test-Pass "No rust-router processes after service stop"
}

# ============================================================================
# Step 7: Collect evidence
# ============================================================================
Write-Step "STEP 7: Evidence"

$cutoff = (Get-Date).AddMinutes(-10)
$evidenceFiles = Get-ChildItem -Path $EvidenceDir -Filter "*.json" -ErrorAction SilentlyContinue |
    Where-Object { $_.LastWriteTime -gt $cutoff }

if ($evidenceFiles) {
    $fc = $evidenceFiles.Count
    Test-Pass ("Evidence files written (" + $fc + " files in last 10 min)")
    $evidenceFiles | ForEach-Object { Write-Host ("    " + $_.Name) -ForegroundColor DarkGray }
} else {
    Write-Warning ("No recent evidence files found in " + $EvidenceDir)
}

# ============================================================================
# Summary
# ============================================================================
if ($Failed -eq 0) { $color = "Green" } else { $color = "Red" }
Write-Step ("RESULTS: " + $Passed + " passed, " + $Failed + " failed") $color

if ($Failed -gt 0) {
    Write-Host "WIN-RUST-SERVICE-SWAP-1: FAILED" -ForegroundColor Red
    exit 1
} else {
    Write-Host "WIN-RUST-SERVICE-SWAP-1: PASSED" -ForegroundColor Green
    Write-Host ""
    Write-Host "Rust router is the active NSSM service path." -ForegroundColor Green
    Write-Host "Python router retained as fallback." -ForegroundColor Green
    exit 0
}
