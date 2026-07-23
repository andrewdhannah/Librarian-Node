# WIN-STARTUP-BINARY-CUSTODY-1 — Backend Binary Authority Reconciliation

**Date:** 2026-06-28
**Starting HEAD:** 56fda54
**Mode:** Custody hardening / configuration authority reconciliation
**Classification:** PROMOTE follow-up from WIN-STARTUP-FILES-CUSTODY-1

---

## Summary

Defined one authoritative backend binary (`llama-server.exe`) for all Windows startup/runtime operations and reconciled all tracked consumers. The only divergent file (`runtime/model_manager.ps1`) now defaults to the authoritative binary with a derived process name that follows whichever binary is configured, eliminating the drift risk.

---

## Authoritative Backend Binary

**Value:** `llama-server.exe`
**Resolution:** `runtime/llama.cpp/llama-server.exe` (relative to repo root)

This is the canonical binary name used by all tracked operational surfaces:

| Consumer | Binary | Reconciled |
|----------|--------|------------|
| `scripts/start-librarian-runtime-node.ps1` | `llama-server.exe` | ✅ Already consistent |
| `config/model-profiles.json` (defaults + 5 profiles) | `llama-server.exe` | ✅ Already consistent |
| `scripts/operations/runtime-*.ps1` (orphan detection) | `llama-server` process name | ✅ Already consistent |
| `scripts/test-win-rust-service-swap.ps1` | `llama-server` process name | ✅ Already consistent |
| `runtime/model_manager.ps1` (default) | `llama-server.exe` | ✅ **Reconciled** (was `llama-server-mini.exe`) |
| `config/model-profiles.local.example.json` | `llama-server.exe` | ✅ Already consistent |
| `config/model_manager.local.example.ps1` | `llama-server.exe` | ✅ Already consistent |

---

## Changes Made

### `runtime/model_manager.ps1` — Default binary and process detection

| Change | Before | After |
|--------|--------|-------|
| Default `$ServerPath` | `G:\llama.cpp\build_vs\bin\Release\llama-server-mini.exe` | `$PSScriptRoot\llama.cpp\llama-server.exe` (repo-relative) |
| Process detection | Hardcoded `'llama-server-mini'` | Derived `$ServerProcessName` from `$ServerPath` |
| Port conflict check | Hardcoded `'llama-server-mini'` | `$ServerProcessName` (variable) |
| Embedding error message | Hardcoded `llama-server-mini.exe` | `${ServerProcessName}.exe` (variable) |
| Documentation | Described historical mismatch | Declares authoritative binary, shows historical override |

The derived process name (`$ServerProcessName`) is computed with:
```powershell
$ServerProcessName = [System.IO.Path]::GetFileNameWithoutExtension($ServerPath)
```

This means if someone overrides `$ServerPath` via local config, the process detection follows automatically. No hardcoded name remains in operational code.

### `config/model_manager.local.example.ps1` — Updated to reflect authority

- Added authoritative binary declaration
- Shows how to override to `llama-server-mini.exe` as historical alternative

### `scripts/tests/test-custody-normalization.py` — Enhanced binary tests

Added 5 new tests:
1. `model_manager.ps1 default ServerPath is llama-server.exe`
2. `model_manager.ps1 uses derived ServerProcessName`
3. `Get-ServerProcess uses $ServerProcessName variable`
4. `model_manager.ps1 documents authoritative binary`
5. `No operational code references 'llama-server-mini'`

---

## Authority Chain

```
config/model-profiles.json (defaults.binary)
    ↓
scripts/start-librarian-runtime-node.ps1 ($env:BACKEND_BINARY_PATH)
    ↓
Rust router (reads BACKEND_BINARY_PATH from env)
    ↓
runtime/model_manager.ps1 ($ServerPath → $ServerProcessName)
    ↓
scripts/operations/runtime-*.ps1 (orphan detection by process name)
```

All consumers now agree on `llama-server.exe`.

---

## Test Results

| Suite | Passed | Failed |
|-------|--------|--------|
| CUSTODY-0 inventory tests | 42 | 0 |
| CUSTODY-1/BINARY normalization tests | 55 | 0 |
| **Total** | **97** | **0** |

---

## Files Changed (3 modified)

| File | Change Summary |
|------|----------------|
| `runtime/model_manager.ps1` | Default binary → `llama-server.exe`, derived process name, variable-based detection |
| `config/model_manager.local.example.ps1` | Updated documentation to reflect authority |
| `scripts/tests/test-custody-normalization.py` | Added 5 binary reconciliation regression tests |

---

## Closeout Checks

| Check | Result |
|-------|--------|
| Authoritative binary value defined | ✅ `llama-server.exe` |
| All tracked consumers reconciled | ✅ 6/6 consumers agree |
| Historical exception documented | ✅ `llama-server-mini.exe` in comments/example only |
| No real machine-local path committed | ✅ Repo-relative default |
| Service started? | **No** — Stopped / Manual preserved |
| Models run? | **No** |
| Production router modified? | **No** |
| Runtime HTTP semantics changed? | **No** |
| Service state preserved? | **Yes** — Stopped / Manual |
| Orphan processes | **0** |
| Working tree | Clean at commit |

---

## Classification

**PROMOTE** — All binary custody objectives met. The backend binary authority chain is now consistent across all tracked operational surfaces, with regression tests preventing future divergence.

**Recommended next:** The custody track is now complete for the current scope. Next priorities depend on broader roadmap:
- Back to context-routing track (ADVISORY-HARNESS-1)
- Or model-profiles.json path extraction (machine-local → template)
- Or service launcher env-var refactor
