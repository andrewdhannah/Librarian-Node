# Example Phi-4 startup script.
# Edit LLAMA_SERVER and MODEL_FILE before use.

$LLAMA_SERVER = "C:\OpenWork\llama.cpp\build\bin\Release\llama-server.exe"
$MODEL_FILE = "C:\OpenWork\TheLibrarian-ModelServices\models\phi4-3b-local\model.gguf"
$PORT = 9120
$CTX = 4096

Write-Host "Starting Phi-4 local server on port $PORT"
Write-Host "Model: $MODEL_FILE"

& $LLAMA_SERVER `
  -m $MODEL_FILE `
  --port $PORT `
  -c $CTX `
  --host 0.0.0.0
