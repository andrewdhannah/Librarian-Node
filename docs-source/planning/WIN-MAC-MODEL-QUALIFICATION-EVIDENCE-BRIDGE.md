# WIN-MAC-MODEL-QUALIFICATION-EVIDENCE-BRIDGE

**Sprint:** MODEL-QUALIFICATION-ROUTER-BASELINE-INTEGRATION-PLAN-1
**Gate:** MQR-BI-5

---

## Purpose

Define the exact contract for transferring execution evidence from Windows operational DB to Mac canonical DB for qualification intake.

---

## Bridge Direction

```
Windows (execution authority) ──── evidence_packet ────→ Mac (qualification authority)
Mac (qualification authority) ──── qualification_request ────→ Windows (execution authority)
```

---

## 1. Evidence Packet Contract

### Trigger

A qualification run has completed on Windows. The Mac qualification runner requests the evidence packet from Windows.

### Windows Evidence Export API

```
GET /evidence/runs/{run_id}
GET /evidence/lifecycle?lease_id={lease_id}
GET /evidence/lifecycle?run_id={run_id}
```

### Evidence Packet Structure

| Section | Purpose | Required |
|---------|---------|----------|
| packet_type | Always "evidence_packet" | YES |
| packet_version | Schema version | YES |
| exported_at | UTC timestamp of export | YES |
| qualification_request_id | Links to the Mac-side request | YES |
| identity | Model identity (model_id, sha256, filename, quantization) | YES |
| execution | Runtime profile, hardware profile, executable identity | YES |
| lease | Lease lifecycle (loaded_at, released_at, vram_released_at) | YES |
| run | Execution metrics (tokens, timing, exit_status) | YES |
| lifecycle_events | Ordered array of lifecycle_evidence records | YES |
| release_verification | PID exit + GPU release proof | YES |
| residency_snapshot | Active leases at start/end of run | NO (for debugging) |

### Evidence Packet Integrity

| Rule | Description |
|------|-------------|
| Lifecycle events must be chronologically ordered | oldest first, based on occurred_at |
| run_id must match the lifecycle_events run_id references | referential integrity |
| release_verification must include baseline and measured free_vram | tolerance check included |
| packet is immutable once created | no updates, only replacement |
| packet_hash is SHA-256 of the full packet JSON | integrity verification |

### Evidence Packet Validation (Mac Side)

The Mac evidence intake validates:
1. Packet version is supported
2. Identity sha256 matches local_models sha256 on Windows
3. Lifecycle events are chronologically ordered
4. run_id references are consistent
5. release_verification tolerance is within bounds
6. packet_hash matches recomputed hash

---

## 2. Qualification Request Contract

### Trigger

Mac qualification runner determines a run is needed for a specific model + task + validator combination.

### Request Structure

| Section | Purpose | Required |
|---------|---------|----------|
| packet_type | Always "qualification_request" | YES |
| packet_version | Schema version | YES |
| request_id | Unique request identifier | YES |
| identity | Model identity for execution binding | YES |
| execution | Task description, max tokens, temperature, timeout | YES |
| constraints | Release proof requirement, VRAM limit | YES |

### Request Validation (Windows Side)

The Windows execution agent validates:
1. Model is installed (local_models contains sha256)
2. Runtime profile exists and is enabled
3. Hardware can accommodate VRAM requirement
4. No active residency conflict (or can acquire)
5. Request timeout is reasonable (≤ 600s)

---

## 3. Residency Status Query Contract

### Trigger

Mac scheduler needs current residency state for routing decisions.

### Query

```
GET /residency/status
GET /residency/status?model_id={model_id}
```

### Response

```json
{
  "active_leases": [...],
  "active_runs": [...],
  "draining": false,
  "available_vram_mb": 3433,
  "baseline_vram_mb": 3433
}
```

### Usage

Mac scheduler uses this to:
1. Check if a model is currently loaded (can route immediately)
2. Check if a model can be loaded (enough VRAM)
3. Check if drain is in progress (wait before acquiring)
4. Check available capacity for concurrent loads (if future multi-model support)

---

## 4. Error Handling

| Error | Windows Response | Mac Handling |
|-------|-----------------|-------------|
| Model not installed | 404 "model not found" | Retry after model install, or skip |
| Runtime profile missing | 404 "profile not found" | Create profile first |
| Hardware insufficient | 403 "insufficient VRAM" | Mark model as hardware-incompatible for this system |
| Residency conflict | 409 "lease active" | Wait for lease release, then retry |
| Execution timeout | 408 "run timed out" | Record timeout as execution failure |
| Process crash | Run completes with exit_status "crashed" | Record crash evidence, continue qualification |
| Evidence export failure | 500 "evidence export failed" | Retry once, then fail the qualification run |

---

## 5. Security Constraints

| Rule | Rationale |
|------|-----------|
| Windows never receives capability data | Authority boundary |
| Evidence packets are read-only snapshots | No mutation during transfer |
| Qualification requests cannot modify Windows DB schema | DB-13 preservation |
| All transfers are authenticated (shared secret or local socket) | Prevent unauthorized access |
| Evidence packets contain no Mac-side decisions | No role assignments, no capability status |

---

## 6. Bridge Record Lifecycle

```
1. Mac creates qualification_request
2. Mac sends request to Windows
3. Windows acquires residency, executes run
4. Windows generates lifecycle evidence
5. Windows creates evidence_packet
6. Mac receives evidence_packet
7. Mac validates packet integrity
8. Mac records qualification_run
9. Mac executes qualification stages
10. Mac produces capability_manifest (if stages pass)
```

Each step is auditable. No step is optional for a valid qualification run.
