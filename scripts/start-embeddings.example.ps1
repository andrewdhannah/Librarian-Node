# Example embeddings startup script.
# Edit EMBEDDING_SERVER and MODEL_FILE before use.

$EMBEDDING_SERVER = "C:\OpenWork\llama.cpp\build\bin\Release\llama-server.exe"
$MODEL_FILE = "C:\OpenWork\TheLibrarian-ModelServices\models\local-embeddings\embedding-model.gguf"
$PORT = 9122
$CTX = 8192

Write-Host "Starting local embeddings server on port $PORT"
Write-Host "Model: $MODEL_FILE"

& $EMBEDDING_SERVER `
  -m $MODEL_FILE `
  --port $PORT `
  -c $CTX `
  --host 0.0.0.0 `
  --embedding
