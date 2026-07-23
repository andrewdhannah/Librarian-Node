<#
.SYNOPSIS
  List available GGUF models in the models directory.

.DESCRIPTION
  Scans G:\openwork\librarian-runtime-node\models\ for .gguf files
  and returns a JSON manifest.

.EXAMPLE
  .\scripts\list-models.ps1
#>

$modelsDir = "G:\openwork\librarian-runtime-node\models"
$models = @()

if (Test-Path $modelsDir) {
  $models = Get-ChildItem -Path $modelsDir -Filter "*.gguf" | ForEach-Object {
    @{
      name = $_.BaseName
      path = $_.FullName
      size_bytes = $_.Length
      last_modified = $_.LastWriteTime.ToString("o")
    }
  }
}

$result = @{
  timestamp = (Get-Date).ToString("o")
  model_dir = $modelsDir
  count = $models.Count
  models = $models
}

$result | ConvertTo-Json -Depth 3
