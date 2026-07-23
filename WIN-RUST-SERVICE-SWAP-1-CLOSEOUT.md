# WIN-RUST-SERVICE-SWAP-1 Closeout Report

## Sprint Summary
- **Sprint**: WIN-RUST-SERVICE-SWAP-1
- **Repository**: librarian-runtime-node
- **Starting HEAD**: 0465937 (RUNTIME-NODE-PUSH-AND-BASELINE-1 sealed)
- **Date**: 2026-06-21
- **Status**: Ready for proof execution (requires admin)

---

## Objective

Make the NSSM service (`LibrarianRunTimeNode`) use the Rust router as the primary service path, while preserving the Python router as a startup fallback. Narrow scope: service wrapper swap, start/select/chat/stop proof, fallback documented — no new router features.

---

## Files Changed

### Modified
- `scripts/start-librarian-runtime-node.ps1` — Service launcher updated to try Rust router first, Python router as startup fallback

### New
- `scripts/test-win-rust-service-swap.ps1` — Automated proof: starts service, runs health/status/select/chat/stop lifecycle, stops service, reports results
- `scripts/run-win-rust-service-swap-proof.ps1` — Admin-elevated runner that orchestrates the full proof (requires admin for service control)
- `WIN-RUST-SERVICE-SWAP-1-CLOSEOUT.md` — This report

---

## What Changed in the Service Launcher

### Before (Python-only)
```powershell
$WorkDir = "G:\openwork\librarian-runtime-node"
Set-Location -LiteralPath $WorkDir
& "C:\Python314\python.exe" -u "G:\openwork\librarian-runtime-node\router\router.py" --port 9130
```

### After (Rust primary, Python fallback)

The updated `start-librarian-runtime-node.ps1` has two phases:

**Phase 1 — Rust Router (Primary):**
1. Sets `ROUTER_PORT`, `LOG_PATH`, `EVIDENCE_PATH`, `BACKEND_BINARY_PATH`, `HEALTH_POLL_INTERVAL_SECS`, `HEALTH_TIMEOUT_SECS` environment variables
2. Launches `rust-router.exe --port 9130` in the foreground via `&` (call operator)
3. If Rust router exits with code 0 (clean shutdown requested by NSSM), service stops cleanly
4. If Rust router exits with non-zero (startup failure: port conflict, bad config, missing binary), logs warning and falls through to Phase 2

**Phase 2 — Python Router (Fallback):**
1. Launches Python router identically to the previous behavior
2. Only reached if Rust router is absent or fails to start
3. All original behavior preserved

### Startup Fallback Conditions

| Condition | Behavior |
|-----------|----------|
| Rust binary missing | Log → fall back to Python |
| Rust binary fails to start (exit code != 0) | Log → fall back to Python |
| Rust binary runs and handles traffic | Active service path |
| Rust binary exits during runtime (crash) | NSSM restarts service → retries Rust → falls back to Python |
| NSSM sends stop signal | Rust router graceful shutdown (backends stopped, clean exit 0) |

---

## NSSM Configuration (Unchanged)

The NSSM service configuration was **not modified**. It still points to:
```
Application:  powershell.exe
Parameters:   -NoProfile -ExecutionPolicy Bypass -File scripts\start-librarian-runtime-node.ps1
Directory:    G:\openwork\librarian-runtime-node
Start type:   SERVICE_DEMAND_START (Manual)
```

The same script now handles both Rust-primary and Python-fallback paths.

---

## Proof Procedure

The proof requires admin privileges (for NSSM service control). Steps:

```powershell
# 1. Open an elevated PowerShell:
Start-Process powershell -Verb RunAs

# 2. Run the admin proof runner:
cd G:\OpenWork\librarian-runtime-node
.\scripts\run-win-rust-service-swap-proof.ps1
```

The proof runner:
1. Cleans stale processes
2. Verifies NSSM config and launcher script
3. Runs `test-win-rust-service-swap.ps1` which:
   - **Step 1**: Start service → wait for /health (Rust router)
   - **Step 2**: Verify identity (authority: advisory_only, profiles loaded)
   - **Step 3**: Select phi-4 profile → backend starts
   - **Step 4**: Send chat via Rust router proxy
   - **Step 5**: Stop backend
   - **Step 6**: Stop service
   - **Step 7**: Check orphans and evidence
4. Collects logs and evidence
5. Reports pass/fail

---

## Expected Results

| Check | Expected |
|-------|----------|
| Service starts via NSSM | LibrarianRunTimeNode → Running |
| Rust router responds on :9130 | `GET /health` → 200 |
| Authority | `advisory_only` |
| Profiles loaded | 5 profiles (phi-4, qwen-coder, llama-3.2, qwen3, gemma-3) |
| Backend select | phi-4 → `status: "selected"`, backend healthy |
| Chat | `status: "ok"`, `content: "OK"` |
| Backend stop | `status: "stopped"` |
| Service stop | Stopped, no orphan processes |
| Evidence files | Written to `fixtures/windows-runtime-node/router-impl/` |

---

## Boundaries

- **No NSSM config changed** — same script, updated behavior
- **No new Rust router features** — only the service launcher wrapper
- **No Python router code changed** — retained as reference/fallback
- **No model files, binaries, or logs committed**

---

## Acceptance Gate

| Gate | Status |
|------|--------|
| ✅ Service wrapper swapped to Rust router primary | **IMPLEMENTED** |
| ⏳ Start/select/chat/stop proof via NSSM | **Pending admin execution** |
| ✅ Fallback documented | **DONE** |
| ✅ No new router features | **DONE** |
| ✅ Clean git status (source files only) | **READY** |
