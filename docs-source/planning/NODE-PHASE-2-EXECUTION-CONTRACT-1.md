# NODE-PHASE-2-EXECUTION-CONTRACT-1

**Status:** Planning Document  
**Scope:** Frozen contracts, state machines, authority boundaries, provenance rules, confidence semantics, negative tests, UI expectations, and adversarial test plan for the remaining Phase 2 subsystems  
**Constraint:** Planning-only sprint — no implementation code  
**Date:** 2026-07-16

---

## Table of Contents

1. Subsystem Contracts
   - 1.1 Pattern Escalation
   - 1.2 Reconciliation Architecture
   - 1.3 Recovery Custody
2. Authority Boundary Diagram
3. Provenance Rules
4. Confidence Semantics
5. State Machines
6. Negative Tests
7. UI Expectations
8. Adversarial Test Plan
9. Implementation Order

---

## 1. Subsystem Contracts

### 1.1 Pattern Escalation

#### What Constitutes a "Pattern"

A pattern is detected when N findings of the same classification category occur within a rolling T time window, scoped to the same context (node, workload type, capability type). The three dimensions of a pattern are:

- **Category**: one of `performance_degradation`, `capability_mismatch`, `repeated_failure`, `allocation_drift`, `node_instability`
- **Context**: the affected entity (node_id, workload_type, capability_type)
- **Time window**: rolling window measured from the earliest finding in the set

#### Minimum Finding Threshold and Time Window

| Parameter | Default | Configurable | Type |
|-----------|---------|--------------|------|
| `min_findings` | 3 | Yes | u32 |
| `time_window_seconds` | 86400 (24h) | Yes | u64 |
| `max_active_patterns` | 100 | Yes | u32 |

Configuration is loaded from `pattern_config.json` at startup. If the file is absent, defaults are used. Owner can modify configuration through `set_pattern_config()` — this is a metadata change that produces a receipt, not a mutation of any operational state.

#### Multi-Node vs. Per-Node

Patterns are **per-node**. A pattern on Node A cannot reference findings on Node B. This prevents cross-node correlation from becoming a covert scheduling signal.

Explicit allowance: if the codebase later introduces fleet-level aggregation, those aggregates must live in a separate `FleetPattern` type that goes through owner review before surfacing. FleetPattern may never trigger any action.

#### Expiration

Patterns expire after `max_idle_seconds` (default: 604800 = 7 days) with no new matching findings. Expired patterns transition to `Resolved` with receipt reason `"expired"`.

A pattern in `Monitoring` state that receives no new findings for the full idle window auto-transitions to `Resolved`. The owner is notified (via the pattern appearing in the insight report) but no automatic cleanup occurs — the receipt is produced and the pattern is flagged as resolved.

#### Overlap

Overlap occurs when a single finding could match multiple pattern definitions (e.g., same finding matches both a `repeated_failure` pattern and a `node_instability` pattern). **Resolution: each finding is assigned to exactly one pattern.** The assignment priority is:

1. `node_instability` (highest severity)
2. `repeated_failure`
3. `performance_degradation`
4. `allocation_drift`
5. `capability_mismatch`

A finding that triggers a new pattern while an existing pattern is still open for the same category+context will be merged into the existing pattern (extending its window), unless the existing pattern is in `Resolved` or `Dismissed` state, in which case a new pattern is created.

#### State Machine

```
Detected → Pending Review → Acknowledged → Monitoring → Resolved
                                                     → Dismissed
```

| Transition | Trigger | Receipt Produced | Owner Action Required |
|-----------|---------|-----------------|-----------------------|
| → Detected | `min_findings` reached within `time_window_seconds` | `PatternDetectionReceipt` | No |
| Detected → Pending Review | Automatic after detection | `PatternPendingReceipt` | No |
| Pending Review → Acknowledged | Owner acknowledges | `PatternAckReceipt` | Yes |
| Acknowledged → Monitoring | Automatic | `PatternMonitoringReceipt` | No |
| Monitoring → Resolved | Owner resolves OR pattern expires | `PatternResolvedReceipt` | Yes (owner resolve) or No (expiry) |
| Monitoring → Dismissed | Owner dismisses | `PatternDismissedReceipt` | Yes |
| Any → Resolved | Pattern expired (`max_idle_seconds` elapsed) | `PatternExpiredReceipt` | No |

#### Receipt Types

Each receipt carries:
- `pattern_id`, `pattern_category`, `context`
- Previous and new state
- Finding IDs in the pattern
- Timestamp
- Actor (owner identity if owner action, `"system"` if automatic)

#### Explicitly NOT Allowed

| Forbidden Path | Enforcement |
|---------------|-------------|
| Patterns cannot trigger allocation changes | No `AllocationService` dependency in pattern escalation module |
| Patterns cannot modify thresholds | No `set_threshold` call from pattern service |
| Patterns cannot create sessions | No `SessionService` or `WorkloadSessionService` dependency |
| Patterns cannot call `dispatch` or `schedule` | No scheduling primitives in contract or service |
| Patterns cannot modify bootstrap plans | No `BootstrapService` dependency |
| Patterns cannot create workloads | No workload creation method call |
| Patterns cannot be used to score or rank nodes | Pattern output is not consumed by allocation scoring |
| Patterns cannot escalate beyond owner | No cross-owner notification or external alerting |
| Pattern config changes cannot affect active patterns retroactively | Config change applies to new patterns only; active patterns use config at detection time |

---

### 1.2 Reconciliation Architecture

#### What Is Compared

Reconciliation compares the local node's persisted state against a last-known-good (LKG) reference. The LKG is either:
- The last successful reconciliation receipt (if one exists)
- The node's own custody chain head (if no prior reconciliation)

#### Participating Artifacts

| Artifact | Source | Compared Field |
|----------|--------|----------------|
| **Sessions** | `SessionService` | session_id, state, timestamps |
| **Custody envelopes** | `CustodyService` | envelope_id, chain_hash, receipt_hash, previous_envelope_hash |
| **Receipts** | All service receipt stores | receipt_id, receipt_type, timestamp |
| **Identity** | `NodeIdentityService` | node_id, display_name, platform, runtime_version |
| **Registration** | `RegistrationService` | node_id, registration_status |
| **Capabilities** | `CapabilityEvidenceBridge` | capability_type, claim_id, verification_status |

#### Difference Classification

| Classification | Description | Auto-Accept? |
|---------------|-------------|--------------|
| `missing_envelope` | Custody envelope in LKG but absent locally | No |
| `divergent_hash` | Envelope exists but hash differs | No |
| `orphan_session` | Session present locally but absent from LKG | No |
| `incomplete_receipt` | Receipt referenced in LKG but absent locally | No |
| `state_mismatch` | Artifact exists in both but state field differs | No |

**No difference type is auto-accepted.** All differences require owner review.

#### Exception Path

| Path | Behavior |
|------|----------|
| **Quarantine** | Differences are flagged and isolated. The reconciliation report is written to disk. The node continues operating with its current state. No merge occurs. |
| **Owner Override** | Owner explicitly accepts or rejects each classified difference. A `ReconciliationDecision` receipt is produced for each override. |

#### Receipts Produced

| Receipt | When |
|---------|------|
| `ReconciliationStartedReceipt` | Reconciliation begins |
| `ReconciliationReportReceipt` | Report generated (differences classified) |
| `ReconciliationAcceptReceipt` | Owner accepts a difference |
| `ReconciliationOverrideReceipt` | Owner overrides a difference |
| `ReconciliationQuarantineReceipt` | Differences quarantined |
| `ReconciliationCompleteReceipt` | All decisions applied, reconciliation closed |

#### Integration with Custody

- Reconciliation does not modify the custody chain
- Reconciliation receipts are appended to the custody chain (the act of reconciling is custodied, not the result of the merge)
- The reconciliation LKG reference is the custody chain head hash
- `verify_integrity()` is called before reconciliation begins; if integrity check fails, reconciliation enters `Quarantine` state automatically

#### Graceful Degradation When Core Is Unavailable

- Reconciliation is purely local — no Core dependency
- If LKG reference is absent (first-ever reconciliation), the node reports all artifacts as "present only in local state" with classification `orphan_session` / `missing_envelope` (as appropriate)
- If custody service is unavailable, reconciliation reports an error and exits; no partial state is produced

#### Explicitly NOT Allowed

| Forbidden Path | Enforcement |
|---------------|-------------|
| Silent merge (merge without owner review) | All classified differences require `ReconciliationAcceptReceipt` or `ReconciliationOverrideReceipt` |
| Bypassing custody | Reconciliation reads custody chain but never calls `append_receipt` directly — only the reconciliation service's own receipt method writes to custody |
| Erasing history | Reconciliation produces new receipts; it never deletes envelopes, sessions, or receipts |
| Automatic repair | No `rollback`, `replay`, or `resume` call in reconciliation service |
| Reconciliation mutating state during comparison | Comparison is read-only; mutation only occurs after owner decision |
| Cross-node reconciliation | Reconciliation is per-node; no fleet-level reconciliation in this sprint |

---

### 1.3 Recovery Custody

#### State Machine

```
Healthy → Suspect → Reconciling → Owner Review → Recovered
                                              → Failed
```

| Transition | Trigger | Receipt/Evidence Produced | Owner Action Required |
|-----------|---------|--------------------------|-----------------------|
| Healthy → Suspect | Integrity check fails OR custody service reports anomaly | `SuspectFlagReceipt` | No |
| Suspect → Reconciling | Owner initiates recovery OR system auto-initiates after configurable delay (default: 300s) | `RecoveryStartedReceipt` | No |
| Reconciling → Owner Review | Reconciliation report generated with differences | `RecoveryReportReceipt` with full diff summary | No |
| Owner Review → Recovered | Owner explicitly accepts recovered state | `RecoveryAcceptedReceipt` | Yes |
| Owner Review → Failed | Owner rejects recovered state OR recovery encounters unrecoverable error | `RecoveryFailedReceipt` | Yes |
| Suspect → Healthy | Owner clears suspect flag without recovery (dismisses suspicion) | `SuspectClearedReceipt` | Yes |
| Reconciling → Failed | Unrecoverable error during comparison (custody unavailable, disk I/O error) | `RecoveryFailedReceipt` | No (system) |

#### Flagging Recovered State

All envelopes, sessions, and records produced or modified during recovery carry a metadata flag:

```
recovery_status: "normal" | "recovered" | "suspect"
```

- `normal`: state produced during normal operation
- `recovered`: state produced or accepted during a recovery cycle
- `suspect`: state flagged by integrity check but not yet recovered

Recovered state is **distinguishable from normal state** in all queries. The owner is shown the `recovery_status` in the UI. Recovered state is visually distinct (see UI Expectations).

#### Evidence Produced During Recovery

| Evidence | Description |
|----------|-------------|
| `SuspectFlagReceipt` | Records what triggered the suspect state (which integrity check failed, what anomaly was detected) |
| `RecoveryStartedReceipt` | Records when recovery began, what state was current at start |
| `RecoveryReportReceipt` | Full diff of what was found during recovery: corrupted envelopes, missing sessions, hash mismatches |
| `RecoveryAcceptedReceipt` | Owner decision to accept recovered state; includes before/after hashes |
| `RecoveryFailedReceipt` | Owner decision to reject or system failure; includes reason |
| `SuspectClearedReceipt` | Owner decision to clear suspect flag without recovery |

All recovery receipts are chainable to the custody chain.

#### What Requires Explicit Owner Decision

| Decision | Required At |
|----------|-------------|
| Accept recovered state | Owner Review → Recovered |
| Reject recovered state / mark as failed | Owner Review → Failed |
| Clear suspect flag without recovery | Suspect → Healthy |
| Approve any state mutation during recovery | Each individual accept/override of a recovery action |

#### Explicitly NOT Allowed

| Forbidden Path | Enforcement |
|---------------|-------------|
| Automatic rollback | No `rollback()` method exists; all state changes require owner decision |
| Recovery-triggered workload resumption | No workload creation, activation, or completion in recovery service |
| Cross-node recovery | Recovery is per-node; no fleet recovery coordination |
| Recovery mutating custody chain directly | Recovery produces its own receipts via `append_receipt` but never modifies existing envelopes |
| Recovery erasing history | Recovery creates new envelopes for recovered state; original corrupted envelopes are preserved and flagged |
| Recovery auto-accepting any state | Every transition to Recovered requires owner decision receipt |
| Recovery creating sessions | No `SessionService` dependency in recovery service |
| Recovery modifying allocation or bootstrap state | Recovery scope is custody chain + session integrity only |

---

## 2. Authority Boundary Diagram

### Invariant Chain

```
Observation
    ↓
Finding
    ↓
Classification
    ↓
Pattern
    ↓
Insight
========== NO STATE CHANGE ==========
Owner Decision
========== State Change ==========
```

### Where Each Subsystem Sits

| Subsystem | Position | Boundary |
|-----------|----------|----------|
| Evidence Intelligence | Finding generation | Returns findings, never mutates state |
| Classification | Finding → Classification | Assigns categories, never mutates operational state |
| Anomaly Detection | Observation → Finding | Produces deviation observations, never acts on them |
| **Pattern Escalation** | Classification → Pattern | Groups classified findings, never triggers action |
| Owner Insight Surface | Pattern → Insight | Aggregates all intelligence into owner-facing reports |
| **Reconciliation** | Below the line (read-only comparison) | Produces reports; mutations require owner decision |
| **Recovery Custody** | Below the line (state reconstruction) | All state changes require owner decision |

### Forbidden Paths (Compile-Time / Runtime Enforcement)

| Path | Why Forbidden | Enforcement Mechanism |
|------|--------------|----------------------|
| Intelligence → Action | Finding data used to dispatch work | No dispatch call in any intelligence service; return types are data-only |
| Classification → Allocation | Category influences allocation score | Classification service returns `ClassifiedFinding`; allocation service does not import classification |
| Pattern → Session creation | Pattern detection creates new sessions | Pattern service has no `SessionService` dependency |
| Anomaly → Threshold modification | Deviation detection changes thresholds | `AnomalyDetectionService.set_threshold()` is owner-gated through owner workflow |
| Reconciliation → State mutation | Report directly mutates state | `ReconcileService` produces report; mutation only occurs through `submit_decision()` |
| Recovery → Workload resumption | Recovery auto-restarts workloads | Recovery service has no workload service dependency |
| Insight → Bootstrap | Insight recommendations become bootstrap plans | Owner must separately create bootstrap plans; insight is informational |

### Enforcement Mechanisms

1. **Dependency injection boundary**: Pattern Escalation module cannot import `SessionService`, `WorkloadSessionService`, `AllocationService`, or `BootstrapService` in its Cargo.toml or `use` statements
2. **Return type boundary**: All intelligence functions return data types; no function returns a `Result<_, _>` whose `Ok` variant includes a mutation handle
3. **Owner gate**: Every mutation path goes through `OwnerWorkflowService.submit_decision()`
4. **Compile-time check**: The boundary is enforced by the Rust module system; a pattern service that tries to call `session_service.create_session()` will not compile
5. **Runtime assertion (debug builds)**: If any intelligence-type module holds a reference to a mutation service, a debug assertion fires at service construction time

### Regression Test to Prove the Boundary Holds

**Test:** `test_pattern_service_cannot_access_allocation()` — instantiate `PatternEscalationService` with only its allowed dependencies (classification service, config, clock). Assert that `AllocationService`, `SessionService`, `WorkloadSessionService`, and `BootstrapService` are not present in its dependency list. This test lives in the pattern escalation module and imports only the allowed types.

---

## 3. Provenance Rules

Every intelligence object must answer: which receipts, workloads, sessions, custody envelopes, and capabilities generated it.

### Classification Findings

| Field | Source |
|-------|--------|
| `finding_id` | Generated at classification time |
| `category` | From `FindingCatalog` |
| `severity` | Derived from context + catalog default |
| `confidence` | Derived from `source_references` count |
| `evidence_references` | Receipt IDs, workload IDs that produced the original intelligence finding |
| `source_references` | The `IntelligenceFinding.source_references` field (workload IDs, session IDs) |
| `detection_method` | The intelligence analysis method that produced the raw finding |

### Anomaly Observations

| Field | Source |
|-------|--------|
| `observation_id` | Generated at deviation detection |
| `metric_name` | From `AnomalyThreshold` definition |
| `context` | Baseline context string (e.g., `"workload_type:inference"`) |
| `baseline_mean`, `baseline_std_dev` | From `BaselineRecord` |
| `observed_value` | Supplied at detection call time |
| `evidence_workload_ids` | Workload IDs passed at detection call time (links back to workload sessions) |

### Pattern Detections

| Field | Source |
|-------|--------|
| `pattern_id` | Generated at detection |
| `finding_ids` | The `ClassifiedFinding.finding_id` values that triggered the pattern |
| `category` | Shared classification category across the findings |
| `context` | Shared context (node_id, workload_type) across the findings |
| `time_window` | `min(time_window_start)` to `max(time_window_end)` across findings |
| `evidence_references` | Union of all `evidence_references` from constituent findings |
| `detection_receipt_id` | The `PatternDetectionReceipt` envelope ID in custody |

### Insight Reports

| Field | Source |
|-------|--------|
| `report_id` | Generated at report generation |
| `dashboard.findings_summary` | `EvidenceClassificationService.get_findings_summary()` |
| `detailed_findings` | All `ClassifiedFinding` values |
| `detailed_anomalies` | All `AnomalyFinding` values from `scan_all_metrics()` |
| `workload_breakdown` | `EvidenceIntelligenceService.analyze_workload_outcomes()` |
| `capability_breakdown` | `EvidenceIntelligenceService.analyze_capability_effectiveness()` |
| `allocation_breakdown` | `EvidenceIntelligenceService.analyze_allocation_accuracy()` |
| `recommendations` | Derived from findings, anomalies, and breakdowns |

### Reconciliation Reports

| Field | Source |
|-------|--------|
| `report_id` | Generated at reconciliation |
| `lkg_reference` | Last-known-good custody chain head hash |
| `differences` | Each classified difference references the specific artifact (session_id, envelope_id, receipt_id) |
| `custody_snapshot` | A hash of the custody chain at reconciliation start |
| `owner_decisions` | Links to `ReconciliationAcceptReceipt` or `ReconciliationOverrideReceipt` |

### Recovery Receipts

| Field | Source |
|-------|--------|
| Each recovery receipt type | Generated at each state transition |
| `previous_state` | State machine state before transition |
| `new_state` | State machine state after transition |
| `recovery_evidence` | Links to the reconciliation report or integrity report that triggered recovery |
| `affected_artifacts` | List of envelope IDs, session IDs touched during recovery |

---

## 4. Confidence Semantics

| Level | Meaning | Evidence Requirements | Authority |
|-------|---------|----------------------|-----------|
| **Low** | Limited evidence; single observation | Single observation, 1–4 finding source references, or baseline with <10 samples | Informational only |
| **Medium** | Multiple corroborating observations | 2+ independent observations, 5–9 finding source references, or baseline with ≥10 samples | Informational only |
| **High** | Strong repeated evidence across sessions | Consistent pattern across 3+ sessions, ≥10 finding source references, or anomaly with deviation factor >3.0 | Informational only |
| **Confirmed** | Verified against authoritative state | Cross-validated with custody chain; integrity check passes; pattern resolved by owner | Informational only |

### Mapping to Existing Code

The existing `confidence_from_evidence_count()` in `evidence_classification_service.rs` maps:
- `count >= 10` → `"high"`
- `count >= 5` → `"medium"`
- `count >= 1` → `"low"`
- `count == 0` → `"low"`

This sprint formalizes that mapping and extends it to patterns, anomalies, and recovery:

| Object Type | Confidence Source |
|-------------|------------------|
| ClassifiedFinding | Evidence reference count (existing) |
| AnomalyFinding | Deviation factor magnitude + sample count |
| Pattern | Number of constituent findings + time window density |
| InsightReport | Not applicable (aggregate — individual findings carry confidence) |
| ReconciliationReport | Not applicable (factual diff — owner decides) |
| RecoveryReceipt | Not applicable (state machine transition — receipt is factual) |

### Critical Rule

**Confidence is about evidence quality, not correctness. Confidence never implies authority to act.**

- A `Confirmed` pattern still requires owner review to escalate into any action.
- Confidence level may not be used as a gate for automatic behavior.
- Confidence metadata is presentational only.

---

## 5. State Machines

### 5.1 Pattern Lifecycle

```
                     ┌─────────────────────────────────────────────┐
                     │                                             │
                     ▼                                             │
Detected ──→ Pending Review ──→ Acknowledged ──→ Monitoring ──────┤──→ Resolved
                                                       │          │
                                                       └──────────┼──→ Dismissed
                                                                  │
                                             (expiry from any     │
                                              active state) ──────┘
```

| Transition | Trigger | Receipt | Owner Action |
|-----------|---------|---------|--------------|
| → Detected | `min_findings` threshold met in time window | `PatternDetectionReceipt` | No |
| Detected → Pending Review | Automatic (immediate after detection) | `PatternPendingReceipt` | No |
| Pending Review → Acknowledged | Owner action `"acknowledge"` | `PatternAckReceipt` | **Yes** |
| Acknowledged → Monitoring | Automatic (immediate after acknowledgement) | `PatternMonitoringReceipt` | No |
| Monitoring → Resolved | Owner action `"resolve"` | `PatternResolvedReceipt` | **Yes** |
| Monitoring → Dismissed | Owner action `"dismiss"` | `PatternDismissedReceipt` | **Yes** |
| Any active state → Resolved | Expiry (`max_idle_seconds` elapsed) | `PatternExpiredReceipt` | No |

### 5.2 Reconciliation Lifecycle

```
Offline ──→ Reconnect ──→ Compare ──→ Validate ──→ Review ──→ Accept
                                                    │
                                                    └──→ Exception ──→ Quarantine ──→ Owner Override
```

| Transition | Trigger | Receipt | Owner Action |
|-----------|---------|---------|--------------|
| → Offline | Node starts or custody integrity check fails | N/A (state is implied) | No |
| Offline → Reconnect | Owner initiates reconciliation OR node detects reconnection after Core availability change | `ReconciliationStartedReceipt` | No |
| Reconnect → Compare | Automatic after reconnection | `ReconciliationStartedReceipt` (updated) | No |
| Compare → Validate | Automatic after comparison completes | `ReconciliationReportReceipt` | No |
| Validate → Review | Differences found during comparison | `ReconciliationReportReceipt` (final) | No |
| Validate → Accept | No differences found | `ReconciliationCompleteReceipt` | No |
| Review → Accept | All differences accepted by owner | `ReconciliationAcceptReceipt` (per difference) + `ReconciliationCompleteReceipt` | **Yes** |
| Review → Exception | Owner chooses to quarantine | `ReconciliationQuarantineReceipt` | **Yes** |
| Exception → Quarantine | System isolates differences | `ReconciliationQuarantineReceipt` (final) | No |
| Quarantine → Owner Override | Owner explicitly overrides each quarantined item | `ReconciliationOverrideReceipt` (per item) | **Yes** |
| Quarantine → Accept | Owner accepts quarantined items | `ReconciliationAcceptReceipt` (per item) | **Yes** |

### 5.3 Recovery Lifecycle

```
Healthy ──→ Suspect ──→ Reconciling ──→ Owner Review ──→ Recovered
                                               │
                                               └──→ Failed
```

| Transition | Trigger | Receipt | Owner Action |
|-----------|---------|---------|--------------|
| → Healthy | Normal operation | N/A | No |
| Healthy → Suspect | Integrity check fails OR custody anomaly detected | `SuspectFlagReceipt` | No |
| Suspect → Reconciling | Owner initiates recovery OR auto-initiate after delay | `RecoveryStartedReceipt` | No (auto) or **Yes** (manual) |
| Reconciling → Owner Review | Reconciliation report generated with differences | `RecoveryReportReceipt` | No |
| Owner Review → Recovered | Owner accepts recovered state | `RecoveryAcceptedReceipt` | **Yes** |
| Owner Review → Failed | Owner rejects OR unrecoverable error | `RecoveryFailedReceipt` | **Yes** (owner) or No (system error) |
| Suspect → Healthy | Owner clears suspect flag (dismiss) | `SuspectClearedReceipt` | **Yes** |
| Reconciling → Failed | Unrecoverable error | `RecoveryFailedReceipt` | No |

---

## 6. Negative Tests

Architectural regression tests that verify forbidden paths cannot be crossed.

| # | Test Name | What It Proves | Subsystem |
|---|-----------|---------------|-----------|
| N-01 | `intelligence_cannot_mutate_workload_state` | No mutation method in intelligence service — all intelligence functions return data types only | All Phase 2 |
| N-02 | `pattern_detection_cannot_schedule_work` | No dispatch, schedule, or timer primitive in pattern service | Escalation |
| N-03 | `anomaly_detection_cannot_modify_thresholds_without_owner_approval` | `set_threshold()` is gated through owner workflow; cannot be called from anomaly detection scan path | Anomaly |
| N-04 | `pattern_escalation_cannot_create_allocations` | No `AllocationService` import in pattern escalation module | Escalation |
| N-05 | `reconciliation_cannot_silently_merge_conflicts` | All classified differences require owner decision receipt; no `auto_accept` field exists | Reconciliation |
| N-06 | `reconciliation_cannot_bypass_custody` | Custody is read-only during comparison; no `append_receipt` call during comparison phase | Reconciliation |
| N-07 | `recovery_cannot_erase_history` | Recovery produces new envelopes; original envelopes are preserved and flagged as `suspect` or `recovered` | Recovery |
| N-08 | `evidence_cannot_be_rewritten` | No `update_finding()` or `update_envelope()` method exists on evidence types | All |
| N-09 | `pattern_service_no_session_dependency` | Compile-time check: `PatternEscalationService` constructor does not accept `SessionService` | Escalation |
| N-10 | `recovery_service_no_workload_dependency` | `RecoveryService` constructor does not accept `WorkloadSessionService` | Recovery |
| N-11 | `reconciliation_no_mutation_during_compare` | `ReconcileService.compare()` is a pure function; its return type contains no mutation handles | Reconciliation |
| N-12 | `owner_decision_required_for_all_state_transitions` | Every mutation path in the system calls `submit_decision()`; no bypass path exists | All |
| N-13 | `pattern_config_change_not_retroactive` | Changing `min_findings` or `time_window_seconds` does not affect active patterns; active patterns use config at detection time | Escalation |
| N-14 | `recovered_state_is_flagged` | After recovery, `recovery_status` is `"recovered"` on all affected artifacts; verified by query after recovery test | Recovery |
| N-15 | `reconciliation_no_cross_node` | `ReconcileService.compare()` accepts a single node_id; no fleet iterator is passed | Reconciliation |

---

## 7. UI Expectations

### 7.1 Pattern Escalation

#### Endpoint: `GET /patterns`

**Dashboard display:**
- Table of active patterns, grouped by status (Detected, Pending Review, Acknowledged, Monitoring)
- Each pattern row shows: category icon, context (node/workload type), finding count, time window, confidence, status badge
- Severity is derived from the highest-severity constituent finding
- Total: "3 active patterns, 1 requires review"

**Owner click:**
- Click a pattern row → expand to show constituent findings (finding_id, title, severity, generated_at)
- Click "Acknowledge" button on a `Pending Review` pattern → modal with optional note
- Click "Resolve" or "Dismiss" on a `Monitoring` pattern → confirmation modal

**Receipt appears:**
- Pattern list updates optimistically; receipt appears in "Recent Receipts" sidebar
- Receipt summary: "Pattern 'repeated_failure' on node-a acknowledged by owner"

#### Endpoint: `GET /patterns/config`

**Dashboard display:**
- Read-only view of current config: `min_findings`, `time_window_seconds`, `max_idle_seconds`
- Owner can modify and submit; change produces a receipt
- "Changes apply to new patterns only. Active patterns: 3 (unaffected)"

**Owner click:**
- "Edit Config" button → inline editor with validation
- Submit → confirmation modal noting that active patterns are unaffected

**Receipt appears:**
- "Pattern escalation config updated: min_findings 3→5, time_window_seconds 86400→172800"

### 7.2 Reconciliation

#### Endpoint: `POST /reconcile`

**Dashboard display:**
- "Reconcile Now" button
- Before clicking: status indicator showing "Last reconciliation: 2026-07-15 14:30 UTC (no differences)"
- After clicking: progress spinner with phase indicator ("Comparing...", "Validating...", "Generating report...")

**Owner click:**
- "Reconcile Now" → triggers full reconciliation cycle
- If differences found: report panel slides in with classified differences grouped by type
- Each difference shows: artifact type (session/envelope/receipt), artifact ID, expected state, actual state, classification tag
- Owner can "Accept" or "Override" each difference individually, or "Accept All" / "Quarantine All"

**Receipt appears:**
- "Reconciliation complete: 2 missing_envelope, 1 orphan_session — all accepted by owner"
- Failed: "Reconciliation quarantined: 3 differences require override"

#### Endpoint: `GET /reconcile/history`

**Dashboard display:**
- Chronological list of past reconciliation receipts
- Each entry shows: timestamp, difference count (by type), outcome (Accept/Quarantine/Failed)

### 7.3 Recovery Custody

#### Endpoint: `GET /recovery/status`

**Dashboard display:**
- Status indicator: `Healthy` (green), `Suspect` (yellow), `Reconciling` (blue), `Owner Review` (orange), `Recovered` (green with "recovered" tag), `Failed` (red)
- Below indicator: summary of current recovery state
- If `Suspect`: "Custody integrity check failed at 2026-07-16 10:00 UTC. 1 envelope flagged."
- If `Owner Review`: "Recovery report ready. 2 differences require your decision."

**Owner click:**
- If `Suspect`: "Begin Recovery" button or "Clear Suspect Flag" button
- If `Owner Review`: "Review Recovery Report" → shows diff similar to reconciliation report
- Accept or Reject buttons
- If `Recovered`: "View Recovery Evidence" → expands to show all recovery receipts

**Receipt appears:**
- "Recovery accepted: 2 custody envelopes flagged as recovered"
- "Recovery failed: Owner rejected recovery of 1 custody envelope"

#### Endpoint: `GET /recovery/evidence`

**Dashboard display:**
- Full recovery evidence chain: SuspectFlag → RecoveryStarted → RecoveryReport → RecoveryAccepted/Failed
- Each evidence entry links to its custody envelope

---

## 8. Adversarial Test Plan

| # | Test Name | Attack Vector | Expected Behavior |
|---|-----------|---------------|-------------------|
| A-01 | `contradictory_evidence` | Feed conflicting data into pattern detection (e.g., 5 findings of `performance_degradation` and 5 findings of `capability_mismatch` for the same context within the same time window) | Pattern detection reports each category separately; no interaction or cross-category escalation; no action triggered |
| A-02 | `false_positive_anomalies` | Generate 1000 noise metric observations with random values across all metrics | Anomaly detection creates findings for each deviation; no automatic threshold adjustment, no action triggered | 
| A-03 | `disconnect_during_reconciliation` | Remove custody persistence file mid-sync during reconciliation | Reconciliation detects missing custody state, reports error, enters Quarantine state; no data loss; original custody state is recoverable from backup (if configured) or flagged as suspect |
| A-04 | `tampered_custody_records` | Modify a custody envelope file on disk (change receipt_payload, chain_hash, or receipt_hash) | `verify_integrity()` detects tampered payload; `SuspectFlagReceipt` produced; node does not use tampered chain; reports integrity error with details |
| A-05 | `simultaneous_multi_node_edits` | Present conflicting state from two different reconciliation sources (simulate two LKG references with divergent hashes) | Reconciliation flags conflict at the comparison stage; requires owner decision; no automatic resolution |
| A-06 | `stale_baselines` | Force outdated baseline data (set baseline timestamps to 90 days ago) | Anomaly detection notes stale baseline (`baseline_stale` field on `DeviationObservation`); produces findings but flags them as `low` confidence with note "baseline may be stale"; no action triggered |
| A-07 | `cyclic_provenance_references` | Create circular evidence chain (envelope A links to envelope B, envelope B links back to envelope A) | Provenance query detects cycle during `get_provenance_graph()`; reports cycle in graph; does not enter infinite loop (graph construction is bounded by envelope count) |
| A-08 | `escalation_into_action` | Attempt to route pattern detection output to an execution path (bypass the owner decision boundary) | Compile-time enforcement: pattern service cannot import execution services. Runtime test: attempt to construct pattern service with invalid dependencies → compilation fails |
| A-09 | `recovery_overwrite_good_state` | Attempt to recover a healthy custody chain by running recovery on an intact chain | Recovery detects no integrity errors; reconciliation finds no differences; recovery exits at Reconciling phase with note "no discrepancies found"; no state is modified |
| A-10 | `pattern_flood` | Create 10,000 findings rapidly across all categories in a 1-second window | Pattern detection respects `max_active_patterns` (default 100); excess patterns are not created; findings are still stored and classified but pattern detection logs warning; no OOM or crash |
| A-11 | `reconciliation_with_zero_state` | Run reconciliation on a freshly initialized node with no sessions, no custody chain, no receipts | Reconciliation reports all LKG fields as absent; differences classified as `missing_envelope` for all expected artifacts; report is produced; owner can accept ("fresh start") or quarantine |
| A-12 | `recovery_suspect_clear_without_investigation` | Clear suspect flag without running recovery on a demonstrably corrupted chain | System produces `SuspectClearedReceipt`; suspect flag is cleared; owner action is logged; integrity check is NOT re-run (owner explicitly chose to dismiss) |

---

## 9. Implementation Order

### Recommended Sprint Sequence

```
NODE-PATTERN-ESCALATION-1
        │
        ▼
NODE-RECONCILIATION-ARCHITECTURE-1    (planning sprint — does not write service code)
        │
        ▼
NODE-RECONCILIATION-FOUNDATION-1
        │
        ▼
NODE-RECOVERY-CUSTODY-1
```

### Sprint: NODE-PATTERN-ESCALATION-1

| Field | Value |
|-------|-------|
| **Subsystem contracts implemented** | Pattern Escalation (§1.1) |
| **State machine implemented** | Pattern lifecycle (Detected → Pending Review → Acknowledged → Monitoring → Resolved/Dismissed) |
| **Contracts to create** | `librarian-contracts/src/pattern_escalation/` — `pattern.rs`, `receipt.rs`, `config.rs`, `mod.rs` |
| **Service to create** | `librarian-node/src/node/pattern_escalation_service.rs` |
| **Negative tests to pass** | N-02, N-04, N-09, N-12, N-13 |
| **Adversarial tests to pass** | A-01, A-08, A-10 |
| **Phase 2 Safety Gate** | Verify pattern service has no dependency on `AllocationService`, `SessionService`, `WorkloadSessionService`, or `BootstrapService` |
| **Dependencies** | Evidence Classification (completed), Anomaly Detection (completed) |

### Sprint: NODE-RECONCILIATION-ARCHITECTURE-1

| Field | Value |
|-------|-------|
| **Purpose** | Design the reconciliation protocol, LKG reference format, sync state machine, and difference classification taxonomy before writing implementation code |
| **Subsystem contracts implemented** | Reconciliation Architecture (§1.2) — planning only, no service code |
| **Output** | `docs/planning/NODE-RECONCILIATION-ARCHITECTURE-1.md` |
| **Dependencies** | None (can run in parallel with NODE-PATTERN-ESCALATION-1) |

### Sprint: NODE-RECONCILIATION-FOUNDATION-1

| Field | Value |
|-------|-------|
| **Subsystem contracts implemented** | Reconciliation Foundation (§1.2) |
| **State machine implemented** | Reconciliation lifecycle (Offline → Reconnect → Compare → Validate → Review → Accept / Exception → Quarantine → Owner Override) |
| **Contracts to create** | `librarian-contracts/src/reconciliation/` — `report.rs`, `difference.rs`, `receipt.rs`, `config.rs`, `mod.rs` |
| **Service to create** | `librarian-node/src/node/reconciliation_service.rs` |
| **Negative tests to pass** | N-05, N-06, N-11, N-12, N-15 |
| **Adversarial tests to pass** | A-03, A-05, A-11 |
| **Phase 2 Safety Gate** | Verify reconciliation service does not call `append_receipt` during comparison phase; verify all difference classifications require owner decision |
| **Dependencies** | NODE-RECONCILIATION-ARCHITECTURE-1 |

### Sprint: NODE-RECOVERY-CUSTODY-1

| Field | Value |
|-------|-------|
| **Subsystem contracts implemented** | Recovery Custody (§1.3) |
| **State machine implemented** | Recovery lifecycle (Healthy → Suspect → Reconciling → Owner Review → Recovered/Failed) |
| **Contracts to create** | `librarian-contracts/src/custody_recovery/` — `state.rs`, `receipt.rs`, `evidence.rs`, `mod.rs` |
| **Service to create** | `librarian-node/src/node/custody_recovery_service.rs` |
| **Negative tests to pass** | N-07, N-10, N-12, N-14 |
| **Adversarial tests to pass** | A-04, A-09, A-12 |
| **Phase 2 Safety Gate** | Verify recovery service has no dependency on workload services; verify `recovery_status` flag is present on all recovered artifacts; verify original corrupted envelopes are preserved (not deleted) |
| **Dependencies** | NODE-RECONCILIATION-FOUNDATION-1 (reconciliation report drives recovery decisions) |

### Phase 2 Safety Gate Verification (All Sprints)

Each sprint must pass the following verification before closeout:

| Gate | Verification |
|------|-------------|
| No dependency crossing | All intelligence/pattern services audited for forbidden imports |
| All mutations gated | Every mutation path traced through `submit_decision()` |
| Custody integrity | `verify_integrity()` passes before and after sprint test suite |
| Owner decision required | No state transition occurs without a DecisionReceipt |
| No auto-action | No intelligence output feeds into any mutation path |
| Recovered state flagged | Every recovered artifact carries `recovery_status != "normal"` |
| Negative tests green | All negative tests from §6 pass |
| Adversarial tests green | All adversarial tests from §8 pass (or documented as out-of-scope with explicit exception) |

---

## Acceptance Gates (This Document)

| Gate | Criteria | Status |
|------|----------|--------|
| PLN-1 | Subsystem contracts defined for all remaining subsystems (Pattern Escalation §1.1, Reconciliation §1.2, Recovery §1.3) | ✅ |
| PLN-2 | Authority boundary diagram documented with forbidden paths (§2) | ✅ |
| PLN-3 | Provenance rules defined for all intelligence objects (§3) | ✅ |
| PLN-4 | Confidence semantics defined and documented (§4) | ✅ |
| PLN-5 | State machines defined for pattern, reconciliation, and recovery (§5) | ✅ |
| PLN-6 | Negative tests defined for all forbidden paths (§6, 15 tests) | ✅ |
| PLN-7 | UI expectations documented for each endpoint (§7) | ✅ |
| PLN-8 | Adversarial test plan defined (§8, 12 tests) | ✅ |
| PLN-9 | Implementation order specified with dependencies (§9) | ✅ |
| PLN-10 | No implementation code written | ✅ |

---

## Document Metadata

- **Generated by:** NODE-PHASE-2-EXECUTION-CONTRACT-1
- **Date:** 2026-07-16
- **Based on review of:** All Phase 2 contracts in `librarian-contracts/src/`, all Phase 2 services in `librarian-node/src/node/`, `NODE-PHASE-2-ARCHITECTURE-PLANNING-1.md`
- **Source tree root:** `G:\openwork\librarian-runtime-node\`
