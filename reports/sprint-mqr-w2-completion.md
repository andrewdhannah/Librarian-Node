# MQR-W2: Windows Residency Status API — Sprint Completion Report

**Sprint:** MQR-W2
**Phase:** 2 — Evidence and Qualification
**Status:** IMPLEMENTATION COMPLETE — awaiting Owner seal
**Date:** 2026-07-11

---

## Objective

Implement Windows-side residency status API that constructs `ResidencyStatusResponse` structs from Windows DB records for Mac-side routing decisions. Windows may:

- Report active leases (non-unloaded, non-failed)
- Report active runs (in-progress generation)
- Report drain state
- Report VRAM posture
- Filter by model ID

Windows may NOT:

- Assign roles
- Classify capability
- Approve qualification
- Take automatic residency action
- Mutate lease state through this endpoint

---

## Implementation

### Files Created

| File | Purpose |
|------|---------|
| `src/evidence/residency_status.rs` | `build_residency_status()` — queries DB records, constructs ResidencyStatusResponse. 15 unit tests. |

### Files Modified

| File | Change |
|------|--------|
| `src/evidence/mod.rs` | Added `pub mod residency_status;` |
| `src/server.rs` | Added import for `build_residency_status`, `ResidencyStatusQuery` type, HTTP handler, route registration |

### New HTTP Endpoint

| Endpoint | Method | Query Params | Response |
|----------|--------|-------------|----------|
| `/residency/status` | GET | `model_id?` | `ResidencyStatusResponse` as JSON |

### Residency Construction Pipeline

```
GET /residency/status?model_id=...
  → db.get_active_leases()
  → filter by model_id (if provided)
  → for each active lease: db.list_runs_for_lease(lease_id) → filter for in-progress (ended_at IS NULL)
  → detect drain state (any lease in Draining)
  → ResidencyStatusResponse { active_leases, active_runs, draining, VRAM }
  → .validate()
  → .assert_no_capability_data()
  → JSON response
```

### Data Mapping

| DB Record | Packet Field | Filter |
|-----------|-------------|--------|
| job_leases (state NOT IN unloaded,failed) | active_leases | Optional model_id filter |
| runtime_runs (ended_at IS NULL) | active_runs | Per active lease |
| Any lease with state=Draining | draining=true | — |
| Baseline VRAM | available_vram_mb, baseline_vram_mb | Fixed 3433 MiB |

### Authority Boundary

- Both endpoints validate the response (`.validate()`)
- The status endpoint verifies authority boundary (`.assert_no_capability_data()`)
- Endpoints return HTTP errors on construction failure, not partial responses
- No capability data, role assignment, or qualification status exposed

---

## Test Results

| Test | Gate | Result |
|------|------|--------|
| `test_empty_db` | W2-1 | PASS |
| `test_active_lease_included` | W2-2 | PASS |
| `test_unloaded_lease_excluded` | W2-3 | PASS |
| `test_failed_lease_excluded` | W2-4 | PASS |
| `test_running_lease_included` | W2-5 | PASS |
| `test_draining_lease_flag` | W2-6 | PASS |
| `test_active_run_included` | W2-7 | PASS |
| `test_completed_run_excluded` | W2-8 | PASS |
| `test_model_id_filter` | W2-9 | PASS |
| `test_timestamp_set` | W2-10 | PASS |
| `test_vram_values` | W2-11 | PASS |
| `test_response_validates` | W2-12 | PASS |
| `test_no_capability_data` | W2-13 | PASS |
| `test_serialization_round_trip` | W2-14 | PASS |
| `test_hash_deterministic` | W2-15 | PASS |

**15/15 export tests pass.**

### Full Suite

| Category | Count | Status |
|----------|-------|--------|
| Unit tests (lib) | 182 | ALL PASS |
| Integration tests | 14 | ALL PASS |
| **Total** | **196** | **ALL PASS** |
| Release build | — | 0 errors, 0 warnings |

---

## Gates

| Gate | Description | Status |
|------|-------------|--------|
| W2-1 | `build_residency_status()` succeeds with empty DB | PASS |
| W2-2 | Active leases are included in response | PASS |
| W2-3 | Unloaded leases are excluded | PASS |
| W2-4 | Failed leases are excluded | PASS |
| W2-5 | Running leases are included | PASS |
| W2-6 | Draining leases set draining=true | PASS |
| W2-7 | In-progress runs (no ended_at) are included | PASS |
| W2-8 | Completed runs (has ended_at) are excluded | PASS |
| W2-9 | model_id filter works correctly | PASS |
| W2-10 | Timestamp is set | PASS |
| W2-11 | VRAM values are set correctly | PASS |
| W2-12 | Constructed response passes `.validate()` | PASS |
| W2-13 | Constructed response passes `.assert_no_capability_data()` | PASS |
| W2-14 | Serialization round-trip preserves all fields | PASS |
| W2-15 | `compute_hash()` is deterministic | PASS |

**15/15 gates pass.**

---

## Authority Boundary Verification

- [x] `ResidencyStatusResponse` has no capability fields (struct-level)
- [x] `.assert_no_capability_data()` passes for all constructed responses
- [x] HTTP endpoints validate and reject malformed responses
- [x] No role assignment, no qualification status, no router eligibility in response
- [x] No automatic residency action taken through this endpoint

---

## Epic Impact

| Metric | Before MQR-W2 | After MQR-W2 |
|--------|---------------|--------------|
| Unit tests | 167 | 182 (+15) |
| Integration tests | 14 | 14 |
| Total tests | 181 | 196 (+15) |
| HTTP endpoints | 12 | 13 (+1) |
| Epic gates | 30/70 | 30/70 (W2 gates counted at seal) |

---

## Files Reference

- `src/evidence/residency_status.rs` — ResidencyStatusResponse construction + 15 tests
- `src/server.rs` — HTTP endpoint (lines ~850-880 for handler)
- `src/canonical/packets/residency_status.rs` — Sealed ResidencyStatusResponse type (F3)
- `src/canonical/packets/common.rs` — Shared packet types (F3)

---

## Awaiting

Owner review and seal to proceed to MQR-Q1 (Qualification Runner Core).
