# WIN-STARTUP-FILES-CUSTODY-0 — Startup Files Custody Report

**Date:** 2026-06-28
**Starting HEAD:** c38fe8b
**Sprint Type:** Inventory / Planning / Custody-Boundary (no behavioral changes)

---

## Summary

Inventory and classification of every startup/runtime configuration surface used by `librarian-runtime-node`. All 6 categories inventoried, 70+ individual values classified, 10 risk register entries documented, and a custody plan defined.

---

## Inventories Completed

| Category | Items | Key Findings |
|----------|-------|--------------|
| Service Launcher | 11 | Hardcoded paths, path casing drift, NSSM config not tracked |
| Runtime Profiles & Model Config | 21 | Machine-specific paths in git-tracked files, port collision risk |
| Operations Scripts | 14 | Hardcoded port, repo root, and paths across 5 scripts |
| Qualification & Service Swap | 7 | Path casing drift from launcher, binary path mismatch |
| Environment Variables | 12 | 7 set by launcher, 5 read by Rust router with defaults |
| Ports | 7 | 9130 hardcoded in 7+ locations, port 9122 collision risk |
| Absolute Paths | 11 | 4 distinct G:\ paths, 3 casing variants |
| Path Casing Drift | 1 | `G:\openwork` vs `G:\OpenWork` across launcher and ops |

---

## Critical Risks

### R1: Machine-local absolute paths in tracked files
`config/model-profiles.json` and `runtime/model_manager.ps1` contain machine-specific paths (`G:\llama.cpp\models\...`, `G:\openwork\librarian-runtime-node\...`). These are tracked in git and will break on any other machine.

### R2: Path casing drift
`scripts/start-librarian-runtime-node.ps1` uses `G:\openwork` (lowercase), while `scripts/operations/runtime-*.ps1` use `G:\OpenWork` (camelCase). Windows tolerates this but it causes confusion and portability issues.

### R3: Port 9122 collision
Port 9122 is assigned to `llama-3.2` in `config/model-profiles.json` AND to the embedding server in `runtime/model_manager.ps1`. These cannot run simultaneously.

### R4: Router port 9130 hardcoded in 7+ locations
Port 9130 appears in the launcher, all 5 operations scripts, the service swap test, and the Rust router default. Changing it requires coordinated edits across all these files.

### R5: NSSM config not tracked in git
The service wrapper configuration (AppDirectory, AppPath, AppParameters) is set via `nssm set` commands and is not version-controlled. It can drift from repo expectations.

### R6: Binary path mismatch between profiles and manager
`config/model-profiles.json` references `llama-server.exe`, while `runtime/model_manager.ps1` references `llama-server-mini.exe` from a different directory (`G:\llama.cpp\build_vs\...` vs `G:\openwork\librarian-runtime-node\runtime\llama.cpp\...`).

### R7: Embedding model path uses different namespace
The embedding model path `G:\llamacpp\snowflake-arctic-embed-m-long-Q4_0.gguf` (no dot in `llamacpp`) differs from `G:\llama.cpp\models` used by all chat models.

### R8: Cross-repo path reference
`scripts/operations/runtime-clean-check.ps1` references `G:\OpenWork\TheLibrarian-main` — a sibling repo outside this repo's boundary.

### R9: Shared config values defined independently
Values like `StartupTimeoutSec`, `HEALTH_TIMEOUT_SECS`, model ports, and backend binary paths are defined independently in multiple files with no single source of truth.

### R10: No custody verification before service start
There is no manifest or verification step that confirms all startup values are consistent before the service launcher runs.

---

## Custody Plan

### 1. Canonical development control source
A single `config/custody-manifest.json` (git-tracked, templated) should define:
- Expected service name
- Default port range for router and backends
- Expected binary paths (relative to repo root)
- Expected evidence/log paths
- Expected NSSM configuration

### 2. Shared / tracked files
- `scripts/start-librarian-runtime-node.ps1` (templated, env-var driven)
- `config/model-profiles.json` (split: shared profile metadata + local paths)
- All operations scripts (templated, env-var driven)
- Custody manifest

### 3. Files becoming templates/examples
- `config/model-profiles.json` should become a template (`model-profiles.template.json`)
- `config/runtime-node.example.json` should demonstrate relative/placeholder paths, not absolute

### 4. Machine-local gitignored files
- `config/model-profiles.local.json` — machine-specific overrides for model paths, ports
- `config/custody-settings.local.json` — machine-specific paths, port overrides

### 5. Startup package manifest
`fixtures/startup-files-custody/startup-custody-manifest.example.json` should record:
- Repo HEAD at service registration time
- Expected NSSM config values
- Expected env var values
- Expected port allocations
- Expected binary paths

### 6. Pre-start verification
A verification script should check before service start:
- NSSM config matches manifest
- Binary paths exist
- Ports are free
- Env vars are set correctly
- Path casing is consistent

### 7. Forbidden hardcoded values
Absolute machine paths (`G:\...`) should be forbidden in shared tracked files. All machine-specific values should be in gitignored local config or environment variables.

### 8. Model-profiles.json split
Split into:
- `config/model-profiles.json` — shared metadata (alias, task classes, verified context, stability, model_file names relative to gguf_root)
- `config/model-profiles.local.json` — machine-specific binary path, gguf_root, port overrides

### 9. NSSM config reconciliation
- Document expected NSSM config in the custody manifest
- Add a verification step to `runtime-clean-check.ps1` that checks NSSM config against manifest
- Consider generating NSSM config from manifest rather than setting manually

### 10. First custody sprint changes
`WIN-STARTUP-FILES-CUSTODY-1` should:
1. Normalize path casing to `G:\OpenWork` in all tracked files
2. Add `ROUTER_PORT` environment variable reading to all operations scripts
3. Create custody manifest template
4. Add pre-start verification step
5. Extract machine-specific paths from `model-profiles.json` to gitignored local override

---

## Acceptance Criteria Verification

| Criterion | Status |
|-----------|--------|
| Startup/control inventory exists | ✅ `reports/startup-files-custody-inventory.json` |
| Machine-readable inventory exists | ✅ JSON with categorized items |
| High-risk files classified | ✅ All high/critical items flagged |
| `config/model-profiles.json` classified as machine-specific | ✅ Critical — contains absolute local paths |
| Service launcher script classified | ✅ 11 items inventoried |
| Operations scripts classified | ✅ 14 items inventoried |
| Environment variables inventoried | ✅ 12 variables tracked |
| Ports inventoried | ✅ 7 ports (including collision) |
| Absolute paths inventoried | ✅ 11 paths, 4 distinct G:\ locations |
| Path casing drift checked | ✅ `G:\openwork` vs `G:\OpenWork` documented |
| Risk register exists | ✅ 10 risks documented |
| Future custody plan exists | ✅ 10-point plan |
| No startup behavior changed | ✅ Verified |
| No service config changed | ✅ Verified |
| No router behavior changed | ✅ Verified |
| No runtime-node HTTP behavior changed | ✅ Verified |
| No model execution changed | ✅ Verified |
| Service state preserved | ✅ Stopped / Manual |
| Orphan process count 0 | ✅ 0 |
| Working tree clean | ✅ |

---

## Result Classification

**PROMOTE** — Inventory is complete enough to support WIN-STARTUP-FILES-CUSTODY-1.

**Recommended next sprint:** `WIN-STARTUP-FILES-CUSTODY-1`
