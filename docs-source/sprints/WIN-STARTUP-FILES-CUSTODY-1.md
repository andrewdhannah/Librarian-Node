# WIN-STARTUP-FILES-CUSTODY-1 — Startup Custody Normalization

**Date:** 2026-06-28
**Starting HEAD:** d2230e7
**Mode:** Custody hardening / operations-surface normalization
**Classification:** PROMOTE follow-up from WIN-STARTUP-FILES-CUSTODY-0

---

## Summary

Normalized Windows startup/runtime custody surfaces. Tracked files no longer encode inconsistent path casing, conflicting ports, or undocumented backend binary drift. Local config pattern introduced for machine-specific overrides.

---

## Changes Made

### 1. Path Casing Normalized to `G:\OpenWork`

| File | Change |
|------|--------|
| `scripts/start-librarian-runtime-node.ps1` | `G:\openwork` → `G:\OpenWork` (line 29) |
| `config/model-profiles.json` defaults.binary | `G:\openwork` → `G:\OpenWork` (line 14) |

No lowercase `G:\openwork` remains in any tracked startup/operations file.

### 2. Machine-Local Path Extraction

| File | Change |
|------|--------|
| `runtime/model_manager.ps1` | Added local config override pattern sourcing `config/model_manager.local.ps1` (gitignored) |
| `runtime/model_manager.ps1` | Documented binary mismatch (`llama-server.exe` vs `llama-server-mini.exe`) |
| `config/model-profiles.json` | Added `_meta.custody_note` documenting machine-specific paths |
| `config/model-profiles.local.example.json` | **Created** — example local overrides for model profiles |
| `config/model_manager.local.example.ps1` | **Created** — example local overrides for model manager |

Machine-local paths (`G:\llama.cpp\...`, `G:\llamacpp\...`, `G:\temp\...`) remain only in:
- `config/model-profiles.json` (Rust-router-consumed, documented as machine-specific)
- `config/model-profiles.local.example.json` (example only)
- `config/model_manager.local.example.ps1` (example only)
- `runtime/model_manager.ps1` (defaults, overrideable via local config)

### 3. Port 9122 Collision Resolved

| Port | Profile/Service | File | Status |
|------|----------------|------|--------|
| 9120 | phi-4 | model-profiles.json, model_manager.ps1 | Unchanged |
| 9121 | qwen-coder | model-profiles.json | Unchanged |
| 9122 | llama-3.2 | model-profiles.json | Unchanged (stays) |
| 9122 | ~~embedding~~ | ~~model_manager.ps1~~ | **MOVED to 9125** |
| 9123 | qwen3 | model-profiles.json | Unchanged |
| 9124 | gemma-3 | model-profiles.json | Unchanged |
| 9125 | embedding (moved) | model_manager.ps1 | **New assignment** |
| 9130 | router | launcher + ops scripts | Unchanged |

### 4. Router Port 9130 — Single Source of Truth

All 4 operations scripts (`runtime-start.ps1`, `runtime-stop.ps1`, `runtime-status.ps1`, `runtime-clean-check.ps1`) now use:

```powershell
$RouterPort = if ($env:ROUTER_PORT) { [int]$env:ROUTER_PORT } else { 9130 }
```

The launcher (`start-librarian-runtime-node.ps1`) sets `$env:ROUTER_PORT` as before.

### 5. Backend Binary Documentation

- `config/model-profiles.json`: All 5 profiles use `llama-server.exe` consistently
- `runtime/model_manager.ps1`: Documents that it historically uses `llama-server-mini.exe` (from `G:\llama.cpp\build_vs`) vs the launcher/profiles `llama-server.exe` (from repo `runtime\llama.cpp\`)
- Local config example shows how to override the model_manager binary path

### 6. Regression Validation Tests

| File | Tests |
|------|-------|
| `scripts/tests/test-custody-normalization.py` | 51 tests covering: casing, machine-paths, port collision, router-port source, binary consistency, local config pattern, production boundary |

---

## Files Changed (7 modified, 3 created)

**Modified:**
| File | Change Summary |
|------|---------------|
| `scripts/start-librarian-runtime-node.ps1` | Fixed casing (line 29) |
| `scripts/operations/runtime-start.ps1` | Added `$env:ROUTER_PORT` fallback |
| `scripts/operations/runtime-stop.ps1` | Added `$env:ROUTER_PORT` fallback |
| `scripts/operations/runtime-status.ps1` | Added `$env:ROUTER_PORT` fallback |
| `scripts/operations/runtime-clean-check.ps1` | Added `$env:ROUTER_PORT` fallback |
| `config/model-profiles.json` | Fixed casing in `defaults.binary`, added custody_note |
| `runtime/model_manager.ps1` | EmbedPort 9122→9125, local config override, binary doc, dynamic PID files |
| `scripts/tests/test-startup-files-custody-inventory.py` | Updated for CUSTODY-1 forward compatibility |

**Created:**
| File | Description |
|------|-------------|
| `config/model-profiles.local.example.json` | Example machine-local model profile overrides |
| `config/model_manager.local.example.ps1` | Example machine-local model manager overrides |
| `scripts/tests/test-custody-normalization.py` | Regression validation tests (51 tests) |

---

## Test Results

| Suite | Passed | Failed |
|-------|--------|--------|
| CUSTODY-1 normalization tests | 51 | 0 |
| CUSTODY-0 inventory tests | 45 | 0 |
| **Total** | **96** | **0** |

---

## Closeout Checks

| Check | Result |
|-------|--------|
| Service started? | **No** — Stopped / Manual preserved |
| Models run? | **No** |
| Production router modified? | **No** — router/ and rust-router/ untouched |
| Runtime HTTP semantics changed? | **No** |
| Machine-local config values committed? | **No** — only examples |
| Service state preserved? | **Yes** — Stopped / Manual |
| Orphan processes | **0** |
| Working tree | Clean at commit |

---

## Authoritative Port Map (After Normalization)

| Port | Assignment |
|------|------------|
| 9120 | phi-4 model backend |
| 9121 | qwen-coder model backend |
| 9122 | llama-3.2 model backend |
| 9123 | qwen3 model backend |
| 9124 | gemma-3 model backend |
| 9125 | embedding server (moved from 9122) |
| 9130 | router HTTP |

---

## Local Config Pattern

Two gitignored override files introduced:

| File | Overrideable Values |
|------|-------------------|
| `config/model_manager.local.ps1` | `$ServerPath`, `$ModelsDir`, `$EmbedModelPath`, `$PidDir` |
| `config/model-profiles.local.json` | `defaults.binary`, `defaults.gguf_root`, profile `model_path`, `launch_command` |

Both have tracked `*.example.*` counterparts. The `.gitignore` pattern `config/*.local.*` covers both.

---

## Backend Binary Reconciliation

| Source | Binary | Location |
|--------|--------|----------|
| Launcher (`start-librarian-runtime-node.ps1`) | `llama-server.exe` | `runtime\llama.cpp\` (repo) |
| Model profiles (`config/model-profiles.json`) | `llama-server.exe` | `runtime\llama.cpp\` (repo) |
| Model manager (`runtime/model_manager.ps1`) | `llama-server-mini.exe` | `G:\llama.cpp\build_vs\...` (external) |

**Recommendation for future sprint:** Reconcile to use `llama-server.exe` from the repo's managed `runtime/llama.cpp/` as the sole authoritative backend binary.

---

## Classification

**PROMOTE** — All custody normalization objectives met. Regression tests enforce the resolved state.

**Recommended next sprint:** If continuing on this track, consider `WIN-STARTUP-FILES-CUSTODY-2` for model-profiles.json template extraction or backend binary reconciliation.
