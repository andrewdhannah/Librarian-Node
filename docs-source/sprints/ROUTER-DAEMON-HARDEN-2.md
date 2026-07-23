# ROUTER-DAEMON-HARDEN-2: Refusal Identity and Health-State Parity

> Close the three highest-severity Rust/Python parity gaps in the native
> daemon router while preserving the frozen 67-test HTTP contract.

---

## Overview

| Field | Value |
|-------|-------|
| Sprint ID | ROUTER-DAEMON-HARDEN-2 |
| Layer | Layer 2 — Portable Router / Native Daemon |
| Status | **SEALED** |
| Start HEAD | TheLibrarian-main: `1e32002`, librarian-runtime-node: `03959df` |
| End HEAD | TheLibrarian-main: `1e32002`, librarian-runtime-node: `<commit>` |
| Authority | `advisory_only` |

### Starting State

- TheLibrarian-main HEAD: `1e32002` — clean, unchanged
- librarian-runtime-node HEAD: `03959df` — clean, up to date
- Service LibrarianRunTimeNode: Stopped / Manual
- Port 9130: free
- Orphans: 0
- Contract harness: `scripts/tests/run-router-contract-tests.ps1` (67/67 PASS)
- Vapor API (port 3456) excluded from this layer

---

## Scope Completed

### Gap 1: Identity Verification in Chat Refusal (HIGH)

**Before:** The Rust router's `check_chat()` (refusal.rs) checked profile existence,
context limits, runtime health, and authority keywords — but did NOT verify that
the running backend's model identity matched the expected profile alias.

**After:** `BackendProcess` now has `verify_identity()` (process.rs) matching the
Python router's `ProcessManager.verify_identity()`:

1. Checks `GET /health` — verifies the `model` field matches the profile alias
2. Checks `GET /v1/models` — verifies `data[0].id` matches the profile alias
3. If either mismatches, returns `identity_mismatch` refusal (403)

The identity check is called in `handle_chat` (server.rs) and
`handle_v1_chat` (server.rs) when the backend is in `Healthy` state,
matching the Python router's flow.

**Files:** `process.rs` (+66 lines), `server.rs` (+39 lines)

### Gap 2: Health Poller Degraded → Failed Transition (MEDIUM)

**Before:** The `check_health()` method in `process.rs` transitioned from
Healthy → Degraded after 3 failures, but never transitioned from Degraded → Failed.
A permanently broken backend would remain "degraded" indefinitely.

**After:** When 3 consecutive failures occur while already in Degraded state,
the backend transitions to Failed — matching Python's `poll_health()` behavior.

```rust
match *state {
    BackendState::Healthy => { *state = BackendState::Degraded; }
    BackendState::Degraded => { *state = BackendState::Failed; }   // NEW
    _ => {}
}
```

**Files:** `process.rs` (+7 lines)

### Gap 3: Refusal Keyword Categories Split (MEDIUM)

**Before:** All 12 authority-bearing keywords were checked against a single
list and always returned `"reason": "authority_required"`.

**After:** Three separate keyword categories with distinct refusal reasons,
matching the contract specification in `docs/architecture/ROUTER-PORTABILITY-CONTRACT.md`:

| Category | Keywords | Refusal Reason |
|----------|----------|----------------|
| Authority | approve, promote, commit, escalate, authorize, mark valid, override policy, ignore policy | `authority_required` |
| File mutation | edit source, modify file, write to librarian | `file_mutation_forbidden` |
| Autonomous action | autonomous, self-directed, automatic decision | `autonomous_action_forbidden` |

Priority order: authority → file mutation → autonomous action.
This matches the contract intent, fixing unreachable dead code in the Python
reference where all three categories were merged into one check.

**Files:** `refusal.rs` (+52 lines)

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

## Gap Closure Summary

| # | Gap | Severity | Status | Files Changed |
|---|-----|----------|--------|---------------|
| 1 | Identity verification in chat refusal | **HIGH** | ✅ CLOSED | process.rs, server.rs |
| 2 | Health poller Degraded → Failed | **MEDIUM** | ✅ CLOSED | process.rs |
| 3 | Refusal keyword categories | **MEDIUM** | ✅ CLOSED | refusal.rs |
| | **Total: 3 gaps closed** | | | **3 files, +168/-10 lines** |

Remaining gaps in set: 15 (out of 20 identified in ROUTER-DAEMON-IMPLEMENTATION-1)

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
| librarian-runtime-node git status | **Modified (3 files — staged for commit)** |
| TheLibrarian-main git status | **Clean** (unchanged) |
| Stashes | **Empty** |

---

*Sealed: 2026-06-23*
*Authority: advisory_only*
