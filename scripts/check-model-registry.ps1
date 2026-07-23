param(
    [string]$RegistryPath = ".\models\registry.json"
)

Write-Host "CUST-INF1 Local Model Registry Check"
Write-Host "Registry: $RegistryPath"

if (!(Test-Path $RegistryPath)) {
    Write-Error "Registry not found: $RegistryPath"
    exit 1
}

$registry = Get-Content $RegistryPath -Raw | ConvertFrom-Json
$root = Split-Path (Split-Path $RegistryPath -Parent) -Parent

$failures = 0

foreach ($svc in $registry.services) {
    Write-Host ""
    Write-Host "Service: $($svc.id)"
    Write-Host "Role:    $($svc.role)"
    Write-Host "Endpoint:$($svc.endpoint)"

    $cardPath = Join-Path $root $svc.model_card
    if (!(Test-Path $cardPath)) {
        Write-Warning "Missing model-card: $cardPath"
        $failures++
        continue
    }

    $card = Get-Content $cardPath -Raw | ConvertFrom-Json
    $modelFile = $card.model_file
    if ([System.IO.Path]::IsPathRooted($modelFile)) {
        $modelPath = $modelFile
    } else {
        $modelPath = Join-Path (Split-Path $cardPath -Parent) $modelFile
    }

    Write-Host "Expected model: $($card.expected_model_id)"
    Write-Host "Model file:     $modelPath"

    if (!(Test-Path $modelPath)) {
        Write-Warning "Model file not found. Put the model file here or update model-card.json."
        $failures++
        continue
    }

    $hash = (Get-FileHash -Algorithm SHA256 $modelPath).Hash.ToLower()
    Write-Host "SHA-256:        $hash"

    if ($card.sha256 -eq "TODO" -or [string]::IsNullOrWhiteSpace($card.sha256)) {
        Write-Warning "model-card.json sha256 is TODO. Record this hash in the model card."
        $failures++
    } elseif ($card.sha256.ToLower().Replace("sha256:", "") -ne $hash) {
        Write-Warning "Hash mismatch. model-card.json does not match actual file."
        $failures++
    } else {
        Write-Host "Hash:           OK"
    }
}

Write-Host ""
if ($failures -gt 0) {
    Write-Warning "Registry check completed with $failures issue(s)."
    exit 2
}

Write-Host "Registry check PASS."
exit 0
