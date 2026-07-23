# EPIC-MODEL-QUALIFICATION-ROUTER-HARDENING-1

**Status:** COMPLETE AND SEALED ✅  
**Parent Epic:** EPIC-MODEL-QUALIFICATION-ROUTER-IMPLEMENTATION-1 (CLOSED/ACCEPTED)  
**Sprints:** 6/6 SEALED  
**Tests:** 624 total, 0 failures, 0 warnings  
**Gates:** 40/40 PASS

---

## Purpose

Harden the sealed qualification system with transport-level integration proof, persistence safety, controlled extensibility, operational retesting, and local batch qualification — before any community-facing lane work begins.

---

## Constraints

- All 14 sealed MQR implementation sprints are immutable
- Each sprint produces testable deliverables with explicit gate criteria
- Failed gates stop the sprint; no silent regressions
- Each sprint builds incrementally
- Persistence must never recreate authority from raw performance evidence
- Transport tests must use real HTTP (not function-call substitution)
- No automatic migration framework for registry schema

---

## Sprints

### Sprint MQR-H1: Live HTTP Bridge Integration Tests

**Status:** SEALED  
**Goal:** Prove the Bridge client and HTTP transport work end-to-end against real Axum servers.

**Deliverables:**
- `BridgeClient` with `get_evidence_run()`, `get_evidence_lifecycle()`, `get_residency_status()`
- `BridgeError` enum: Transport, Timeout, HttpStatus, Deserialization, Validation, IdentityMismatch
- `TestServerControl` with per-endpoint override capability
- Axum test server on random ports

**Gate Criteria:**

| Gate | Description | Status |
|------|-------------|--------|
| H1-1 | Bridge client makes real HTTP requests (not function calls) | ✅ PASS |
| H1-2 | BridgeError variants are all distinguishable and populated | ✅ PASS |
| H1-3 | Query parameter identity continuity works end-to-end | ✅ PASS |
| H1-4 | Transport errors (connection refused, timeout) are classified correctly | ✅ PASS |
| H1-5 | 30+ integration tests pass | ✅ PASS |

**Tests:** 30 integration + 5 unit = 35 tests  
**Files:** `src/canonical/bridge/mod.rs`, `src/canonical/bridge/client.rs`, `tests/bridge_integration_test.rs`

---

### Sprint MQR-H2: Persistent Router Registry State

**Status:** IMPLEMENTATION COMPLETE — Owner Seal Pending  
**Goal:** Prove approved qualification state survives process restart; persistence may NOT recreate authority from raw performance evidence.

**Deliverables:**
- `RegistryStore` (atomic write via temp+rename)
- `RegistryFile` (schema v2 JSON format)
- `RegistryState`, `RegistryLoadResult` (Loaded/Empty/Incompatible/Corrupt)
- `RegistryError` (7 classified variants)
- Content hash + authority chain validation

**Gate Criteria:**

| Gate | Description | Status |
|------|-------------|--------|
| H2-1 | Atomic write prevents partial state corruption on save | ✅ PASS |
| H2-2 | Schema version mismatch returns Incompatible (not Corrupt) | ✅ PASS |
| H2-3 | Content hash validation detects tampered or corrupted records | ✅ PASS |
| H2-4 | Authority chain validation detects dangling refs, missing authority, identity divergence | ✅ PASS |
| H2-5 | Metrics-only state has zero routing eligibility (negative control) | ✅ PASS |
| H2-6 | 39+ tests pass (9 unit + 30 integration) | ✅ PASS |

**Tests:** 9 unit + 30 integration = 39 tests  
**Files:** `src/canonical/registry/mod.rs`, `src/canonical/registry/store.rs`, `tests/registry_persistence_test.rs`  
**Report:** `reports/sprint-mqr-h2-completion.md`

---

### Sprint MQR-H3: Comparative Registry Persistence

**Status:** SEALED  
**Goal:** Move comparative analysis from transient in-memory computation to durable, auditable records. Comparative evidence must become durable without becoming authoritative.

**Deliverables:**
- `ComparisonAuditRecord` type with full comparison context
- `ArtifactReference`, `ThresholdSnapshot`, `ComparisonMethodology`
- `ANALYZER_VERSION` ("1.0.0")
- Registry schema v3 with `comparison_audit_records` field
- Content hash + validation for audit records
- `from_comparison()` constructor from `ComparisonResult` + `RosterRecommendation`

**Gate Criteria:**

| Gate | Description | Result |
|------|-------------|--------|
| H3-1 | ComparisonAuditRecord persists and reloads with all fields | ✅ PASS |
| H3-2 | Reloaded audit record preserves advisory status (no auto-mutation) | ✅ PASS |
| H3-3 | Audit record does not create routing eligibility | ✅ PASS |
| H3-4 | Full comparison context (methodology, thresholds, analyzer version, findings) preserved | ✅ PASS |
| H3-5 | 15+ tests pass (9 unit + 15 integration) | ✅ PASS |

**Tests:** 9 unit + 15 integration = 24 tests  
**Files:** `src/canonical/comparative/audit.rs`, `tests/comparative_persistence_test.rs`  
**Report:** `reports/sprint-mqr-h3-completion.md`

---

### Sprint MQR-H4: Custom Validator Rule Execution

**Status:** SEALED  
**Goal:** Create a bounded validator execution layer that produces validation evidence — not capability authority. Custom rules execute with identity, versioning, timeouts, and failure isolation.

**Deliverables:**
- `CustomRuleDefinition` with explicit identity and versioning
- `CustomRuleExecutor` with built-in strategies (pass, fail, contains, min_tokens, panic, hang)
- `CustomRuleOutcome` with distinct fields for passed, timed_out, panicked, execution_duration_ms
- `CustomRuleEvidence` with content hash for tamper detection
- `validation_rule_to_definition()` converter from existing `ValidationRule::RuleType::Custom`
- Three-layer failure isolation (outer catch_unwind, inner catch_unwind, recv_timeout)

**Gate Criteria:**

| Gate | Description | Result |
|------|-------------|--------|
| H4-1 | CustomRuleExecutor executes with explicit identity + versioning | ✅ PASS |
| H4-2 | Timeout handling terminates hanging rules within bound | ✅ PASS |
| H4-3 | Panic isolation catches panics without crashing the executor | ✅ PASS |
| H4-4 | Structured evidence is serializable, hashable, preserves outcome | ✅ PASS |
| H4-5 | 20+ tests pass (20 unit; exceeds requirement) | ✅ PASS |

**Tests:** 20 unit tests  
**Files:** `src/canonical/qualification/custom_executor.rs`  
**Report:** `reports/sprint-mqr-h4-completion.md`

---

### Sprint MQR-H5: Custom Validation Evidence Integration

**Status:** SEALED  
**Goal:** Integrate the bounded custom validation evidence layer into the qualification pipeline while preserving the authority boundary. Custom evidence flows through qualification run results without becoming authoritative.

**Deliverables:**
- `custom_evidence` field on `QualificationRunResult` (evidence collection container)
- `apply_custom_rules()` free function in custom_executor module
- Updated `assert_no_capability_data()` to include custom_evidence
- Evidence lifecycle: creation, serialization, persistence, hash stability, ordering
- Authority boundary regression tests (25 total)

**Gate Criteria:**

| Gate | Requirement | Result |
|------|-------------|--------|
| H5-1 | Custom evidence flows through pipeline without authority escalation | ✅ PASS |
| H5-2 | Evidence remains deterministic and reproducible | ✅ PASS |
| H5-3 | Failure outcomes represented as evidence, not decisions | ✅ PASS |
| H5-4 | Existing qualification behavior unchanged when custom validation absent | ✅ PASS |
| H5-5 | Regression suite passes with new integration coverage | ✅ PASS |

**Tests:** 25 integration tests  
**Files:** `src/canonical/qualification/custom_executor.rs` (modified), `src/canonical/qualification/run_result.rs` (modified), `tests/custom_evidence_integration_test.rs` (created)  
**Report:** `reports/sprint-mqr-h5-completion.md`

---

### Sprint MQR-H6: Local Batch Qualification + Closure Validation

**Status:** IMPLEMENTATION COMPLETE — Owner Seal Pending  
**Goal:** Complete the final MQR capability (Local Batch Qualification) and perform final hardening validation proving the full qualification router pipeline remains deterministic, evidence-preserving, and authority-safe.

**Deliverables:**
- `BatchQualificationInput`, `BatchTarget`, `BatchQualificationResult`, `IndividualBatchResult`, `AggregateBatchSummary`
- `BatchQualificationRunner` — sequential multi-model qualification
- 5 unit tests + 15 integration tests
- Closure validation: 624 total tests pass
- Full regression suite verification

**Gate Criteria:**

| Gate | Requirement | Result |
|------|-------------|--------|
| H6-1 | Batch accepts multiple qualification targets | ✅ PASS |
| H6-2 | Sequential execution with bounded resource behavior | ✅ PASS |
| H6-3 | Individual results independently addressable | ✅ PASS |
| H6-4 | Aggregated results preserve evidence provenance | ✅ PASS |
| H6-5 | One failed model does not corrupt unrelated results | ✅ PASS |
| H6-6 | Full regression suite passes (624 tests) | ✅ PASS |
| H6-7 | No authority escalation paths introduced | ✅ PASS |
| H6-8 | Evidence chain remains deterministic | ✅ PASS |
| H6-9 | Router behavior remains policy-controlled | ✅ PASS |
| H6-10 | Epic completion report generated | ✅ PASS |

**Tests:** 5 unit + 15 integration = 20 tests  
**Files:** `src/canonical/qualification/batch.rs`, `tests/batch_qualification_test.rs`  
**Report:** `reports/sprint-mqr-h6-completion.md`

---

## Sprint Summary

| Sprint | Deliverable | Tests | Gates | Status |
|--------|-------------|-------|-------|--------|
| H1 | Live HTTP Bridge Integration | 35 | 5 | SEALED |
| H2 | Persistent Router Registry State | 39 | 6 | SEALED |
| H3 | Comparative Registry Persistence | 24 | 5 | SEALED |
| H4 | Custom Validator Rule Execution | 20 | 5 | SEALED |
| H5 | Custom Validation Evidence Integration | 25 | 5 | SEALED |
| H6 | Local Batch Qualification + Closure Validation | 20 | 10 | SEALED |
| **Total** | | **163** | **40** | **6/6 SEALED** |

---

## Dependencies

```
H1 → H2 → H3 → H4 → H5 → H6
```

Sequential: each sprint builds on the previous. H1 proves transport; H2 proves persistence; H3 extends persistence to comparative records; H4 adds governed extensibility; H5 adds retest policy; H6 adds batch capability.

---

## After Hardening: Pilot Epic

Once H1-H6 are all sealed, the pilot epic (MRQ-P1 through MRQ-P10) begins for actual model roster qualification against real hardware.
