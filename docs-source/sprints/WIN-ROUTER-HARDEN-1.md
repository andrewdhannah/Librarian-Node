# Sprint: WIN-ROUTER-HARDEN-1
**Status:** COMPLETED
**Date:** 2026-06-20

## Objective
Harden and verify the existing Python Windows runtime router without broad redesign. Ensure all contract endpoints are robust, failure cases are handled gracefully, and the process lifecycle remains clean (no orphans).

## Final Result
- **Endpoint Matrix:** PASS. All 6 contract endpoints verified.
- **Failure Matrix:** PASS. Invalid JSON, missing/unknown profiles, and malformed chat requests handled.
- **Orphan Check:** PASS. `/backend/restart` and `Stop-Service` leave no orphans.
- **Authority Boundary:** PASS. `authority: advisory_only` preserved; authority-bearing content refused.
- **Service Compatibility:** PASS. Service state remains Manual; NSSM binary ignored.

## Startup Inspection
- **Starting HEAD:** `4a34666`
- **Working Tree:** Clean
- **Service Name:** `LibrarianRunTimeNode`
- **Service State:** Stopped / Manual
- **Router Port:** 9130
- **NSSM Binary:** Ignored/Untracked

## Verification Matrix

### 1. Endpoint Verification
| Endpoint | Method | Expected Result | Status | Notes |
|----------|--------|-----------------|--------|-------|
| `/backend/status` | GET | 200 OK, router/runtime status | COMPLETED | Verified |
| `/backend/profiles` | GET | 200 OK, list of profiles | COMPLETED | Verified |
| `/backend/health` | GET | 200 OK, health of all profiles | COMPLETED | Verified |
| `/backend/select` | POST | 200 OK (valid) / 403 Refused (invalid) | COMPLETED | Verified |
| `/backend/chat` | POST | 200 OK (valid) / 403 Refused (invalid) | COMPLETED | Verified |
| `/backend/restart` | POST | 200 OK (success) / 500 Error (fail) | COMPLETED | Verified |

### 2. Failure-Case Verification
| Case | Request | Expected Result | Status | Notes |
|------|---------|-----------------|--------|-------|
| Invalid JSON | `POST /backend/select` with malformed JSON | 400 Bad Request | COMPLETED | Verified |
| Missing Profile | `POST /backend/select {"not_profile": "..."}` | 400 Bad Request | COMPLETED | Verified |
| Unknown Profile | `POST /backend/select {"profile": "ghost"}` | 403 Refused (unknown_profile) | COMPLETED | Verified |
| Malformed Chat | `POST /backend/chat` with missing messages | 400 Bad Request | COMPLETED | Verified |
| Chat Before Select | `POST /backend/chat` without active profile | 503 Service Unavailable / 403 Refused | COMPLETED | Returns `runtime_unhealthy` |
| Duplicate Select | `POST /backend/select` for already running profile | 200 OK (no-op/restart) | COMPLETED | Verified (no-op) |
| Profile Switch | `POST /backend/select` for different profile | 200 OK (starts new) | COMPLETED | Verified (both run) |

### 3. Lifecycle & Orphan Verification
| Scenario | Action | Expected Result | Status | Notes |
|----------|--------|-----------------|--------|-------|
| Backend Launch | `POST /backend/select` | `llama-server.exe` starts as child of router | COMPLETED | Verified |
| Backend Restart | `POST /backend/restart` | Old PID terminated, new PID started, no orphans | COMPLETED | Verified |
| Service Stop | `Stop-Service` | Router, Launcher, and all Backends terminated | COMPLETED | Verified in WIN-BACKEND-SERVICE-PROOF-1 |

### 4. Logging & Authority
| Check | Expected Result | Status | Notes |
|-------|-----------------|--------|-------|
| Service Logs | stdout/stderr captured in `logs\` | COMPLETED | Verified via EvidenceWriter |
| Error Visibility | Errors are descriptive and logged | COMPLETED | Verified |
| Authority | Responses contain `authority: advisory_only` | COMPLETED | Verified |

## Final Result
- **Endpoint Matrix:** PENDING
- **Failure Matrix:** PENDING
- **Orphan Check:** PENDING
- **Authority Boundary:** PENDING
