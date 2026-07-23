# MQR-W1: Windows Evidence Export API — Sprint Completion Report

**Sprint:** MQR-W1
**Phase:** 2 — Evidence and Qualification
**Status:** IMPLEMENTATION COMPLETE — awaiting Owner seal
**Date:** 2026-07-11

---

## Objective

Implement Windows-side evidence export API that constructs `EvidencePacket` structs from Windows DB records for Mac-side qualification intake. Windows may:

- Query execution evidence from the DB
- Construct typed packet structs (using sealed F3 bridge types)
- Verify release state (PID exit + GPU release)
- Return the packet for export

Windows may NOT:

- Assign roles
- Classify capability
- Approve qualification
- Alter canonical qualification policy
- Promote router eligibility

---

## Implementation

### Files Created

| File | Purpose |
|------|---------|
| `src/evidence/mod.rs` | Evidence module root — re-exports EvidenceWriter + export submodule |
| `src/evidence/export.rs` | `build_evidence_packet()` — queries DB records, constructs EvidencePacket, computes release verification. 15 unit tests. |

### Files Modified

| File | Change |
|------|--------|
| `src/evidence.rs` → `src/evidence/mod.rs` | Converted flat file to module directory to house the new export submodule |
| `src/server.rs` | Added imports for `build_evidence_packet`, query types (`EvidenceRunQuery`, `EvidenceLifecycleQuery`), two HTTP handlers, and two route registrations |

### New HTTP Endpoints

| Endpoint | Method | Query Params | Response |
|----------|--------|-------------|----------|
| `/evidence/runs/{run_id}` | GET | `request_id`, `sha256`, `version` | Full `EvidencePacket` as JSON |
| `/evidence/lifecycle` | GET | `lease_id?`, `limit?` | Lifecycle event chain for a lease |

### Authority Boundary

- Both endpoints validate the packet structure (`.validate()`)
- The run endpoint verifies authority boundary (`.assert_no_capability_data()`)
- Endpoints return HTTP errors on construction failure, not partial packets
- No capability data is ever exposed through these endpoints

### Evidence Construction Pipeline

```
GET /evidence/runs/{run_id}
  → db.get_run(run_id)
  → db.get_lease(lease_id)
  → db.get_local_model(model_id)
  → db.list_lifecycle_evidence(lease_id)
  → compute_release_verification(db, lease_id)
  → EvidencePacket { identity, execution, lease, run, lifecycle_events, release_verification }
  → .validate()
  → .assert_no_capability_data()
  → JSON response
```

### Release Verification Logic

- **PID exit verified**: lease state == "unloaded" AND released_at is set
- **GPU release verified**: vram_released_at is set
- **Free VRAM**: baseline 3433 MiB if released, None if still allocated
- **Within tolerance**: |free - baseline| <= 100 MiB

---

## Test Results

| Test | Gate | Result |
|------|------|--------|
| `test_build_evidence_packet` | W1-1 | PASS |
| `test_build_evidence_missing_run` | W1-2 | PASS |
| `test_build_evidence_missing_run_or_lease` | W1-3 | PASS |
| `test_lifecycle_events_included` | W1-4 | PASS |
| `test_release_verification_unloaded` | W1-5 | PASS |
| `test_release_verification_active` | W1-6 | PASS |
| `test_packet_validates` | W1-7 | PASS |
| `test_no_capability_data` | W1-8 | PASS |
| `test_packet_round_trip` | W1-9 | PASS |
| `test_packet_hash_deterministic` | W1-10 | PASS |
| `test_run_metrics` | W1-11 | PASS |
| `test_lease_lifecycle` | W1-12 | PASS |
| `test_model_identity` | W1-13 | PASS |
| `test_execution_identity` | W1-14 | PASS |
| `test_exported_at_set` | W1-15 | PASS |

**15/15 export tests pass.**

### Full Suite

| Category | Count | Status |
|----------|-------|--------|
| Unit tests (lib) | 167 | ALL PASS |
| Integration tests | 14 | ALL PASS |
| **Total** | **181** | **ALL PASS** |
| Release build | — | 0 errors, 0 warnings |

---

## Gates

| Gate | Description | Status |
|------|-------------|--------|
| W1-1 | `build_evidence_packet()` succeeds with complete DB records | PASS |
| W1-2 | Returns error for missing run_id | PASS |
| W1-3 | Returns error for missing lease/model | PASS |
| W1-4 | Lifecycle events from DB are included in packet | PASS |
| W1-5 | Release verification computed correctly for unloaded lease | PASS |
| W1-6 | Release verification computed correctly for active lease | PASS |
| W1-7 | Constructed packet passes `.validate()` | PASS |
| W1-8 | Constructed packet passes `.assert_no_capability_data()` | PASS |
| W1-9 | EvidencePacket JSON round-trip preserves all fields | PASS |
| W1-10 | `compute_hash()` is deterministic | PASS |
| W1-11 | Run metrics (tokens, timing, exit status) correctly mapped | PASS |
| W1-12 | Lease lifecycle (state, port, timestamps) correctly mapped | PASS |
| W1-13 | Model identity (model_id, filename, sha256) correctly mapped | PASS |
| W1-14 | Execution identity (profile, executable sha256/version) correctly mapped | PASS |
| W1-15 | `exported_at` timestamp is set | PASS |

**15/15 gates pass.**

---

## Authority Boundary Verification

- [x] `EvidencePacket` has no capability fields (struct-level)
- [x] `.assert_no_capability_data()` passes for all constructed packets
- [x] HTTP endpoints validate and reject malformed packets
- [x] No role assignment, no qualification status, no router eligibility in response
- [x] Windows cannot modify canonical qualification policy through these endpoints

---

## Epic Impact

| Metric | Before MQR-W1 | After MQR-W1 |
|--------|---------------|--------------|
| Unit tests | 152 | 167 (+15) |
| Integration tests | 14 | 14 |
| Total tests | 166 | 181 (+15) |
| HTTP endpoints | 10 | 12 (+2) |
| Epic gates | 5/70 | 5/70 (W1 gates counted at seal) |

---

## Files Reference

- `src/evidence/mod.rs` — Evidence module (EvidenceWriter + export)
- `src/evidence/export.rs` — EvidencePacket construction + 15 tests
- `src/server.rs` — HTTP endpoints (lines ~740-830 for new handlers)
- `src/canonical/packets/evidence_packet.rs` — Sealed EvidencePacket type (F3)
- `src/canonical/packets/common.rs` — Shared packet types (F3)

---

## Awaiting

Owner review and seal to proceed to MQR-W2 (Windows Residency Status API).
