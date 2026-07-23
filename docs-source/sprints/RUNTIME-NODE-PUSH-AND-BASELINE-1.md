# Sprint: RUNTIME-NODE-PUSH-AND-BASELINE-1
**Status:** COMPLETED  
**Date:** 2026-06-21  

## Objective
Push the 10 local commits accumulated since `6e2ab34`, verify remote state, and record the new Rust-router operational baseline. No feature work.

## Starting State
- **Local HEAD**: `53f0450` (10 commits ahead of `origin/main` at `6e2ab34`)
- **Working Tree**: Dirty (modified files from ROUTER-RUST-HARDEN-1)
- **Remote**: `origin/main` at `6e2ab34`

## Acceptance Gates

| Gate | Result |
|------|--------|
| ✅ `origin/main` updated to `53f0450` | **PASS** |
| ✅ Clean tree after push | **PASS** |
| ✅ Closeout docs present | **PASS** (`docs/sprints/RUNTIME-NODE-PUSH-AND-BASELINE-1.md`) |
| ✅ `.gitignore` excludes `target/`, `logs/`, `*.log`, `*.gguf`, `*.exe`, `*.dll` | **PASS** |
| ✅ Service remains `Stopped / Manual` | **PASS** |
| ✅ No `llama-server`, `rust-router`, or `python` orphan processes | **PASS** |

## Verification Steps

### Push
```bash
$ git push origin main
6e2ab34..53f0450  main -> main
```

### Remote Sync
```bash
$ git rev-parse HEAD
53f0450ca85f8317e75fcd771b6ecc6288c0b349
$ git rev-parse origin/main
53f0450ca85f8317e75fcd771b6ecc6288c0b349
✅ Local and remote are in sync
```

### Tree Cleanliness
```bash
$ git status --short
(nothing)
$ git ls-files --others --exclude-standard
(nothing)
✅ Clean tree, no untracked files
```

### Service State
```bash
$ Get-Service LibrarianRunTimeNode
Status  Name               DisplayName
------  ----               -----------
Stopped LibrarianRunTimeNode  Librarian Runtime Node
✅ Stopped / Manual — no unintended activation
```

### Orphan Check
```bash
$ Get-Process -Name "llama-server","rust-router","python"
(nothing)
✅ No orphan processes
```

## Baseline Summary

### Runtime Node State at `53f0450`

| Component | Status |
|-----------|--------|
| **Python router** | Retained at `router/router.py` (fallback) |
| **Rust router** | Operational at `rust-router/target/release/rust-router.exe` |
| **Router endpoints** | 10 endpoints (6 original + `/backend/restart`, `/v1/models`, `/v1/chat/completions`, `/health` legacy) |
| **Integration tests** | 15/15 passing |
| **Configuration** | 11 env var overrides with fallback defaults |
| **Health monitoring** | Background poller (5s) + on-demand |
| **Structured logging** | `LOG_PATH` env var for file output, stderr default |
| **Evidence** | Writes to `fixtures/windows-runtime-node/router-impl/` |
| **Service registration** | NSSM-ready (docs in `ROUTER-RUST-HARDEN-1-CLOSEOUT.md`) |
| **Commit hash** | `53f0450ca85f8317e75fcd771b6ecc6288c0b349` |
| **GitHub** | `https://github.com/andrewdhannah/librarian-runtime-node` |

### Profiles Loaded (5)
- `phi-4` — verified, port 9120, `general_advisory`, `summarization_advisory`
- `qwen-coder` — verified, port 9121, `code_advisory`, `fallback_small_model`
- `llama-3.2` — verified, port 9122, `general_advisory`
- `qwen3` — verified, port 9123, `general_advisory`, `reasoning`
- `gemma-3` — verified, port 9124, `general_advisory`

## Recommendation
The remote baseline is sealed. The next feature sprint can proceed cleanly from `53f0450`. Likely next steps:

1. **Windows service replacement** — Replace NSSM-wrapped Python router with NSSM-wrapped Rust router
2. **Mac/Linux port** — Add non-Windows process management (no `CREATE_NO_WINDOW`, no `CommandExt`)
3. **Metrics endpoint** — Prometheus `/metrics` for observability parity
