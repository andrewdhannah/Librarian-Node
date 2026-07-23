# Test script for WIN-RUNTIME-NETWORK-BOUNDARY-1

$port = 9130
$token = "secret-token"

Write-Host "--- Testing Localhost Binding (Default) ---" -ForegroundColor Cyan
try {
    $response = Invoke-WebRequest -Uri "http://127.0.0.1:$port/backend/status" -Method Get -ErrorAction Stop
    Write-Host "Success: Localhost is reachable." -ForegroundColor Green
} catch {
    Write-Host "Failure: Localhost is NOT reachable. Error: $_" -ForegroundColor Red
}

Write-Host "`n--- Testing Authentication (Missing Token) ---" -ForegroundColor Cyan
try {
    $response = Invoke-WebRequest -Uri "http://127.0.0.1:$port/backend/status" -Method Get -ErrorAction Stop
    Write-Host "Failure: Request succeeded without token." -ForegroundColor Red
} catch {
    if $_.Exception.Response.StatusCode -eq 401 {
        Write-Host "Success: Request rejected with 401 Unauthorized." -ForegroundColor Green
    } else {
        Write-Host "Failure: Unexpected status code: $($_.Exception.Response.StatusCode)" -ForegroundColor Red
    }
}

Write-Host "`n--- Testing Authentication (Valid Token) ---" -ForegroundColor Cyan
try {
    $response = Invoke-WebRequest -Uri "http://127.0.0.1:$port/backend/status" -Method Get -Headers @{"Authorization" = $token} -ErrorAction Stop
    Write-Host "Success: Request succeeded with valid token." -ForegroundColor Green
} catch {
    Write-Host "Failure: Request failed with valid token. Error: $_" -ForegroundColor Red
}

Write-Host "`n--- Testing Request Size Limit ---" -ForegroundColor Cyan
$large_body = "A" * 20000 # 20KB
try {
    $response = Invoke-WebRequest -Uri "http://127.0.0.1:$port/backend/select" -Method Post -Body $large_body -ContentType "application/json" -ErrorAction Stop
    Write-Host "Failure: Oversized request succeeded." -ForegroundColor Red
} catch {
    if $_.Exception.Response.StatusCode -eq 413 {
        Write-Host "Success: Oversized request rejected with 413 Payload Too Large." -ForegroundColor Green
    } else {
        Write-Host "Failure: Unexpected status code: $($_.Exception.Response.StatusCode)" -ForegroundColor Red
    }
}

Write-Host "`n--- Testing LAN Binding (0.0.0.0) ---" -ForegroundColor Cyan
# We can't easily test 0.0.0.0 from within the same machine without knowing the IP, 
# but we can check if the server starts with it.
# This is more of a manual check or requires more complex setup.
Write-Host "Manual check required: Run 'ROUTER_HOST=0.0.0.0 cargo run' and try to connect from another device." -ForegroundColor Yellow
