# ROUTER-DAEMON-HARDEN-4: Profile Serialization and Internal Test Coverage

> Final parity gap closure sprint. Adds additive profile fields and internal
> Rust test coverage. Remaining ledger closed as Accepted Divergence or
> Deferred Architecture.

---

## Overview

| Field | Value |
|-------|-------|
| Sprint ID | ROUTER-DAEMON-HARDEN-4 |
| Layer | Layer 2 — Portable Router / Native Daemon |
| Status | **SEALED** |
| Start HEAD | TheLibrarian-main: `1e32002`, librarian-runtime-node: `06c0cae` |
| End HEAD | TheLibrarian-main: `1e32002`, librarian-runtime-node: `<commit>` |
| Authority | `advisory_only` |

---

## Scope Completed

### #8: Full Profile Serialization

**File:** `config.rs`

**Before:** `ProfileManager::list_all()` returned 5 fields per profile
(alias, task_classes, verified, port, model_file).

**After:** 5 additive fields added — all derived from existing in-memory data:

| Field | Source | Type |
|-------|--------|------|
| `backend` | `profile.backend` | string |
| `context` | `profile.context` | integer |
| `ngl` | `profile.ngl` | integer |
| `evidence_path` | `profile.evidence_path` | string or null |
| `limitations` | `profile.limitations` | string |

**Contract impact:** None — additive only. Existing fields unchanged.
Existing consumers ignore unknown fields per JSON convention.

**Change:** `+5` lines in `list_all()`

### #14: Internal Test Coverage

**File:** `tests/integration_test.rs`

Complete rewrite with 14 tests (+10 new, 4 original preserved):

| Category | Tests | What It Tests |
|----------|-------|---------------|
| Auth (preserved) | 3 | Success, failure, disabled |
| Body limit (preserved) | 1 | Oversized body → 413 |
| Profile shape (new) | 2 | `test_profiles_contains_all_fields`, `test_profiles_contains_authority` |
| Refusal engine (new) | 5 | Authority content, file mutation, autonomous action, unknown profile, context overflow |
| 404 catch-all (new) | 1 | `test_404_returns_json_error` |
| Status shape (new) | 1 | `test_status_contains_contract_fields` |
| Refusal priority (new) | 1 | Runtime-unhealthy gates before content check |

**No flaky tests:** All tests use `tower::ServiceExt::oneshot()` against
a constructed router with no actual backend processes. No llama-server,
GPU, or port timing required.

**Change:** `+358/-106` lines in test file

---

## Build & Test

```
cargo build --release   → PASS (0 warnings)
cargo test              → 14/14 PASS (0 warnings)
```

## Contract Harness

```
scripts/tests/run-router-contract-tests.ps1 → 67/67 PASS
```

---

## Final Gap Ledger

| Category | Count | Items |
|----------|-------|-------|
| **Closed** | **14** | All 3 sprints combined |
| Accepted divergence | 4 | #7, #12, #17, #20 |
| Deferred architecture | 3 | #4, #15, #16 |
| Future runtime work | **0** | ✅ |
| **Total original gaps** | **20** | Fully classified |

---

## Token Safety Statement

> **No credentials, secrets, API keys, or tokens were committed.**

---

## Final State Verification

| Check | Result |
|-------|--------|
| Service state | **Stopped / Manual** |
| Port 9130 | **Free** |
| Orphans | **0** |
| Working trees | **Clean** |
| Stashes | **Empty** |

---

*Sealed: 2026-06-23*
*Authority: advisory_only*
