# WIN-STARTUP-FILES-CUSTODY-0

## Sprint Closeout

**Date:** 2026-06-28
**Starting HEAD:** c38fe8b
**Final HEAD:** *pending commit*
**Sprint Type:** Inventory / Planning / Custody-Boundary (no behavioral changes)

---

## Starting Checks

| Check | Result |
|-------|--------|
| HEAD | c38fe8b |
| Working tree | Clean |
| Service | LibrarianRunTimeNode — Stopped / Manual |
| Orphans | 0 |
| NSSM | Not available in PATH (non-admin shell) |

---

## Deliverables

| Artifact | Path | Status |
|----------|------|--------|
| Machine-readable inventory | `reports/startup-files-custody-inventory.json` | Created — 70+ values classified |
| Human-readable report | `reports/WIN-STARTUP-FILES-CUSTODY-0.md` | Created |
| Planning document | `docs/planning/WIN-STARTUP-FILES-CUSTODY-0.md` | Created |
| Sprint closeout | `docs/sprints/WIN-STARTUP-FILES-CUSTODY-0.md` | Created |
| Custody manifest example | `fixtures/startup-files-custody/startup-custody-manifest.example.json` | Created |
| Machine-local config example | `fixtures/startup-files-custody/machine-local-config.example.json` | Created |
| Inventory validation test | `scripts/tests/test-startup-files-custody-inventory.py` | Created |

---

## Key Findings

### Path Casing Drift

| File | Casing | 
|------|--------|
| `scripts/start-librarian-runtime-node.ps1` | `G:\openwork` (lowercase) |
| `scripts/operations/runtime-*.ps1` (3 files) | `G:\OpenWork` (camelCase) |
| `scripts/test-win-rust-service-swap.ps1` | `G:\OpenWork` (camelCase) |
| `config/runtime-node.local.json` (gitignored) | `G:\openwork\thelibrarian` (third variant) |

### Port Collision

Port **9122** is assigned to **llama-3.2** in `config/model-profiles.json` AND to the **embedding server** in `runtime/model_manager.ps1`. These cannot run simultaneously.

### Machine-Specific Git-Tracked Paths

`config/model-profiles.json` and `runtime/model_manager.ps1` contain `G:\llama.cpp\...`, `G:\openwork\...`, and `G:\llamacpp\...` paths — all tracked in git, all machine-specific.

### Binary Mismatch

Launcher + profiles use `llama-server.exe` from repo's `runtime\llama.cpp\`. Model manager uses `llama-server-mini.exe` from `G:\llama.cpp\build_vs\...`. These are different binaries from different builds.

---

## Risk Register Highlights

| # | Risk | Level |
|---|------|-------|
| R1 | Machine-local absolute paths in git-tracked files | Critical |
| R2 | Path casing drift (openwork vs OpenWork) | High |
| R3 | Port 9122 collision (llama-3.2 vs embedding) | Critical |
| R4 | Router port 9130 hardcoded in 7+ locations | High |
| R5 | NSSM config not tracked in git | Medium |
| R6 | Binary path mismatch (llama-server vs llama-server-mini) | Critical |
| R7 | Embedding model uses different namespace | High |
| R8 | Cross-repo reference to TheLibrarian-main | High |
| R9 | Shared values defined independently | High |
| R10 | No custody verification before service start | Medium |

---

## Classification

**PROMOTE** — Inventory complete. Ready for WIN-STARTUP-FILES-CUSTODY-1.

**Recommended next sprint:** Normalize path casing, add env-var-driven operations scripts, create custody manifest, extract machine-local paths.
