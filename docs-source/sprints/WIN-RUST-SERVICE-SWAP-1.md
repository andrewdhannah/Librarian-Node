# Sprint: WIN-RUST-SERVICE-SWAP-1
**Status:** PENDING PROOF (requires admin for service control)
**Date:** 2026-06-21

## Objective
Make the NSSM service (`LibrarianRunTimeNode`) use the Rust router as the primary service path, while preserving the Python router as a startup fallback. Keep it narrow: service wrapper swap, start/select/chat/stop proof, fallback documented, no new router features.

## Starting State
- **HEAD**: `0465937` (RUNTIME-NODE-PUSH-AND-BASELINE-1 sealed)
- **Tree**: Clean
- **Service**: `LibrarianRunTimeNode` — Stopped, Manual

## Implementation

### Changed: `scripts/start-librarian-runtime-node.ps1`
The NSSM service launcher now tries the Rust router first:

**Phase 1 — Rust Router (Primary):**
- Sets env vars (`ROUTER_PORT`, `LOG_PATH`, `EVIDENCE_PATH`, `BACKEND_BINARY_PATH`, etc.)
- Launches `rust-router.exe --port 9130` in foreground
- Exit code 0 → clean shutdown (service stopping)
- Non-zero exit → logs warning, falls through to Phase 2

**Phase 2 — Python Router (Fallback):**
- Unchanged from previous behavior
- Only reached if Rust router is absent or fails to start

### New: `scripts/test-win-rust-service-swap.ps1`
Automated proof script: starts service, verifies health/status/profiles, selects phi-4 backend, sends chat, stops backend, stops service.

### New: `scripts/run-win-rust-service-swap-proof.ps1`
Admin-elevated runner that cleans stale processes, runs the proof, and collects evidence.

## Acceptance Gates

| Gate | Result |
|------|--------|
| Service wrapper swapped to Rust router primary | **IMPLEMENTED** |
| Fallback to Python on Rust startup failure | **IMPLEMENTED** |
| Start/select/chat/stop proof via NSSM | ⏳ **Pending admin execution** |
| Fallback documented | **DONE** |
| No new router features | **DONE** |
| Clean git status (source files only) | **READY** |

## NSSM Configuration (Unchanged)
```
Application:  powershell.exe
Parameters:   -NoProfile -ExecutionPolicy Bypass -File scripts\start-librarian-runtime-node.ps1
Directory:    G:\openwork\librarian-runtime-node
Stdout:       logs\service-stdout.log
Stderr:       logs\service-stderr.log
Start type:   SERVICE_DEMAND_START (Manual)
```
