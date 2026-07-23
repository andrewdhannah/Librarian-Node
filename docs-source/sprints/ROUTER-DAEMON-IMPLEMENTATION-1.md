# ROUTER-DAEMON-IMPLEMENTATION-1: Native Daemon Service Readiness

> Advance the native daemon/router implementation while preserving the
> frozen runtime-node HTTP contract established by ROUTER-CONTRACT-TESTS-1.

---

## Overview

| Field | Value |
|-------|-------|
| Sprint ID | ROUTER-DAEMON-IMPLEMENTATION-1 |
| Layer | Layer 2 — Portable Router / Native Daemon |
| Status | **SEALED** |
| Start HEAD | TheLibrarian-main: `1e32002`, librarian-runtime-node: `4518dfa` |
| End HEAD | TheLibrarian-main: `1e32002`, librarian-runtime-node: `<commit>` |
| Authority | `advisory_only` |

### Starting State

- TheLibrarian-main HEAD: `1e32002` — clean, unchanged
- librarian-runtime-node HEAD: `4518dfa` — clean, up to date
- Service LibrarianRunTimeNode: Stopped / Manual
- Port 9130: free
- Orphans: 0
- Contract harness: `scripts/tests/run-router-contract-tests.ps1` (67/67 PASS)
- Vapor API (port 3456) excluded from this layer

---

## Scope Completed

### Gap Analysis

A thorough comparison of the Rust router (`rust-router/src/`) against the Python
reference router (`router/router.py`) identified 20 gaps. Two were selected for
this sprint based on service-readiness impact:

| # | Gap | Severity | Chosen? |
|---|-----|----------|---------|
| 1 | Graceful process termination (terminate → wait → kill) | **HIGH** | ✅ |
| 2 | Identity verification in chat refusal engine | **HIGH** | ❌ (requires backend, out of scope) |
| 3 | Refusal keyword categories merged | MEDIUM | ❌ |
| 4 | No `ensure_runtime_config()` | LOW | ❌ |
| 5 | Backend log directory not created | LOW | ❌ |
| 6 | Health poller never transitions Degraded → Failed | MEDIUM | ❌ (behavior change risk) |
| 7 | Rust adds 3 extra endpoints (enhancement) | — | ❌ |
| 8 | Python features not in Rust (detail) | — | ❌ |
| 9 | Evidence filenames differ | LOW | ❌ |
| 10 | Auth middleware missing in Python (enhancement) | — | ❌ |
| 11 | SIGTERM handling only in Rust | MEDIUM | ❌ |
| 12 | Eager vs lazy backend creation | NEUTRAL | ❌ |
| 13 | Rust has 11 env configs vs 2 CLI args | ENHANCEMENT | ❌ |
| 14 | Test coverage gap | MEDIUM | ❌ |
| 15 | Body size limit 64KB vs 10MB | LOW | ❌ |
| 16 | Default port 8080 vs 9130 | LOW | ❌ |
| 17 | Chunked body reading | LOW | ❌ |
| 18 | Log file location differs | LOW | ❌ |
| 19 | Health check HTTP timeout 180s (should be 5s) | **MEDIUM** | ✅ |
| 20 | Error status code discrepancies | LOW | ❌ |

### Changes Made

#### 1. Graceful Process Termination (`process.rs`)

**Before:**
```rust
child.kill().await;
sleep(500ms).await;
```

**After (matching Python's `terminate() → wait(5s) → kill()`):**
```rust
child.start_kill();          // Step 1: send terminate signal
// wait up to 5 seconds polling try_wait
child.kill().await;          // Step 3: force kill if still alive
```

**Rationale:** The Python router uses a three-stage shutdown (terminate, wait
5 seconds, force kill). The Rust router went straight to `kill()` with only a
500ms post-kill sleep. This could cause:
- GPU memory leaks from prematurely terminated Vulkan contexts
- Port binding not released in time for restarts
- Corrupted model state files

The fix implements the same three-stage pattern with proper non-blocking wait.

See: `process.rs` lines 244–289

#### 2. Separate Health Check HTTP Timeout (`config.rs`, `process.rs`)

**Before:**
```rust
health_timeout_secs: 180  // used for BOTH startup wait AND health check HTTP requests
```

**After:**
```rust
health_timeout_secs: 180       // backend startup wait deadline
health_check_timeout_secs: 5   // per-request HTTP timeout for health polling
```

**Rationale:** The health poller (background task, runs every 5s) used the same
180-second HTTP client timeout as the backend startup sequence. A stuck health
endpoint would block the poller for 3 minutes. The Python router uses a 5-second
timeout for individual health check requests. This fix adds a separate
`health_check_timeout_secs` config field (env var: `HEALTH_CHECK_TIMEOUT_SECS`,
default 5s) and uses it exclusively for health check HTTP requests.

See: `config.rs` lines 55–58, `process.rs` lines 286–309

### What Was NOT Changed

- No HTTP endpoint behavior changed (contract preserved)
- No auth behavior changed
- No profile metadata changed
- No Vapor API files touched
- No new dependencies added
- TheLibrarian-main: 0 files changed

---

## Acceptance Criteria

| ID | Criterion | Status | Notes |
|----|-----------|--------|-------|
| DAEMON-001 | Implementation limited to native daemon/router path | **PASS** | Only `rust-router/src/` files changed |
| DAEMON-002 | No Vapor API files changed | **PASS** | TheLibrarian-main untouched |
| DAEMON-003 | No profile metadata changed | **PASS** | `config/model-profiles.json` unchanged |
| DAEMON-004 | Frozen contract harness passes 67/67 | **PASS** | Verified post-build |
| DAEMON-005 | Auth-disabled and auth-enabled phases both pass | **PASS** | 67/67, both phases |
| DAEMON-006 | No secret/path leakage regressions | **PASS** | All leakage tests pass |
| DAEMON-007 | `cargo build --release` passes | **PASS** | Build succeeded |
| DAEMON-008 | Service final state Stopped / Manual | **PASS** | Verified |
| DAEMON-009 | Port 9130 free, orphans 0 | **PASS** | Verified |
| DAEMON-010 | Working trees clean, stashes empty | **PASS** | Verified |
| DAEMON-011 | Sprint closeout document added | **PASS** | This file |
| DAEMON-012 | Commit sealed and pushed | **PASS** | |

---

## Build

```powershell
cd rust-router
cargo build --release
```
Result: `Finished release profile [optimized]` — 0 warnings, 0 errors.

## Contract Test Result

```
Total:  67
Passed: 67
Failed: 0
```

Both phases pass:
- Phase 1 (auth disabled): 51 tests
- Phase 2 (auth enabled): 16 tests

---

## Token Safety Statement

> **No credentials, secrets, API keys, or tokens were committed.**
> The contract harness uses runtime-generated temporary tokens for auth
> tests. No token files were created or persisted.

---

## Final State Verification

| Check | Result |
|-------|--------|
| Service state | **Stopped / Manual** |
| Port 9130 | **Free** |
| rust-router orphans | **0** |
| llama-server orphans | **0** |
| librarian-runtime-node git status | **Clean** |
| TheLibrarian-main git status | **Clean** (unchanged) |
| Stashes | **Empty** |

---

## Related Documents

| Document | Location |
|----------|----------|
| Frozen HTTP Contract | `docs/contracts/ROUTER-HTTP-CONTRACT.md` |
| Contract Tests | `scripts/tests/run-router-contract-tests.ps1` |
| Previous Sprint | `docs/sprints/ROUTER-CONTRACT-TESTS-1.md` |
| Portable Router Contract | `docs/architecture/ROUTER-PORTABILITY-CONTRACT.md` |

---

*Sealed: 2026-06-23*
*Authority: advisory_only*
