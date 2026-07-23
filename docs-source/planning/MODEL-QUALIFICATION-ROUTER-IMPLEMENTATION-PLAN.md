# MODEL-QUALIFICATION-ROUTER-IMPLEMENTATION-PLAN

**Sprint:** MODEL-QUALIFICATION-ROUTER-BASELINE-INTEGRATION-PLAN-1
**Gate:** MQR-BI-14, MQR-BI-15

---

## Purpose

Bounded implementation sprint chain to build the qualification and routing system on top of the sealed Windows runtime baseline. Each sprint produces a testable deliverable. No sprint changes the sealed Windows tables.

---

## Constraints

- Sealed Windows DB (6 domain tables) is never altered
- Sealed residency supervisor is never modified
- Sealed process.rs / BackendState is never modified
- Each sprint must compile and pass all tests
- Each sprint must have verifiable gate criteria
- Sprints build incrementally; no throwaway work

---

## Phase 1: Mac-Side Foundation

### Sprint MQR-F1: Mac Canonical DB + Model Identity

**Goal:** Create the Mac-side canonical database and model identity records.

**Deliverables:**
- Mac canonical DB schema with migrations
- model_identity_record table
- system_profile table
- RuntimeDatabase pattern (matching Windows pattern)
- CRUD operations for both tables
- 20+ unit tests

**Gate criteria:**
- MQR-F1-1: Mac canonical DB initializes successfully
- MQR-F1-2: Migrations are idempotent
- MQR-F1-3: model_identity_record CRUD works
- MQR-F1-4: system_profile CRUD works
- MQR-F1-5: 20+ tests pass

**Files:** `src/canonical/mod.rs`, `db.rs`, `migrations.rs`, `models/identity.rs`, `models/system.rs`

---

### Sprint MQR-F2: Task Pack + Validator Pack

**Goal:** Versioned work fixtures and validation rules for qualification.

**Deliverables:**
- task_pack table
- validator_pack table
- CRUD operations
- Fixture loading from filesystem
- Validator rule execution engine
- 15+ unit tests

**Gate criteria:**
- MQR-F2-1: task_pack CRUD works
- MQR-F2-2: validator_pack CRUD works
- MQR-F2-3: Fixture loading from filesystem works
- MQR-F2-4: Validator rule execution works
- MQR-F2-5: 15+ tests pass

**Files:** `src/canonical/task_packs.rs`, `src/canonical/validator_packs.rs`, `src/canonical/validators/mod.rs`

---

### Sprint MQR-F3: Bridge Packet Types + Serialization

**Goal:** Evidence packet and qualification request packet types with serialization/deserialization.

**Deliverables:**
- EvidencePacket struct + JSON serialization
- QualificationRequest struct + JSON serialization
- ResidencyStatus struct + JSON serialization
- Packet validation logic
- 20+ unit tests

**Gate criteria:**
- MQR-F3-1: EvidencePacket serializes/deserializes correctly
- MQR-F3-2: QualificationRequest serializes/deserializes correctly
- MQR-F3-3: ResidencyStatus serializes/deserializes correctly
- MQR-F3-4: Packet validation catches malformed inputs
- MQR-F3-5: 20+ tests pass

**Files:** `src/canonical/packets/mod.rs`, `src/canonical/packets/evidence.rs`, `src/canonical/packets/request.rs`, `src/canonical/packets/status.rs`

---

## Phase 2: Windows Evidence Export

### Sprint MQR-W1: Windows Evidence Export API

**Goal:** Windows HTTP endpoints to export execution evidence for Mac qualification intake.

**Deliverables:**
- GET /evidence/runs/{run_id} endpoint
- GET /evidence/lifecycle?lease_id={lease_id} endpoint
- GET /evidence/lifecycle?run_id={run_id} endpoint
- EvidencePacket construction from DB records
- Release verification computation
- 15+ unit tests

**Gate criteria:**
- MQR-W1-1: Evidence export returns correct runtime_run data
- MQR-W1-2: Lifecycle events are chronologically ordered
- MQR-W1-3: Release verification is computed correctly
- MQR-W1-4: Missing run_id returns 404
- MQR-W1-5: 15+ tests pass

**Files:** `src/server.rs` (new endpoints), `src/evidence/mod.rs`, `src/evidence/export.rs`

---

### Sprint MQR-W2: Windows Residency Status API

**Goal:** Windows HTTP endpoint for Mac scheduler to query residency state.

**Deliverables:**
- GET /residency/status endpoint
- GET /residency/status?model_id={model_id} endpoint
- ResidencySnapshot construction from supervisor state
- VRAM availability computation
- 10+ unit tests

**Gate criteria:**
- MQR-W2-1: Residency status returns current leases and runs
- MQR-W2-2: VRAM availability is computed correctly
- MQR-W2-3: Model-specific query returns filtered results
- MQR-W2-4: 10+ tests pass

**Files:** `src/server.rs` (new endpoints), `src/residency/status.rs`

---

## Phase 3: Mac Qualification Runner

### Sprint MQR-Q1: Qualification Runner Core

**Goal:** Mac-side qualification runner that sends requests to Windows and records evidence.

**Deliverables:**
- QualificationRunner struct
- qualification_request table + CRUD
- qualification_run table + CRUD
- qualification_stage_log table + CRUD
- Request sending to Windows (HTTP client)
- Evidence packet receiving and validation
- 20+ unit tests

**Gate criteria:**
- MQR-Q1-1: QualificationRunner sends requests correctly
- MQR-Q1-2: Evidence packets are validated
- MQR-Q1-3: qualification_run records are created
- MQR-Q1-4: Stage progression is tracked
- MQR-Q1-5: 20+ tests pass

**Files:** `src/canonical/qualification/mod.rs`, `src/canonical/qualification/runner.rs`, `src/canonical/qualification/requests.rs`, `src/canonical/qualification/runs.rs`

---

### Sprint MQR-Q2: Stage 1 Smoke Test Executor

**Goal:** Automated Stage 1 smoke test execution with pass/fail criteria.

**Deliverables:**
- Stage1SmokeTest executor
- Pass/fail criteria evaluation
- Timeout handling
- GPU release verification
- 15+ unit tests

**Gate criteria:**
- MQR-Q2-1: Stage 1 executes correctly against Windows
- MQR-Q2-2: Pass criteria are evaluated correctly
- MQR-Q2-3: Fail criteria are evaluated correctly
- MQR-Q2-4: Timeouts are handled
- MQR-Q2-5: 15+ tests pass

**Files:** `src/canonical/qualification/stages/mod.rs`, `src/canonical/qualification/stages/smoke.rs`

---

### Sprint MQR-Q3: Stage 2 Primitive Probe Executor

**Goal:** Automated Stage 2 primitive probe execution with threshold tests.

**Deliverables:**
- Stage2PrimitiveProbe executor
- Task pack loading and fixture execution
- Validator pack application
- Threshold evaluation
- 20+ unit tests

**Gate criteria:**
- MQR-Q3-1: Stage 2 executes primitive probes correctly
- MQR-Q3-2: Task packs are loaded and executed
- MQR-Q3-3: Validator packs are applied to outputs
- MQR-Q3-4: Threshold tests evaluate correctly
- MQR-Q3-5: 20+ tests pass

**Files:** `src/canonical/qualification/stages/primitive_probes.rs`

---

## Phase 4: Capability and Routing

### Sprint MQR-C1: Capability Manifest + Owner Decision

**Goal:** Capability manifest creation and Owner decision workflow.

**Deliverables:**
- capability_manifest table + CRUD
- owner_decision table + CRUD
- Manifest status transitions
- Owner decision application
- 15+ unit tests

**Gate criteria:**
- MQR-C1-1: capability_manifest CRUD works
- MQR-C1-2: Owner decision CRUD works
- MQR-C1-3: Status transitions are enforced
- MQR-C1-4: Owner decisions apply correctly
- MQR-C1-5: 15+ tests pass

**Files:** `src/canonical/capability/mod.rs`, `src/canonical/capability/manifest.rs`, `src/canonical/capability/decisions.rs`

---

### Sprint MQR-R1: Execution Profile + Router Projection

**Goal:** Execution profile characterization and router projection management.

**Deliverables:**
- execution_profile table + CRUD
- router_projection table + CRUD
- Projection creation from approved manifests
- Projection supersession logic
- Projection expiry handling
- 15+ unit tests

**Gate criteria:**
- MQR-R1-1: execution_profile CRUD works
- MQR-R1-2: router_projection CRUD works
- MQR-R1-3: Projection creation works
- MQR-R1-4: Supersession works correctly
- MQR-R1-5: 15+ tests pass

**Files:** `src/canonical/routing/mod.rs`, `src/canonical/routing/projection.rs`, `src/canonical/routing/execution_profile.rs`

---

### Sprint MQR-R2: Packet Router Integration

**Goal:** Router consumes approved projection to route work packets.

**Deliverables:**
- Router selection algorithm
- Role-based projection query
- Hardware constraint filtering
- Priority-based selection
- routing_log table + CRUD
- 15+ unit tests

**Gate criteria:**
- MQR-R2-1: Router selects correct model for role
- MQR-R2-2: Hardware constraints are filtered
- MQR-R2-3: Priority selection works
- MQR-R2-4: routing_log records decisions
- MQR-R2-5: 15+ tests pass

**Files:** `src/canonical/routing/router.rs`, `src/canonical/routing/log.rs`

---

## Phase 5: Integration and Validation

### Sprint MQR-I1: End-to-End Qualification Pipeline

**Goal:** Full qualification pipeline from model intake to capability manifest.

**Deliverables:**
- Intake agent: model discovery → identity record
- Stage 1 smoke test execution
- Stage 2 primitive probe execution (subset)
- Capability manifest creation
- Owner decision workflow
- 20+ integration tests

**Gate criteria:**
- MQR-I1-1: Model intake works end-to-end
- MQR-I1-2: Stage 1 executes correctly
- MQR-I1-3: Stage 2 executes correctly (subset)
- MQR-I1-4: Capability manifest is created
- MQR-I1-5: Owner decision is recorded
- MQR-I1-6: 20+ integration tests pass

**Files:** Integration test suite, pipeline orchestration

---

### Sprint MQR-I2: End-to-End Routing Pipeline

**Goal:** Full routing pipeline from work packet to model response.

**Deliverables:**
- Packet receipt and role extraction
- Router projection selection
- Windows residency acquisition
- Generation execution
- Response delivery
- Routing log
- 15+ integration tests

**Gate criteria:**
- MQR-I2-1: Packet routing works end-to-end
- MQR-I2-2: Router selects correct model
- MQR-I2-3: Windows residency is acquired
- MQR-I2-4: Generation completes successfully
- MQR-I2-5: Response is delivered correctly
- MQR-I2-6: 15+ integration tests pass

**Files:** Integration test suite, routing orchestration

---

### Sprint MQR-I3: Comparative Analysis + Roster

**Goal:** Compare candidate models against qualified roster.

**Deliverables:**
- comparative_analysis table + CRUD
- Roster comparison logic
- Findings generation
- 10+ unit tests

**Gate criteria:**
- MQR-I3-1: comparative_analysis CRUD works
- MQR-I3-2: Roster comparison produces findings
- MQR-I3-3: Findings are recorded correctly
- MQR-I3-4: 10+ tests pass

**Files:** `src/canonical/comparison/mod.rs`, `src/canonical/comparison/roster.rs`

---

## Sprint Summary

| Phase | Sprint | Deliverable | Tests | Gates |
|-------|--------|-------------|-------|-------|
| 1 | MQR-F1 | Mac Canonical DB + Model Identity | 20+ | 5 |
| 1 | MQR-F2 | Task Pack + Validator Pack | 15+ | 5 |
| 1 | MQR-F3 | Bridge Packet Types | 20+ | 5 |
| 2 | MQR-W1 | Windows Evidence Export API | 15+ | 5 |
| 2 | MQR-W2 | Windows Residency Status API | 10+ | 5 |
| 3 | MQR-Q1 | Qualification Runner Core | 20+ | 5 |
| 3 | MQR-Q2 | Stage 1 Smoke Test | 15+ | 5 |
| 3 | MQR-Q3 | Stage 2 Primitive Probes | 20+ | 5 |
| 4 | MQR-C1 | Capability Manifest + Owner Decision | 15+ | 5 |
| 4 | MQR-R1 | Execution Profile + Router Projection | 15+ | 5 |
| 4 | MQR-R2 | Packet Router Integration | 15+ | 5 |
| 5 | MQR-I1 | End-to-End Qualification Pipeline | 20+ | 6 |
| 5 | MQR-I2 | End-to-End Routing Pipeline | 15+ | 6 |
| 5 | MQR-I3 | Comparative Analysis + Roster | 10+ | 4 |
| **Total** | **14 sprints** | | **220+** | **70** |

---

## Dependencies

```
Phase 1 (all sprints) → Phase 2 (all sprints) → Phase 3 (all sprints) → Phase 4 (all sprints) → Phase 5 (all sprints)
```

Within each phase, sprints can proceed sequentially.

---

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| Windows API compatibility | Sprint MQR-W1 and MQR-W2 are early; validate against sealed runtime |
| Evidence packet format mismatch | Sprint MQR-F3 defines exact types before implementation |
| Qualification runner complexity | Sprint MQR-Q1 is core; Q2 and Q3 build incrementally |
| Router selection edge cases | Sprint MQR-R2 has 15+ tests covering all scenarios |
| End-to-end integration | Sprint MQR-I1 and MQR-I2 are final; all earlier sprints must pass |
