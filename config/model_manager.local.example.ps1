<#
.SYNOPSIS
    EXAMPLE machine-local overrides for runtime/model_manager.ps1

.DESCRIPTION
    Copy to config/model_manager.local.ps1 (gitignored by config/*.local.* pattern)
    and edit paths/ports to match your machine. Only define variables you need
    to override — defaults from model_manager.ps1 are used for any undefined.

    See WIN-STARTUP-FILES-CUSTODY-1 for custody documentation.

.EXAMPLE
    Set-Content -Path "config\model_manager.local.ps1" -Value @"
    `$ServerPath = "G:\OpenWork\librarian-runtime-node\runtime\llama.cpp\llama-server.exe"
    `$ModelsDir  = "G:\llama.cpp\models"
    `$EmbedModelPath = "G:\llama.cpp\models\snowflake-arctic-embed-m-long-Q4_0.gguf"
    "@
#>

# ─── Machine-local backend binary ──────────────────────────────────────────
# AUTHORITATIVE BACKEND BINARY: llama-server.exe
# The default in model_manager.ps1 uses: PSScriptRoot\llama.cpp\llama-server.exe
# (resolves to: runtime/llama.cpp/llama-server.exe relative to the repo)
#
# If you need a different binary (e.g., historical 'llama-server-mini.exe' from
# a separate build directory), uncomment and set your local path:
# $ServerPath = "G:\OpenWork\librarian-runtime-node\runtime\llama.cpp\llama-server.exe"
# $ServerPath = "G:\llama.cpp\build_vs\bin\Release\llama-server-mini.exe"

# ─── Machine-local model directory ─────────────────────────────────────────
# $ModelsDir = "G:\llama.cpp\models"

# ─── Machine-local embedding model path ────────────────────────────────────
# $EmbedModelPath = "G:\llamacpp\snowflake-arctic-embed-m-long-Q4_0.gguf"

# ─── Temp/PID directory ────────────────────────────────────────────────────
# $PidDir = "G:\temp"
