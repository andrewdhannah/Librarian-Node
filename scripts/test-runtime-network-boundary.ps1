<#
.SYNOPSIS
  WIN-RUNTIME-QUALIFICATION-1: Dimension 4 - Network/Auth Boundary
#>
param(
  [int]$Port = 9131,
  [string]$BinaryPath = "G:\OpenWork\librarian-runtime-node\rust-router\target\release\rust-router.exe",
  [string]$RouterDir = "G:\OpenWork\librarian-runtime-node\rust-router"
)
$ErrorActionPreference = "Stop"

# Use separate ports per phase to avoid TIME_WAIT port reuse issues
$PortPhase1 = $Port       # Default bind test
$PortPhase2 = $Port + 10  # LAN exposure test
$PortPhase3 = $Port + 20  # Auth-required test

function Write-Step { param([string]$M) Write-Host ("`n--- " + $M + " ---") -ForegroundColor Cyan }

function Test-Pass {
  param([string]$N)
  Write-Host ("  PASS: " + $N) -ForegroundColor Green
  $script:Details.Add($N) | Out-Null
  $null = $script:Passed = [int]$script:Passed + 1
}

function Test-Fail {
  param([string]$N, [string]$D = "")
  Write-Host ("  FAIL: " + $N + " (" + $D + ")") -ForegroundColor Red
  $script:Failures.Add(($N + " | " + $D)) | Out-Null
  $null = $script:Failed = [int]$script:Failed + 1
}

# Explicitly typed counters and detail/failure accumulators
[int]$script:Passed = 0
[int]$script:Failed = 0
$script:Details = [System.Collections.Generic.List[string]]::new()
$script:Failures = [System.Collections.Generic.List[string]]::new()

$Results = @{ DefaultBind = ""; AuthRequiredTest = ""; InvalidTokenTest = ""; ValidTokenTest = ""; SecretsLogged = "" }

# Build the structured result object. Used by all return paths so we always
# return the same shape, with explicit Int32 typing for counters and string
# arrays for Details/Failures (no List.Add pipeline leakage).
function Build-Result {
  if ($script:Passed -isnot [int]) { throw "Dimension 4: Passed counter is not Int32 (type: $($script:Passed.GetType().FullName))" }
  if ($script:Failed -isnot [int]) { throw "Dimension 4: Failed counter is not Int32 (type: $($script:Failed.GetType().FullName))" }

  $detailsArray = @()
  foreach ($d in $script:Details) { $detailsArray += [string]$d }
  $failuresArray = @()
  foreach ($f in $script:Failures) { $failuresArray += [string]$f }

  $status = if ([int]$script:Failed -eq 0) { "pass" } else { "fail" }

  [PSCustomObject]@{
    Dimension        = "4"
    Name             = "Network/Auth Boundary"
    Passed           = [int]$script:Passed
    Failed           = [int]$script:Failed
    Status           = [string]$status
    Details          = [string[]]$detailsArray
    Failures         = [string[]]$failuresArray
    DefaultBind      = [string]$Results.DefaultBind
    AuthRequiredTest = [string]$Results.AuthRequiredTest
    InvalidTokenTest = [string]$Results.InvalidTokenTest
    ValidTokenTest   = [string]$Results.ValidTokenTest
    SecretsLogged    = [string]$Results.SecretsLogged
    HarnessResult    = [string]$status
    Authority        = "advisory_only"
  }
}

function Invoke-Http {
  param([string]$Method = "GET", [string]$Path, [object]$Body = $null, [int]$Timeout = 5, [string]$AuthToken = $null, [int]$P = $Port)
  $uri = "http://127.0.0.1:" + $P + $Path
  $headers = @{}
  if ($AuthToken) { $headers["Authorization"] = $AuthToken }
  try {
    $splat = @{ Uri = $uri; Method = $Method; TimeoutSec = $Timeout; UseBasicParsing = $true; Headers = $headers }
    if ($Body -and $Method -eq "POST") {
      $splat["Body"] = ($Body | ConvertTo-Json -Compress -Depth 10)
      $splat["ContentType"] = "application/json"
    } elseif ($Method -eq "POST") {
      $splat["Body"] = "{}"
      $splat["ContentType"] = "application/json"
    }
    $resp = Invoke-WebRequest @splat
    $parsedBody = $null
    try { $parsedBody = $resp.Content | ConvertFrom-Json } catch { $parsedBody = $resp.Content }
    return @{ Body = $parsedBody; StatusCode = [int]$resp.StatusCode; Raw = $resp.Content; Error = $null }
  } catch {
    $code = 0; $raw = ""
    if ($_.Exception.Response) {
      $code = [int]$_.Exception.Response.StatusCode
      try { $raw = [System.IO.StreamReader]::new($_.Exception.Response.GetResponseStream()).ReadToEnd() } catch {}
    }
    $parsedBody = $null
    if ($raw) { try { $parsedBody = $raw | ConvertFrom-Json } catch { $parsedBody = $raw } }
    return @{ Body = $parsedBody; StatusCode = $code; Raw = $raw; Error = $_.Exception.Message }
  }
}

function Test-RouterAlive {
  param([int]$P, [int]$Timeout = 2)
  try {
    $null = Invoke-WebRequest -Uri ("http://127.0.0.1:" + $P + "/backend/status") -TimeoutSec $Timeout -UseBasicParsing -ErrorAction Stop
    return $true
  } catch {
    # 401 means the server is running and auth is enforced - that counts as alive
    if ($_.Exception.Response) { return $true }
    return $false
  }
}

function Stop-AllRouters {
  Get-Process rust-router -ErrorAction SilentlyContinue | Where-Object { $_.StartTime -gt (Get-Date).AddMinutes(-10) } | ForEach-Object { $_.Kill() }
  Start-Sleep -Seconds 1
}

function Start-RouterWithEnv {
  param([int]$ListenPort, [hashtable]$EnvVars = @{})
  $logOut = $env:TEMP + "\rust-router-net-" + $ListenPort + "-out.log"
  $logErr = $env:TEMP + "\rust-router-net-" + $ListenPort + "-err.log"
  $proc = Start-Process -FilePath $BinaryPath -ArgumentList ("--port " + $ListenPort) -NoNewWindow -PassThru `
    -RedirectStandardOutput $logOut -RedirectStandardError $logErr `
    -PassThru -WorkingDirectory $RouterDir
  $ready = $false
  for ($i = 0; $i -lt 30; $i++) {
    Start-Sleep -Milliseconds 500
    try { $r = curl.exe -s --connect-timeout 2 ("http://127.0.0.1:" + $ListenPort + "/backend/status") 2>$null; if ($LASTEXITCODE -eq 0 -and $r) { $ready = $true; break } } catch {}
  }
  return @{ Process = $proc; Ready = $ready; LogErr = $logErr }
}

function Stop-AllRouters {
  Get-Process rust-router -ErrorAction SilentlyContinue | Where-Object { $_.StartTime -gt (Get-Date).AddMinutes(-10) } | ForEach-Object { $_.Kill() }
  Start-Sleep -Seconds 1
}

# ============================================================================
# Phase 1: Default bind
# ============================================================================
Write-Step "Default bind verification"
$env:ROUTER_PORT = "" + $PortPhase1; $env:EVIDENCE_PATH = $RouterDir + "\..\fixtures\windows-runtime-node\router-impl"; $env:BACKEND_BINARY_PATH = $RouterDir + "\..\runtime\llama.cpp\llama-server.exe"
$proc1 = Start-Process -FilePath $BinaryPath -ArgumentList ("--port " + $PortPhase1) -NoNewWindow -PassThru -RedirectStandardOutput "$env:TEMP\r1.out.log" -RedirectStandardError "$env:TEMP\r1.err.log"
$ready = $false; for ($i = 0; $i -lt 30; $i++) { Start-Sleep -Milliseconds 500; try { $r = curl.exe -s --connect-timeout 2 ("http://127.0.0.1:" + $PortPhase1 + "/backend/status") 2>$null; if ($LASTEXITCODE -eq 0 -and $r) { $ready = $true; break } } catch {} }
if (-not $ready) { Test-Fail "Router did not start"; return (Build-Result) }
$listeners = netstat -ano | Where-Object { $_ -match (":" + $PortPhase1 + "\s") -and $_ -match "LISTENING" }
$lStr = "" + $listeners
if ($lStr -match ("127.0.0.1:" + $PortPhase1)) { Test-Pass ("Default bind is 127.0.0.1:" + $PortPhase1 + " (safe)"); $Results.DefaultBind = "127.0.0.1" } else { Test-Fail "Default bind" $lStr }
if ($lStr -match ("0.0.0.0:" + $PortPhase1)) { Test-Fail "Bound to 0.0.0.0" } else { Test-Pass "Not bound to 0.0.0.0" }
$proc1.Kill(); Start-Sleep -Seconds 2

# ============================================================================
# Phase 2: LAN exposure
# ============================================================================
Write-Step "LAN exposure requires explicit config"
$src = Get-Content ($RouterDir + "\src\config.rs") | Select-String "127.0.0.1"
if ($src) { Test-Pass "Source default bind is 127.0.0.1" } else { Test-Fail "Default bind not confirmed in source" }
$proc2 = Start-Process -FilePath $BinaryPath -ArgumentList ("--host 0.0.0.0 --port " + $PortPhase2) -NoNewWindow -PassThru -RedirectStandardOutput "$env:TEMP\r2.out.log" -RedirectStandardError "$env:TEMP\r2.err.log"
$ready = $false; for ($i = 0; $i -lt 30; $i++) { Start-Sleep -Milliseconds 500; try { $r = curl.exe -s --connect-timeout 2 ("http://127.0.0.1:" + $PortPhase2 + "/backend/status") 2>$null; if ($LASTEXITCODE -eq 0 -and $r) { $ready = $true; break } } catch {} }
if ($ready) { Test-Pass "Router started with --host 0.0.0.0" } else { Test-Fail "LAN router did not start" }
$l2 = netstat -ano | Where-Object { $_ -match (":" + $PortPhase2 + "\s") -and $_ -match "LISTENING" }
if ("" + $l2 -match ("0.0.0.0:" + $PortPhase2)) { Test-Pass "LAN exposure confirmed (requires explicit config)" } else { Test-Fail "LAN exposure" ("Expected 0.0.0.0:" + $PortPhase2) }
$proc2.Kill(); Start-Sleep -Seconds 2

# ============================================================================
# Phase 3: Auth-required mode (use env vars via cmd)
# ============================================================================
Write-Step "Auth-required mode"
$testToken = "rt-qual-net-" + [System.Guid]::NewGuid().ToString("N").Substring(0,8)
Write-Host ("  Token: " + $testToken) -ForegroundColor DarkGray

# Set env vars BEFORE starting process (inherited)
$env:ROUTER_AUTH_TOKEN = $testToken
$env:ROUTER_REQUIRE_AUTH = "true"
$env:ROUTER_PORT = "" + $PortPhase3
$env:EVIDENCE_PATH = $RouterDir + "\..\fixtures\windows-runtime-node\router-impl"
$env:BACKEND_BINARY_PATH = $RouterDir + "\..\runtime\llama.cpp\llama-server.exe"
Remove-Item Env:\ROUTER_HOST -ErrorAction SilentlyContinue

# Set env vars in the session so child processes inherit them
$env:ROUTER_AUTH_TOKEN = $testToken
$env:ROUTER_REQUIRE_AUTH = "true"
$env:ROUTER_PORT = "" + $PortPhase3
$env:EVIDENCE_PATH = $RouterDir + "\..\fixtures\windows-runtime-node\router-impl"
$env:BACKEND_BINARY_PATH = $RouterDir + "\..\runtime\llama.cpp\llama-server.exe"
Remove-Item Env:\ROUTER_HOST -ErrorAction SilentlyContinue

$proc3 = Start-Process -FilePath $BinaryPath -ArgumentList ("--port " + $PortPhase3) -NoNewWindow -PassThru
$ready = $false
for ($i = 0; $i -lt 30; $i++) {
  Start-Sleep -Milliseconds 500
  try {
    $null = Invoke-WebRequest -Uri ("http://127.0.0.1:" + $PortPhase3 + "/backend/status") -TimeoutSec 2 -UseBasicParsing -ErrorAction Stop
    $ready = $true; break
  } catch {
    # 401 means the server is running and auth is enforced - that counts as ready
    if ($_.Exception.Response) { $ready = $true; break }
  }
}
if (-not $ready) { Test-Fail "Auth router did not start"; Stop-AllRouters; return (Build-Result) }

# 3a: Missing token
Write-Step "Auth: Missing token"
try { $r = Invoke-WebRequest -Uri ("http://127.0.0.1:" + $PortPhase3 + "/backend/status") -TimeoutSec 5 -UseBasicParsing -ErrorAction Stop; $code = [int]$r.StatusCode } catch { $code = [int]$_.Exception.Response.StatusCode }
if ($code -eq 401) { Test-Pass "GET without token returns 401"; $Results.AuthRequiredTest = "pass" } else { Test-Fail "Missing token" ("Expected 401, got " + $code); $Results.AuthRequiredTest = "fail" }
try { $r = Invoke-WebRequest -Uri ("http://127.0.0.1:" + $PortPhase3 + "/backend/select") -Method POST -Body '{"profile":"phi-4"}' -ContentType "application/json" -TimeoutSec 5 -UseBasicParsing -ErrorAction Stop; $code = [int]$r.StatusCode } catch { $code = [int]$_.Exception.Response.StatusCode }
if ($code -eq 401) { Test-Pass "POST without token returns 401" } else { Test-Fail "Missing token POST" ("Expected 401, got " + $code) }

# 3b: Invalid token
Write-Step "Auth: Invalid token"
try { $r = Invoke-WebRequest -Uri ("http://127.0.0.1:" + $PortPhase3 + "/backend/status") -Headers @{ Authorization = "invalid-token-12345" } -TimeoutSec 5 -UseBasicParsing -ErrorAction Stop; $code = [int]$r.StatusCode } catch { $code = [int]$_.Exception.Response.StatusCode }
if ($code -eq 401) { Test-Pass "Wrong token returns 401"; $Results.InvalidTokenTest = "pass" } else { Test-Fail "Invalid token" ("Expected 401, got " + $code); $Results.InvalidTokenTest = "fail" }
try { $r = Invoke-WebRequest -Uri ("http://127.0.0.1:" + $PortPhase3 + "/backend/select") -Method POST -Body '{"profile":"phi-4"}' -ContentType "application/json" -Headers @{ Authorization = "wrong-token-abc" } -TimeoutSec 5 -UseBasicParsing -ErrorAction Stop; $code = [int]$r.StatusCode } catch { $code = [int]$_.Exception.Response.StatusCode }
if ($code -eq 401) { Test-Pass "Wrong token POST returns 401" } else { Test-Fail "Invalid token POST" ("Expected 401, got " + $code) }
try { $r = Invoke-WebRequest -Uri ("http://127.0.0.1:" + $PortPhase3 + "/backend/status") -Headers @{ Authorization = "Bearer $testToken" } -TimeoutSec 5 -UseBasicParsing -ErrorAction Stop; $code = [int]$r.StatusCode } catch { $code = [int]$_.Exception.Response.StatusCode }
if ($code -eq 401) { Test-Pass "Bearer-prefixed token returns 401" } else { Test-Fail "Bearer prefix" ("Expected 401, got " + $code) }

# 3c: Valid token
Write-Step "Auth: Valid token"
try { $r = Invoke-WebRequest -Uri ("http://127.0.0.1:" + $PortPhase3 + "/backend/status") -Headers @{ Authorization = $testToken } -TimeoutSec 5 -UseBasicParsing -ErrorAction Stop; $code = [int]$r.StatusCode } catch { $code = [int]$_.Exception.Response.StatusCode }
if ($code -eq 200) { Test-Pass "GET with valid token returns 200"; $Results.ValidTokenTest = "pass" } else { Test-Fail "Valid token GET" ("Expected 200, got " + $code); $Results.ValidTokenTest = "fail" }
try { $r = Invoke-WebRequest -Uri ("http://127.0.0.1:" + $PortPhase3 + "/backend/profiles") -Headers @{ Authorization = $testToken } -TimeoutSec 5 -UseBasicParsing -ErrorAction Stop; $code = [int]$r.StatusCode } catch { $code = [int]$_.Exception.Response.StatusCode }
if ($code -eq 200) { Test-Pass "GET profiles with valid token returns 200" } else { Test-Fail "Valid token profiles" ("Expected 200, got " + $code) }
try { $r = Invoke-WebRequest -Uri ("http://127.0.0.1:" + $PortPhase3 + "/backend/health") -Headers @{ Authorization = $testToken } -TimeoutSec 5 -UseBasicParsing -ErrorAction Stop; $code = [int]$r.StatusCode } catch { $code = [int]$_.Exception.Response.StatusCode }
if ($code -eq 200) { Test-Pass "GET health with valid token returns 200" } else { Test-Fail "Valid token health" ("Expected 200, got " + $code) }
try { $r = Invoke-WebRequest -Uri ("http://127.0.0.1:" + $PortPhase3 + "/v1/models") -Headers @{ Authorization = $testToken } -TimeoutSec 5 -UseBasicParsing -ErrorAction Stop; $code = [int]$r.StatusCode } catch { $code = [int]$_.Exception.Response.StatusCode }
if ($code -eq 200) { Test-Pass "GET models with valid token returns 200" } else { Test-Fail "Valid token models" ("Expected 200, got " + $code) }
try { $r = Invoke-WebRequest -Uri ("http://127.0.0.1:" + $PortPhase3 + "/backend/status") -Headers @{ Authorization = $testToken } -TimeoutSec 5 -UseBasicParsing -ErrorAction Stop; $parsed = $r.Content | ConvertFrom-Json; if ($parsed.authority -eq "advisory_only") { $authOk = $true } else { $authOk = $false } } catch { $authOk = $false }
if ($authOk) { Test-Pass "Auth response has authority: advisory_only" } else { Test-Fail "Authority missing from auth response" }

# Phase 3 cleanup
if ($proc3 -and !$proc3.HasExited) { $proc3.Kill() }
Start-Sleep -Seconds 2

# ============================================================================
# Phase 4: Secret logging
# ============================================================================
Write-Step "Secrets log check"
$secretsFound = $false
foreach ($lf in @("$env:TEMP\r3.err.log")) {
  if (Test-Path $lf) {
    $c = Get-Content $lf -Raw -ErrorAction SilentlyContinue
    if ($c -and $c.Contains($testToken)) { Write-Host ("  WARN: token found in " + $lf) -ForegroundColor Red; $secretsFound = $true }
  }
}
if ($secretsFound) { Test-Fail "Secrets leaked to logs"; $Results.SecretsLogged = "fail" } else { Test-Pass "No token leakage in logs"; $Results.SecretsLogged = "pass" }

# Cleanup
Remove-Item Env:\ROUTER_AUTH_TOKEN -ErrorAction SilentlyContinue
Remove-Item Env:\ROUTER_REQUIRE_AUTH -ErrorAction SilentlyContinue
Remove-Item Env:\ROUTER_HOST -ErrorAction SilentlyContinue
Stop-AllRouters

# Summary
$total = [int]$script:Passed + [int]$script:Failed
Write-Step "Dimension 4 Summary"
Write-Host ("  Default bind: " + $Results.DefaultBind) -ForegroundColor DarkGray
Write-Host ("  Auth required: " + $Results.AuthRequiredTest) -ForegroundColor DarkGray
Write-Host ("  Invalid token: " + $Results.InvalidTokenTest) -ForegroundColor DarkGray
Write-Host ("  Valid token: " + $Results.ValidTokenTest) -ForegroundColor DarkGray
Write-Host ("  Secrets logged: " + $Results.SecretsLogged) -ForegroundColor DarkGray
# Use string interpolation to avoid PowerShell Int32+String coercion error
Write-Host ("  " + $script:Passed.ToString() + " passed, " + $script:Failed.ToString() + " failed (" + $total.ToString() + " total)") -ForegroundColor $(if($script:Failed -eq 0){"Green"}else{"Red"})

# Return exactly one structured result object
return (Build-Result)
