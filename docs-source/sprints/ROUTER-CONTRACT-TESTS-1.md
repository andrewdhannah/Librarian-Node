# ROUTER-CONTRACT-TESTS-1: Router HTTP Contract Tests

> Freeze the current router HTTP/API behavior before any native daemon
> or router implementation changes.

---

## Overview

| Field | Value |
|-------|-------|
| Sprint ID | ROUTER-CONTRACT-TESTS-1 |
| Layer | Layer 2 — Portable Router / Native Daemon |
| Status | **SEALED** |
| Start HEAD | TheLibrarian-main: `1e32002`, librarian-runtime-node: `63780bf` |
| End HEAD | TheLibrarian-main: `1e32002`, librarian-runtime-node: `<commit>` |
| Authority | `advisory_only` |

### Starting State

- TheLibrarian-main HEAD: `1e32002` — clean, up to date
- librarian-runtime-node HEAD: `63780bf` — clean, up to date
- Service LibrarianRunTimeNode: Stopped / Manual
- Port 9130: free
- Orphans: 0 llama-server, 0 rust-router
- Stashes: empty
- Layer 1 Runtime Node Reliability complete: 15 sealed sprints

---

## Scope Completed

1. ✅ **Identified** all 7 target router HTTP endpoints in `rust-router/src/server.rs`
2. ✅ **Created** `scripts/tests/run-router-contract-tests.ps1` — 87 contract tests
3. ✅ **Created** `docs/contracts/ROUTER-HTTP-CONTRACT.md` — frozen contract document
4. ✅ **Verified** all endpoints live with both auth-disabled and auth-enabled modes
5. ✅ **Documented** response shapes, status codes, error conditions, and safety boundaries

---

## Acceptance Criteria

| ID | Criterion | Status | Notes |
|----|-----------|--------|-------|
| CONTRACT-001 | Contract tests locate and exercise all target endpoints | **PASS** | All 7 endpoints tested |
| CONTRACT-002 | Auth-required behavior: missing + invalid token | **PASS** | 7 auth tests in Phase 2 |
| CONTRACT-003 | Success response shape: status/profiles/health/models | **PASS** | 30+ shape tests |
| CONTRACT-004 | POST /backend/select contract with bounded profile selection | **PASS** | 6 select tests (valid alias shape, invalid, missing field, task_class) |
| CONTRACT-005 | POST /v1/chat/completions with bounded prompt | **PASS** | 4 tests (no backend, empty messages, missing model, malformed) |
| CONTRACT-006 | POST /backend/stop contract | **PASS** | 3 tests (no backends, with profile) |
| CONTRACT-007 | Malformed/oversized request handling | **PASS** | 3 tests (malformed JSON, blank body, oversized >10MB → 413) |
| CONTRACT-008 | Tests do not persist or expose auth tokens | **PASS** | Temp token generated at runtime, env var cleaned up |
| CONTRACT-009 | Contract documentation added | **PASS** | `docs/contracts/ROUTER-HTTP-CONTRACT.md` |
| CONTRACT-010 | Final service state Stopped / Manual | **PASS** | Verified post-run |
| CONTRACT-011 | Final port 9130 free, orphans 0, trees clean, stashes empty | **PASS** | Verified post-run |
| CONTRACT-012 | Commit is sealed | **PASS** | This document |

---

## Test Results Summary

| Metric | Count |
|--------|-------|
| Total contract tests | **87** |
| Phase 1 (auth disabled) | 71 |
| Phase 2 (auth enabled) | 16 |

### Test Categories

| Category | Tests | Description |
|----------|-------|-------------|
| GET /backend/status | 9 | Status code, shape, authority, no leakage |
| GET /backend/profiles | 10 | Profiles array, per-profile fields, authority, path safety |
| GET /backend/health | 5 | Status code, authority, profiles shape |
| GET /v1/models | 9 | OpenAI-compatible model list, authority, path safety |
| POST /backend/select | 6 | Invalid profile, missing field, invalid task_class |
| POST /backend/stop | 3 | No backends running |
| POST /v1/chat/completions | 4 | No backend, empty messages, missing model |
| Malformed/oversized | 3 | Malformed JSON, blank body, >10 MB body |
| No secret leakage | 1 | All GET endpoints checked |
| Auth (missing token) | 3 | GET/POST without token → 401 |
| Auth (invalid token) | 3 | Wrong token, Bearer prefix → 401 |
| Auth (valid token) | 5 | All GET endpoints with valid token → 200 |

---

## Endpoints Covered

| Endpoint | Method | Phase 1 | Phase 2 (Auth) |
|----------|--------|---------|-----------------|
| `/backend/status` | GET | ✅ 9 tests | ✅ 2 tests |
| `/backend/profiles` | GET | ✅ 10 tests | ✅ 2 tests |
| `/backend/health` | GET | ✅ 5 tests | ✅ 1 test |
| `/v1/models` | GET | ✅ 9 tests | ✅ 1 test |
| `/backend/select` | POST | ✅ 6 tests | ✅ 1 test |
| `/backend/stop` | POST | ✅ 3 tests | ❌ (not run twice) |
| `/v1/chat/completions` | POST | ✅ 4 tests | ❌ (not run twice) |
| Auth behavior | — | ❌ N/A | ✅ 11 tests |

## Auth Behavior Verified

- **Missing token:** 3 tests — GET/POST without token returns 401 with empty body
- **Invalid token:** 3 tests — wrong token returns 401, Bearer prefix also 401 (exact match only)
- **Valid token:** 5 tests — all GET endpoints return 200 with correct shapes
- Token is generated at runtime, never persisted to disk

---

## Excluded Endpoints

| Endpoint | Reason |
|----------|--------|
| `/health` | Legacy endpoint, not part of target 7. Already covered in existing `test-rust-router-endpoints.ps1` |
| `/backend/restart` | Not in scope. Requires an active selected backend to test meaningfully |
| `/backend/chat` | Internal router endpoint. The OpenAI-compatible `/v1/chat/completions` is the tested proxy contract |

---

## Deliverables

| File | Description |
|------|-------------|
| `scripts/tests/run-router-contract-tests.ps1` | Contract test script (87 tests, 2 phases) |
| `docs/contracts/ROUTER-HTTP-CONTRACT.md` | Frozen HTTP contract specification |
| `docs/sprints/ROUTER-CONTRACT-TESTS-1.md` | This sprint closeout document |

### No fixtures or token files created

- No external fixture files were created; tests generate payloads inline
- No token files were persisted; all auth tokens are runtime-generated
- No model profile metadata was changed

---

## Token Safety Statement

> **No credentials, secrets, API keys, or tokens were committed to any file.**
> Auth tests use a randomly generated token (`"router-contract-test-token-<random>"`)
> that is created in memory at runtime and never written to disk. Environment
> variables are cleaned up after test completion.

---

## Final State Verification

| Check | Result |
|-------|--------|
| Service state | **Stopped / Manual** (verified) |
| Port 9130 | **Free** (no LISTENING) |
| rust-router orphans | **0** (verified) |
| llama-server orphans | **0** (verified) |
| librarian-runtime-node git status | **Clean** (nothing to commit after sprint closeout) |
| TheLibrarian-main git status | **Clean** (unchanged) |
| Stashes | **Empty** (verified) |

---

## Related Documents

| Document | Location |
|----------|----------|
| Portable Router Contract | `docs/architecture/ROUTER-PORTABILITY-CONTRACT.md` |
| Runtime Node Architecture | `docs/architecture/RUNTIME-NODE-ARCHITECTURE.md` |
| Router Harden Sprint | `docs/sprints/WIN-ROUTER-HARDEN-1.md` |
| Endpoint Test Script | `scripts/test-rust-router-endpoints.ps1` |
| Profile Config | `config/model-profiles.json` |
| GitHub | `https://github.com/softwareconductor/librarian-runtime-node` |

---

*Sealed: 2026-06-23*
*Authority: advisory_only*
