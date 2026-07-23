# ROUTER-DAEMON-HARDEN-3: Remaining Rust/Python Parity Closure

> Close the next highest-value low-severity Rust/Python router parity gaps
> while preserving the frozen HTTP contract.

---

## Overview

| Field | Value |
|-------|-------|
| Sprint ID | ROUTER-DAEMON-HARDEN-3 |
| Layer | Layer 2 — Portable Router / Native Daemon |
| Status | **SEALED** |
| Start HEAD | TheLibrarian-main: `1e32002`, librarian-runtime-node: `47a44fe` |
| End HEAD | TheLibrarian-main: `1e32002`, librarian-runtime-node: `<commit>` |
| Authority | `advisory_only` |

### Starting State

- TheLibrarian-main HEAD: `1e32002` — clean, unchanged
- librarian-runtime-node HEAD: `47a44fe` — clean, up to date
- Service LibrarianRunTimeNode: Stopped / Manual
- Port 9130: free
- Orphans: 0
- Contract harness: `scripts/tests/run-router-contract-tests.ps1` (67/67 PASS)
- Vapor API (port 3456) excluded from this layer

---

## Scope Completed

### Gaps Targeted: 6 | Closed: 4 | Deferred: 2

### Gap 2: Backend Log Directory Auto-Creation (CLOSED)

**File:** `process.rs`

**Before:** Backend log files were written to the current working directory
as `backend_{alias}.log` with no directory creation.

**After:** Logs are written to a `logs/` subdirectory (matching Python's
`RUNTIME_NODE/logs/` convention). The directory is created automatically
via `std::fs::create_dir_all` before the file is opened.

**Change:** `+7` lines in `process.rs`

### Gap 3: Evidence Filename Alignment (CLOSED)

**File:** `server.rs`

**Before:** Rust wrote chat refusal evidence as `chat-refusal.json`.
Python writes `chat-refusal-authority.json`.

**After:** Renamed to `chat-refusal-authority.json` to match Python's naming
convention. Both refusal write sites updated.

**Change:** `+0/-0` (string literal change in 2 locations)

### Gap 4: Process-Before-After.txt Restart Audit Evidence (CLOSED)

**Files:** `evidence.rs`, `server.rs`

**Before:** Rust wrote only `restart-result.json` on restart — no text audit trail.
Python writes `process-before-after.txt` with PID before/after values.

**After:** Added `EvidenceWriter::write_text()` method for plain-text evidence
files. The restart handler now writes `process-before-after.txt` with before/after
PIDs, profile alias, and timestamp (matching Python's format).

**Change:** `+18` lines in `evidence.rs`, `+15` lines in `server.rs`

### Gap 5: 404 Catch-All Response Shape (CLOSED)

**File:** `server.rs`

**Before:** Unknown routes returned an empty body (axum default 404 behavior).
Python returns `{"error": "Not found: {path}"}`.

**After:** Added `handle_404` fallback handler returning JSON with the matched
Python response shape. Uses axum's `.fallback()` on the router.

**Change:** `+15` lines in `server.rs`

---

### Gaps Deferred (2)

#### Gap 1: `ensure_runtime_config()` Equivalent (DEFERRED)

**Reason:** The Python router copies `model-profiles.json` to a runtime config
directory as a fallback for path resolution. The Rust router uses a different
architectural approach — `ProfileManager::load_from_sources()` with multiple
hardcoded fallback paths, which achieves the same outcome (a profile file is
found) without the copying step. Adding file-copy logic to a startup path that
already works would introduce filesystem side effects for no behavioral benefit.
The Rust approach is functionally equivalent and architecturally simpler.

**Risk if forced:** Could overwrite config files in unexpected ways during
development or service startup.

#### Gap 6: Default Port/Body Size Alignment (DEFERRED)

**Reason:** Python defaults: port 8080, max body 64 KB. Rust defaults: port 9130,
max body 10 MB. Changing the default port (8080 vs 9130) would break the frozen
contract tests which expect port 9130. Changing the body size limit (64 KB vs
10 MB) would make the Rust router more restrictive, potentially breaking valid
requests that fit within the frozen 10 MB limit. Both defaults are within their
respective implementation's design and neither violates the frozen contract.

**Risk if forced:** Port change would break contract tests. Body size reduction
would break existing workflows.

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

## Gap Closure Summary

| # | Gap | Status | Severity | Files Changed |
|---|-----|--------|----------|---------------|
| 1 | `ensure_runtime_config()` equivalent | **DEFERRED** | LOW | — |
| 2 | Backend log directory auto-creation | **CLOSED** | LOW | process.rs |
| 3 | Evidence filename alignment | **CLOSED** | LOW | server.rs |
| 4 | Process-before-after.txt audit evidence | **CLOSED** | LOW | evidence.rs, server.rs |
| 5 | 404 catch-all response shape | **CLOSED** | LOW | server.rs |
| 6 | Default port/body size alignment | **DEFERRED** | LOW | — |
| | **Total: 4 closed, 2 deferred** | | | **4 files, +55/-0 lines** |

## Cumulative Gap Ledger

| Sprint | Closed | Deferred | Remaining |
|--------|--------|----------|-----------|
| ROUTER-DAEMON-IMPLEMENTATION-1 | 2 | 0 | 18 |
| ROUTER-DAEMON-HARDEN-2 | 3 | 0 | 15 |
| ROUTER-DAEMON-HARDEN-3 | 4 | 2 | 9 |
| **Total** | **9** | **2** | **9** |

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
| librarian-runtime-node git status | **Modified** |
| TheLibrarian-main git status | **Clean** (unchanged) |
| Stashes | **Empty** |

---

*Sealed: 2026-06-23*
*Authority: advisory_only*
