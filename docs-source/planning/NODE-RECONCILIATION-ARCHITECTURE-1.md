# NODE-RECONCILIATION-ARCHITECTURE-1

**Status:** Planning Document  
**Scope:** Detailed implementation specifications for the Reconciliation subsystem  
**Source:** Frozen contracts in NODE-PHASE-2-EXECUTION-CONTRACT-1.md §1.2, §1.3, §5.2  
**Constraint:** Planning-only sprint — no implementation code  
**Date:** 2026-07-16

---

## Table of Contents

1. Artifact Comparison Model
2. Conflict Categories
3. Reconciliation Lifecycle
4. Authority Model
5. Receipt Model
6. Offline Rules
7. Implementation Sprint Specifications

---

## 1. Artifact Comparison Model

### 1.1 Overview

Reconciliation compares the local node's persisted state against a **last-known-good (LKG) reference**. The LKG is either:
- The last successful reconciliation receipt (if one exists)
- The node's own custody chain head (if no prior reconciliation)

Comparison is **read-only**. No state is modified during comparison. All classified differences are collected into a report for owner review.

### 1.2 Sessions

#### Local State Source
`SessionService.list_sessions(None)` — returns all `Session` records with fields:
- `session_id`, `node_id`, `state` ("created" | "active" | "closed" | "expired"), `started_at`, `closed_at`, `capability_snapshot`, `context`

#### LKG State Source
The custody chain envelopes of type `"session"`. Each envelope's `receipt_payload` is deserialized as a `SessionReceipt` containing:
- `session_id`, `started_at`, `closed_at`, `operations_executed`, `evidence_ids`, `capability_snapshot_hash`

#### Compared Fields

| Field | Local | LKG | Comparison |
|-------|-------|-----|------------|
| session_id | `Session.session_id` | `SessionReceipt.session_id` | Key match |
| state | `Session.state` | Derived from receipt (closed_at present → closed) | Semantic match |
| started_at | `Session.started_at` | `SessionReceipt.started_at` | Exact match |
| closed_at | `Session.closed_at` | `SessionReceipt.closed_at` | Exact match |

#### Comparison Algorithm

```
for each session_id in (local_sessions ∪ lkg_sessions):
  if session_id in local_sessions and not in lkg_sessions:
    → orphan_session (session exists locally with no custody evidence)
  if session_id in lkg_sessions and not in local_sessions:
    → missing_envelope (session receipt in custody but absent locally)
  if session_id in both:
    compare state fields → state_mismatch if different
```

### 1.3 Custody Envelopes

#### Local State Source
`CustodyService.get_envelopes_by_time_range(None, None)` — returns all `ReceiptEnvelope` records.

#### LKG State Source
The custody chain itself at a known-good point in time. Since reconciliation uses the **current custody chain head** as LKG, custody envelope comparison compares the envelopes **against their own integrity check results**.

#### Compared Fields

| Field | Local | LKG | Comparison |
|-------|-------|-----|------------|
| envelope_id | `ReceiptEnvelope.envelope_id` | From `verify_integrity()` traversal | Key match |
| chain_hash | `ReceiptEnvelope.chain_hash` | Computed chain hash | `divergent_hash` |
| receipt_hash | `ReceiptEnvelope.receipt_hash` | Computed `SHA256(payload)` | `divergent_hash` |
| previous_envelope_hash | `ReceiptEnvelope.previous_envelope_hash` | Previous envelope's chain_hash | Broken chain |
| envelope_count | `CustodyChain.envelope_count` | Count from integrity traversal | Count mismatch |

#### Comparison Algorithm

```
integrity = CustodyService.verify_integrity()
if not integrity.verified:
  for each integrity_error in integrity.errors:
    classify based on error_type:
      "tampered_payload"     → divergent_hash
      "broken_chain"         → divergent_hash
      "hash_mismatch"        → divergent_hash
      "missing_previous"     → missing_envelope

# Also check that the chain metadata matches the actual envelope count
chain = CustodyService.get_chain()
if chain.envelope_count != integrity.envelopes_checked:
  → state_mismatch (envelope_count does not match actual)
```

### 1.4 Registration

#### Local State Source
`RegistrationService.get_record()` — returns `NodeRecord`:
- `registration_status` ("unregistered" | "registration_requested" | "registered" | "suspended" | "retired")

#### LKG State Source
Registration state is determined by the last registration receipt in the custody chain (envelopes of type `"registration"` and `"owner_decision"` with item_type `"registration"`).

#### Compared Fields

| Field | Local | LKG | Comparison |
|-------|-------|-----|------------|
| registration_status | `NodeRecord.registration_status` | Derived from last registration custody envelope | `state_mismatch` |
| node_id | `NodeRecord.node_id` | From identity custody envelope | Identity mismatch |
| capabilities_snapshot | `NodeRecord.capabilities_snapshot` | From capability evidence custody envelopes | `state_mismatch` |

#### Comparison Algorithm

```
lkg_status = derive_status_from_custody_envelopes(custody, "registration")
local_status = registration_service.get_record().registration_status

if lkg_status != local_status:
  → state_mismatch (artifact: "registration", field: "registration_status",
                    expected: lkg_status, actual: local_status)

lkg_cap_hash = derive_capability_hash_from_custody(custody)
local_cap = registration_service.get_record().capabilities_snapshot
local_cap_hash = SHA256(local_cap) if local_cap else None

if lkg_cap_hash != local_cap_hash:
  → state_mismatch (artifact: "capabilities_snapshot")
```

### 1.5 Capabilities

#### Local State Source
`CapabilityEvidenceBridge.get_verification_state(node_id)` — returns `CapabilityVerificationState` containing:
- `VerifiedCapability.capability_type`, `claim_id`, `verification_status`, `last_verified_at`

#### LKG State Source
Custody envelopes of type `"capability_evidence"`. Each envelope's payload contains a `CapabilityVerificationState`.

#### Compared Fields

| Field | Local | LKG | Comparison |
|-------|-------|-----|------------|
| capability_type | `VerifiedCapability.capability_type` | From envelope | Key match |
| verification_status | `VerifiedCapability.verification_status` | From envelope payload | `state_mismatch` |
| claim_id | `VerifiedCapability.claim_id` | From envelope payload | Key match |

#### Comparison Algorithm

```
local_state = bridge.get_verification_state(node_id)
lkg_envelopes = custody.get_envelopes_by_type("capability_evidence")
latest_lkg = most_recent(lkg_envelopes)  # by timestamp
lkg_state: CapabilityVerificationState = deserialize(latest_lkg.receipt_payload)

for each cap_type in (local_state.capabilities ∪ lkg_state.capabilities):
  local_cap = find in local_state by cap_type
  lkg_cap = find in lkg_state by cap_type

  if local_cap and not lkg_cap:
    → orphan_session for capability_type
  if lkg_cap and not local_cap:
    → missing_envelope for capability_type
  if both:
    if local_cap.verification_status != lkg_cap.verification_status:
      → state_mismatch (capability verification status differs)
```

### 1.6 Receipts

#### Local State Source
All service receipt stores aggregated across services:
- `SessionService.get_receipts()`
- `BootstrapService.get_receipts()` (if applicable)
- Custody envelopes themselves

#### LKG State Source
Custody chain envelopes of every receipt type.

#### Compared Fields

| Field | Local | LKG | Comparison |
|-------|-------|-----|------------|
| receipt_id | From service receipt stores | From custody envelope `receipt_id` | Key match |
| receipt_type | From service receipt stores | `ReceiptEnvelope.receipt_type` | Type match |

#### Comparison Algorithm

```
lkg_receipt_ids = {e.receipt_id for e in custody.get_envelopes_by_time_range(None, None)}
local_receipt_ids = collect all receipt IDs from session + bootstrap + other services

for each rid in (local_receipt_ids ∪ lkg_receipt_ids):
  if rid in lkg_receipt_ids and rid not in local_receipt_ids:
    → missing_envelope (receipt referenced in LKG but absent from local stores)
  # Note: receipts present locally but not in custody are tracked as part of
  # normal operation — they will be custodied on next close/sync.
  # This is not a conflict; it's expected work-in-progress.
```

---

## 2. Conflict Categories

### 2.1 Taxonomy

Five conflict categories, all requiring owner review (no auto-accept):

| Classification | Code | Description | Severity | Affected Artifacts |
|---------------|------|-------------|----------|-------------------|
| `missing_envelope` | ME | Custody envelope exists on one side (LKG) but not the other (local) | High | Session receipts, registration receipts, capability evidence envelopes |
| `divergent_hash` | DH | Envelope exists in both but the content hash differs (tamper detection) | Critical | Custody envelopes with payload, receipt_hash, or chain_hash mismatch |
| `orphan_session` | OS | Session is present locally but has no corresponding custody evidence in LKG | Medium | Session records |
| `incomplete_receipt` | IR | Receipt is referenced in custody chain but absent from local service stores | High | Receipt records across all services |
| `state_mismatch` | SM | Artifact exists in both LKG and local but a state field differs | Medium | Registration status, capability verification status, chain metadata |

### 2.2 Conflict Structure

All conflicts are represented as `ClassifiedDifference`:

```rust
pub struct ClassifiedDifference {
    pub difference_id: String,
    pub classification: String,
    pub artifact_type: String,
    pub artifact_id: String,
    pub expected_state: serde_json::Value,
    pub actual_state: serde_json::Value,
    pub field_path: Option<String>,
    pub details: String,
    pub detected_at: String,
}
```

### 2.3 Severity Mapping

| Classification | Default Severity | Overrideable | Owner Urgency |
|---------------|-----------------|-------------|---------------|
| `divergent_hash` | `critical` | No | Immediate — indicates possible tampering |
| `missing_envelope` | `high` | No | Requires decision before next sync |
| `incomplete_receipt` | `high` | No | Data integrity concern |
| `orphan_session` | `medium` | Yes | Safe to defer |
| `state_mismatch` | `medium` | Yes | Safe to defer |

### 2.4 Resolution Actions

| Classification | Accept Action | Override Action |
|---------------|--------------|-----------------|
| `missing_envelope` | Re-create envelope from local data and append to custody | Quarantine — mark as known discrepancy, exclude from future LKG |
| `divergent_hash` | Accept the LKG hash as authoritative (re-hash local) | Quarantine — flag envelope for owner investigation |
| `orphan_session` | Create custody envelope for the local session | Quarantine — mark session as suspect (recovery_status: "suspect") |
| `incomplete_receipt` | Re-create receipt record from custody envelope payload | Quarantine — mark receipt as externally referenced only |
| `state_mismatch` | Accept LKG state as authoritative (update local) | Quarantine — preserve local state, flag discrepancy |

---

## 3. Reconciliation Lifecycle

### 3.1 State Machine

```
                         ┌──────────────────────────────────┐
                         │                                  │
                         ▼                                  │
Offline ──→ Reconnect ──→ Compare ──→ Validate ──→ Review ──┤──→ Accept
                                        │                   │
                                        │ (no differences)  │
                                        └──→ Accept ────────┘
                                                   │
                                             Review ──→ Exception ──→ Quarantine ──→ Owner Override
                                                                                        │
                                                                                        └──→ Accept
```

### 3.2 State Transitions

| Transition | Trigger | Precondition | Receipt Produced | Owner Action Required |
|-----------|---------|-------------|------------------|-----------------------|
| → Offline | Node starts OR `verify_integrity()` returns errors | N/A | None (implied state) | No |
| Offline → Reconnect | Owner initiates reconciliation OR node detects reconnection after custody service becomes available | Node is initialized, custody service is readable | `ReconciliationStartedReceipt` (reason: "reconnect" or "owner_initiated") | No |
| Reconnect → Compare | Automatic after reconnection | LKG reference resolved (either prior receipt or custody chain head) | `ReconciliationStartedReceipt` (phase: "comparing") | No |
| Compare → Validate | Automatic after comparison completes | All five artifact categories compared | `ReconciliationReportReceipt` (draft) | No |
| Validate → Review | Differences found during comparison | Comparison produced at least one `ClassifiedDifference` | `ReconciliationReportReceipt` (final, contains differences) | No |
| Validate → Accept | No differences found | Comparison produced zero `ClassifiedDifference` | `ReconciliationCompleteReceipt` (reason: "no_differences") | No |
| Review → Accept | Owner accepts each classified difference individually or "Accept All" | All differences have owner decision (accept) | `ReconciliationAcceptReceipt` (per difference) + `ReconciliationCompleteReceipt` | **Yes** |
| Review → Exception | Owner chooses to quarantine differences | At least one difference exists | `ReconciliationQuarantineReceipt` | **Yes** |
| Exception → Quarantine | System isolates flagged differences | All differences assigned to quarantine | `ReconciliationQuarantineReceipt` (final) | No |
| Quarantine → Owner Override | Owner explicitly overrides a quarantined item | Item is in quarantined set | `ReconciliationOverrideReceipt` (per item) | **Yes** |
| Quarantine → Accept | Owner accepts a quarantined item | Item is in quarantined set | `ReconciliationAcceptReceipt` (per item) | **Yes** |
| Review → Offline | Interruption (node shutdown, custody unavailable) | System failure during review | `ReconciliationQuarantineReceipt` (reason: "interrupted") | No |

### 3.3 Phase Behaviors

#### Offline
- No reconciliation is running
- Node may be operating normally with local state
- All local state is preserved; no sync occurs

#### Reconnect
- Resolve LKG reference: load last reconciliation receipt, or use custody chain head
- Create `ReconciliationStartedReceipt` with phase metadata
- Verify custody integrity (`verify_integrity()`)
  - If integrity check fails, skip to **Exception** before comparison begins
- Store pre-reconciliation snapshot: custody chain hash at start

#### Compare
- Collect all local state from services (sessions, custody envelopes, registration, capabilities, receipts)
- Compare against LKG reference using the artifact comparison model (§1)
- Produce a list of `ClassifiedDifference` (may be empty)
- This phase is a **pure function**: given the same inputs, it produces the same output

#### Validate
- If differences list is empty → transition to **Accept** with `ReconciliationCompleteReceipt`
- If differences list is non-empty → transition to **Review** with `ReconciliationReportReceipt`
- Validate also checks that all differences reference valid artifacts (no dangling IDs)

#### Review
- Differences are presented to the owner through the review surface
- Owner can:
  - Accept all differences (batched)
  - Accept individual differences
  - Quarantine all differences (batched)
  - Quarantine individual differences
- Each owner action produces a receipt

#### Accept
- All differences have been resolved (either accepted or overridden and accepted)
- Final `ReconciliationCompleteReceipt` is produced
- The acceptance phase does **not** mutate state — it records the decision
- State mutation (if any) is performed by the caller (e.g., recovery service) after reconciliation completes

#### Exception
- Integrity check failed before comparison
- Or: interruption occurred during comparison
- Or: custody service unavailable at reconnection time
- A `ReconciliationQuarantineReceipt` is produced with the error details
- The node continues operating with its current state

#### Quarantine
- Differences are isolated: a quarantined difference is written to a quarantine file
- Quarantined items are **excluded** from future reconciliation LKG comparisons
- The node continues operating; quarantined differences do not block normal operation
- Owner can later override or accept quarantined items

#### Owner Override
- The owner chooses to override a quarantined item
- Override means: "I accept the discrepancy without resolving it"
- A `ReconciliationOverrideReceipt` is produced per item
- Overridden items remain quarantined (they are flagged as "override_accepted") but no state mutation occurs

### 3.4 Interruption Handling

| Interruption Point | Behavior |
|--------------------|----------|
| During Reconnect | No report produced; state unchanged; next reconciliation starts fresh from Offline |
| During Compare | No report produced; partial comparison result is discarded; next reconciliation re-compares |
| During Validate | No final report produced; partial validation result is discarded |
| During Review (owner hasn't decided) | In-progress decisions are lost; differences remain pending; next reconciliation starts from Compare |
| During Review (some decisions made) | Already-produced receipts remain in custody; remaining differences are re-evaluated on next reconciliation |
| During Quarantine | Quarantine file is written atomically; if write fails, no quarantine state is committed |

---

## 4. Authority Model

### 4.1 Owner vs. Auto

| Classification | Auto-Accept? | Requires Owner Review? | Rationale |
|---------------|-------------|----------------------|-----------|
| `missing_envelope` | No | **Yes** | Re-creating envelopes is a state mutation |
| `divergent_hash` | No | **Yes** | Indicates possible tampering; owner must inspect |
| `orphan_session` | No | **Yes** | Creating custody evidence for local state is a mutation |
| `incomplete_receipt` | No | **Yes** | Re-creating receipts is a data integrity decision |
| `state_mismatch` | No | **Yes** | Choosing which state is authoritative is a governance decision |

**No difference type is auto-accepted.** This is a hard invariant enforced at compile time and runtime.

### 4.2 Owner Override Semantics

"Owner override" in the reconciliation context means:
- The owner explicitly acknowledges a classified difference
- The owner directs the system to **not resolve** the difference — i.e., to accept that the two sides disagree
- The difference is recorded as overridden and preserved in the reconciliation report
- Future reconciliation runs **exclude** overridden items from comparison (they are tracked in an exclusion list)
- The override does **not** modify state on either side (local or LKG)

Owner override is **not** a bypass of the authority boundary. It is an explicit recorded decision that satisfies the owner-gate requirement.

### 4.3 Authority Lookup

Authority for reconciliation decisions is determined by:

1. **Node ownership**: The node has exactly one owner, resolved at registration time
2. **Owner identity**: Stored in `OwnerDecision.owner_identity` field
3. **Verification**: The `OwnerWorkflowService.submit_decision()` method validates that the decision originates from the registered owner
4. **Forwarding**: Reconciliation decisions flow through `OwnerWorkflowService`:

```
ReconciliationService
  → submit_decision(difference_id, decision)
    → OwnerWorkflowService.submit_decision()
      → validates owner identity
      → produces DecisionReceipt (custodied)
      → returns custody envelope ID
```

### 4.4 Authority Boundary

```
Comparison (read-only)
    ↓
ClassifiedDifference (data)
    ↓
OwnerDecision (data + identity)
========== AUTHORITY BOUNDARY ==========
ReconciliationAcceptReceipt (custodied)
    ↓
State mutation (if accepted: local state updated to match LKG)
         OR
No-op (if overridden: difference quarantined but no mutation)
```

The authority boundary is the same `submit_decision()` gate used by all Phase 2 subsystems.

---

## 5. Receipt Model

### 5.1 Contract Types

All reconciliation receipt types live in `librarian-contracts/src/reconciliation/`:

#### ReconciliationRequest

```rust
/// Initiated when reconciliation begins.
pub struct ReconciliationRequest {
    pub reconciliation_id: String,
    pub node_id: String,
    pub lkg_reference: String,
    pub initiated_at: String,
    pub initiated_by: String,
    pub phase: String,
}
```

Fields:
- `reconciliation_id`: UUID v4, generated at initiation
- `node_id`: The node being reconciled
- `lkg_reference`: The custody chain head hash (or prior reconciliation receipt ID)
- `initiated_at`: RFC 3339 timestamp
- `initiated_by`: `"owner"` | `"system"` | `"reconnect"`
- `phase`: `"reconnecting"` | `"comparing"` | `"validating"` | `"reviewing"` | `"accepting"`

#### ReconciliationReport

```rust
/// Produced after comparison completes.
pub struct ReconciliationReport {
    pub report_id: String,
    pub reconciliation_id: String,
    pub node_id: String,
    pub lkg_reference: String,
    pub custody_snapshot: String,
    pub differences: Vec<ClassifiedDifference>,
    pub total_differences: u32,
    pub generated_at: String,
    pub phase: String,
}
```

Fields:
- `report_id`: UUID v4
- `reconciliation_id`: Links to the initiating request
- `lkg_reference`: Same as request (frozen at comparison start)
- `custody_snapshot`: Hash of custody chain at comparison start
- `differences`: All classified differences (may be empty)
- `total_differences`: Count of differences
- `generated_at`: RFC 3339 timestamp
- `phase`: `"draft"` | `"final"`

#### ReconciliationDecision

```rust
/// Owner decision on a single classified difference.
pub struct ReconciliationDecision {
    pub decision_id: String,
    pub reconciliation_id: String,
    pub difference_id: String,
    pub node_id: String,
    pub decision: String,
    pub reason: Option<String>,
    pub decided_at: String,
    pub actor: String,
}
```

Fields:
- `decision_id`: UUID v4
- `reconciliation_id`: Links to the reconciliation cycle
- `difference_id`: Links to a specific `ClassifiedDifference`
- `decision`: `"accept"` | `"override"` | `"quarantine"`
- `reason`: Optional owner-provided reason
- `decided_at`: RFC 3339 timestamp
- `actor`: Owner identity string or `"system"`

#### ReconciliationReceipt

```rust
/// Receipt produced for each reconciliation lifecycle transition.
pub struct ReconciliationReceipt {
    pub receipt_id: String,
    pub reconciliation_id: String,
    pub node_id: String,
    pub receipt_type: String,
    pub previous_phase: Option<String>,
    pub new_phase: Option<String>,
    pub decision_id: Option<String>,
    pub difference_ids: Vec<String>,
    pub payload: serde_json::Value,
    pub generated_at: String,
}
```

Fields:
- `receipt_id`: UUID v4
- `reconciliation_id`: Links to the reconciliation cycle
- `receipt_type`: One of the receipt types below
- `previous_phase`, `new_phase`: Phase transition tracking
- `decision_id`: Present if this receipt records a decision
- `difference_ids`: IDs of affected differences
- `payload`: The actual data (report, decision, etc.)
- `generated_at`: RFC 3339 timestamp

### 5.2 Receipt Types (receipt_type values)

| receipt_type | Produced At | payload Contains |
|-------------|-------------|------------------|
| `reconciliation_started` | Reconnect phase begins | `ReconciliationRequest` |
| `reconciliation_report` | Validate phase completes | `ReconciliationReport` |
| `reconciliation_accept` | Owner accepts a difference | `ReconciliationDecision` |
| `reconciliation_override` | Owner overrides a difference | `ReconciliationDecision` |
| `reconciliation_quarantine` | Exception or Quarantine transition | Error details + affected differences |
| `reconciliation_complete` | Accept phase completes | Summary (total differences, accept/override/quarantine counts) |

### 5.3 Custody Integration

Each reconciliation receipt is appended to the custody chain via `CustodyService.append_receipt()` with:
- `receipt_type`: one of the values above
- `receipt_id`: the `ReconciliationReceipt.receipt_id`
- `receipt_payload`: serialized `ReconciliationReceipt`

The act of reconciling is custodied, not the result of the merge. Reconciliation does not modify existing custody envelopes.

### 5.4 Receipt Flow Per Cycle

```
1. ReconciliationStartedReceipt  ───→ custody chain
2. ReconciliationReportReceipt        (if differences found)
3. ReconciliationAcceptReceipt        (per accepted difference)
   OR ReconciliationOverrideReceipt   (per overridden difference)
4. ReconciliationCompleteReceipt  ───→ custody chain
```

If no differences found:
```
1. ReconciliationStartedReceipt
2. ReconciliationCompleteReceipt (reason: "no_differences")
```

If exception/quarantine:
```
1. ReconciliationStartedReceipt
2. ReconciliationQuarantineReceipt
```

---

## 6. Offline Rules

### 6.1 State Preservation During Offline Operation

When the node is offline (not reconciling), the following state is preserved:

| Artifact | Preserved? | Mechanism |
|----------|-----------|-----------|
| Session state | ✅ Yes | `SessionService` persists to `sessions.json` |
| Session receipts | ✅ Yes | Custody chain persists to `custody.json` |
| Registration state | ✅ Yes | `RegistrationService` persists to `node-registration.json` |
| Capability claims | ✅ Yes | `CapabilityEvidenceBridge` persists to `capability_evidence.json` |
| Custody chain | ✅ Yes | `CustodyService` persists to `custody.json` |
| Previous reconciliation state | ✅ Yes | Last `ReconciliationCompleteReceipt` in custody chain |
| Quarantined differences | ✅ Yes | Quarantine file at `reconciliation_quarantine.json` |

**Invariant**: Offline operation never modifies state that would invalidate the LKG reference. The LKG reference (custody chain head hash) remains a valid point of comparison.

### 6.2 What Changes During Offline Operation

During offline operation, local state diverges from LKG:
- New sessions are created (not yet custodied as receipts)
- Active sessions are closed (receipts are generated but not yet in LKG if LKG is a prior snapshot)
- Capability claims may be made (verified but not custodied)
- Registration state may change (but will be captured in a registration receipt when online)

These divergences are expected and are precisely what reconciliation detects and classifies.

### 6.3 Reconnection Detection

Reconnection is detected when:

1. **Custody service becomes available**: If custody was unavailable (file system error), reconnection occurs when `verify_integrity()` succeeds
2. **Owner manually initiates**: Owner calls `POST /reconcile`
3. **Node state transition**: If `NodeStateMachine` transitions from any state to a non-`Reconciling` state after startup

Reconnection detection is implemented as a check in the reconciliation service:

```rust
pub fn detect_reconnection(&self) -> bool {
    // Returns true if custody is available and either:
    // 1. No reconciliation is in progress, or
    // 2. Last reconciliation completed and new sessions/receipts exist since then
}
```

### 6.4 Interruption During Reconciliation

If reconciliation is interrupted at any phase:

| Interruption | State After Interruption | Recovery on Next Start |
|-------------|-------------------------|----------------------|
| Before any receipt produced | No reconciliation state exists | Starts fresh from **Offline** |
| After `ReconciliationStartedReceipt` | Receipt in custody, phase known | Detects incomplete reconciliation from custody; restarts from **Reconnect** |
| After `ReconciliationReportReceipt` | Report in custody, differences classified | Restarts from **Validate** using the persisted report |
| During owner decision submission | Some decisions in custody, some missing | On restart, reads all decisions from custody for the reconciliation_id; compares to report; un-decided differences remain pending |
| During quarantine file write | Quarantine file may be incomplete | If quarantine file is absent or corrupt, quarantined differences are re-created from custody receipts |

**Invariant**: Receipts are always written to custody before any other state change. This ensures that even if the process crashes, the reconciliation trail is preserved.

### 6.5 Startup Recovery After Interrupted Reconciliation

On startup, the reconciliation service:

1. Reads the custody chain for the most recent `reconciliation_started` receipt
2. If found and no corresponding `reconciliation_complete` receipt exists:
   - Reads all intermediate receipts (report, decisions, quarantine)
   - Determines the last known phase
   - Resumes from that phase, or restarts from **Compare** if the phase cannot be determined
3. If no in-progress reconciliation found: starts from **Offline**

---

## 7. Implementation Sprint Specifications

### 7.1 Sprint: NODE-RECONCILIATION-FOUNDATION-1

| Field | Value |
|-------|-------|
| **Subsystem** | Reconciliation Foundation |
| **State machine implemented** | Reconciliation lifecycle (Offline → Reconnect → Compare → Validate → Review → Accept / Exception → Quarantine → Owner Override) |
| **State transitions implemented** | All transitions from §3.2 except Quarantine → Owner Override (deferred to Recovery sprint) |
| **Dependencies** | NODE-RECONCILIATION-ARCHITECTURE-1 (this document) |

#### Contract Types to Create

File: `librarian-contracts/src/reconciliation/mod.rs`

```rust
pub mod report;
pub mod difference;
pub mod receipt;
pub mod config;

pub use report::{ReconciliationReport, ReconciliationRequest};
pub use difference::{ClassifiedDifference, ConflictSeverity};
pub use receipt::{ReconciliationDecision, ReconciliationReceipt};
pub use config::ReconciliationConfig;
```

##### `report.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationRequest {
    pub reconciliation_id: String,
    pub node_id: String,
    pub lkg_reference: String,
    pub initiated_at: String,
    pub initiated_by: String,
    pub phase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationReport {
    pub report_id: String,
    pub reconciliation_id: String,
    pub node_id: String,
    pub lkg_reference: String,
    pub custody_snapshot: String,
    pub differences: Vec<ClassifiedDifference>,
    pub total_differences: u32,
    pub generated_at: String,
    pub phase: String,
}
```

##### `difference.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictSeverity {
    Critical,
    High,
    Medium,
}

impl ConflictSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConflictSeverity::Critical => "critical",
            ConflictSeverity::High => "high",
            ConflictSeverity::Medium => "medium",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassifiedDifference {
    pub difference_id: String,
    pub classification: String,
    pub artifact_type: String,
    pub artifact_id: String,
    pub severity: ConflictSeverity,
    pub expected_state: serde_json::Value,
    pub actual_state: serde_json::Value,
    pub field_path: Option<String>,
    pub details: String,
    pub detected_at: String,
}
```

##### `receipt.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationDecision {
    pub decision_id: String,
    pub reconciliation_id: String,
    pub difference_id: String,
    pub node_id: String,
    pub decision: String,
    pub reason: Option<String>,
    pub decided_at: String,
    pub actor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationReceipt {
    pub receipt_id: String,
    pub reconciliation_id: String,
    pub node_id: String,
    pub receipt_type: String,
    pub previous_phase: Option<String>,
    pub new_phase: Option<String>,
    pub decision_id: Option<String>,
    pub difference_ids: Vec<String>,
    pub payload: serde_json::Value,
    pub generated_at: String,
}
```

##### `config.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationConfig {
    pub auto_reconcile_on_reconnect: bool,
    pub quarantine_on_integrity_failure: bool,
    pub max_differences_per_report: u32,
    pub version: String,
}

impl Default for ReconciliationConfig {
    fn default() -> Self {
        ReconciliationConfig {
            auto_reconcile_on_reconnect: true,
            quarantine_on_integrity_failure: true,
            max_differences_per_report: 1000,
            version: "1".to_string(),
        }
    }
}
```

#### Service Methods to Create

File: `librarian-node/src/node/reconciliation_service.rs`

| Method | Signature | Phase | Description |
|--------|-----------|-------|-------------|
| `new(persistence_path)` | `fn new(persistence_path: PathBuf) -> Self` | Construction | Initialize service, load persisted state |
| `with_custody(...)` | `fn with_custody(self, custody: Arc<Mutex<CustodyService>>) -> Self` | Construction | Wire custody dependency |
| `with_session_service(...)` | `fn with_session_service(self, sessions: Arc<Mutex<SessionService>>) -> Self` | Construction | Wire session service dependency |
| `with_registration_service(...)` | `fn with_registration_service(self, reg: Arc<Mutex<RegistrationService>>) -> Self` | Construction | Wire registration service dependency |
| `with_capability_bridge(...)` | `fn with_capability_bridge(self, bridge: Arc<Mutex<CapabilityEvidenceBridge>>) -> Self` | Construction | Wire capability evidence bridge dependency |
| `initiate_reconciliation()` | `fn initiate_reconciliation(&mut self) -> Result<ReconciliationRequest, ReconciliationError>` | Reconnect | Begin new reconciliation cycle, resolve LKG |
| `run_comparison()` | `fn run_comparison(&mut self) -> Result<Vec<ClassifiedDifference>, ReconciliationError>` | Compare | Compare all five artifact categories, return classified differences |
| `generate_report(differences)` | `fn generate_report(&mut self, differences: Vec<ClassifiedDifference>) -> Result<ReconciliationReport, ReconciliationError>` | Validate | Produce final report, transition to Review or Accept |
| `submit_decision(difference_id, decision, reason)` | `fn submit_decision(&mut self, difference_id: &str, decision: &str, reason: Option<&str>) -> Result<ReconciliationDecision, ReconciliationError>` | Review | Record owner decision on a single difference |
| `submit_batch_decision(difference_ids, decision, reason)` | `fn submit_batch_decision(&mut self, difference_ids: &[String], decision: &str, reason: Option<&str>) -> Result<Vec<ReconciliationDecision>, ReconciliationError>` | Review | Record batch owner decision |
| `quarantine(difference_ids, reason)` | `fn quarantine(&mut self, difference_ids: &[String], reason: &str) -> Result<(), ReconciliationError>` | Exception | Quarantine differences, write quarantine file |
| `finalize_reconciliation()` | `fn finalize_reconciliation(&mut self) -> Result<ReconciliationReceipt, ReconciliationError>` | Accept | Produce ReconciliationCompleteReceipt, persist state |
| `get_pending_differences()` | `fn get_pending_differences(&self) -> Vec<ClassifiedDifference>` | Review | Return differences awaiting decision |
| `get_reconciliation_status()` | `fn get_reconciliation_status(&self) -> ReconciliationStatus` | Query | Return current phase and status |
| `get_reconciliation_history()` | `fn get_reconciliation_history(&self) -> Vec<ReconciliationReceipt>` | Query | Return past reconciliation receipts |
| `detect_reconnection()` | `fn detect_reconnection(&self) -> bool` | Offline→Reconnect | Check if reconciliation should begin |
| `get_config()` | `fn get_config(&self) -> ReconciliationConfig` | Query | Return current config |
| `set_config(config)` | `fn set_config(&self, config: ReconciliationConfig) -> Result<(), ReconciliationError>` | Config | Update config (metadata change, produces receipt) |

#### Error Type

```rust
#[derive(Debug)]
pub enum ReconciliationError {
    CustodyUnavailable(String),
    IntegrityCheckFailed(String),
    LkgReferenceNotFound(String),
    ComparisonFailed(String),
    InvalidPhaseTransition { from: String, to: String },
    DuplicateDecision(String),
    UnknownDifference(String),
    PersistenceFailed(String),
    OwnerWorkflowDenied(String),
}
```

#### Endpoints

| Method | Endpoint | Service Method | Auth |
|--------|----------|---------------|------|
| `POST` | `/reconcile` | `initiate_reconciliation()` + `run_comparison()` + `generate_report()` | Owner |
| `GET` | `/reconcile/status` | `get_reconciliation_status()` | Owner |
| `GET` | `/reconcile/report` | Returns current `ReconciliationReport` from custody (most recent) | Owner |
| `GET` | `/reconcile/differences` | `get_pending_differences()` | Owner |
| `POST` | `/reconcile/decisions` | `submit_batch_decision()` | Owner |
| `POST` | `/reconcile/quarantine` | `quarantine()` | Owner |
| `GET` | `/reconcile/history` | `get_reconciliation_history()` | Owner |
| `GET` | `/reconcile/config` | `get_config()` | Owner |
 `PUT` | `/reconcile/config` | `set_config()` | Owner |

#### Tests Required

| # | Test Name | What It Tests | Type |
|---|-----------|---------------|------|
| R-01 | `test_initiate_reconciliation_creates_receipt` | Reconnect phase produces `ReconciliationStartedReceipt` in custody | Unit |
| R-02 | `test_comparison_no_differences_fresh_node` | Freshly initialized node produces empty differences list | Unit |
| R-03 | `test_comparison_detects_missing_envelope` | Envelope in LKG but deleted locally is classified as `missing_envelope` | Unit |
| R-04 | `test_comparison_detects_divergent_hash` | Tampered envelope is classified as `divergent_hash` | Unit |
| R-05 | `test_comparison_detects_orphan_session` | Session without custody evidence is classified as `orphan_session` | Unit |
| R-06 | `test_comparison_detects_state_mismatch` | Registration status differs from LKG | Unit |
| R-07 | `test_generate_report_with_differences` | Report is generated with differences; phase transitions to Review | Unit |
| R-08 | `test_generate_report_no_differences` | Report is generated with 0 differences; phase transitions to Accept | Unit |
| R-09 | `test_submit_decision_accept` | Owner accept produces `ReconciliationAcceptReceipt` | Unit |
| R-10 | `test_submit_decision_override` | Owner override produces `ReconciliationOverrideReceipt` | Unit |
| R-11 | `test_batch_decision_all_accepted` | Batch accept of all differences completes reconciliation | Unit |
| R-12 | `test_quarantine_preserves_state` | Quarantine writes differences to file; local state unchanged | Unit |
| R-13 | `test_finalize_reconciliation_produces_complete_receipt` | Finalize produces `ReconciliationCompleteReceipt` in custody | Unit |
| R-14 | `test_integrity_failure_enters_quarantine` | `verify_integrity()` fails → exception → quarantine | Unit |
| R-15 | `test_get_reconciliation_history` | Past receipts are queryable | Unit |
| R-16 | `test_persistence_survives_restart` | Service state survives restart from persistence file | Unit |
| R-17 | `test_no_auto_accept` | All classifications require owner decision; no `auto_accept` field exists | Negative |
| R-18 | `test_no_mutation_during_compare` | `run_comparison()` does not call `append_receipt` or modify services | Negative |
| R-19 | `test_no_mutation_during_validate` | `generate_report()` does not mutate service state | Negative |
| R-20 | `test_single_node_only` | Service accepts a single node_id; no fleet iterator | Negative |

#### Negative Tests (from NODE-PHASE-2-EXECUTION-CONTRACT-1 §6)

| # | Test Name | Status |
|---|-----------|--------|
| N-05 | `reconciliation_cannot_silently_merge_conflicts` | Implemented (R-17) |
| N-06 | `reconciliation_cannot_bypass_custody` | Implemented (R-18) |
| N-11 | `reconciliation_no_mutation_during_compare` | Implemented (R-19) |
| N-12 | `owner_decision_required_for_all_state_transitions` | Implemented (R-17 covers reconciliation) |
| N-15 | `reconciliation_no_cross_node` | Implemented (R-20) |

#### Adversarial Tests (from NODE-PHASE-2-EXECUTION-CONTRACT-1 §8)

| # | Test Name | Status |
|---|-----------|--------|
| A-03 | `disconnect_during_reconciliation` | Covered by R-14 (integrity failure) |
| A-05 | `simultaneous_multi_node_edits` | Covered by R-04 (divergent_hash) |
| A-11 | `reconciliation_with_zero_state` | Covered by R-02 (fresh node) |

#### Explicit Exclusions

| Feature | Reason | Sprint |
|---------|--------|--------|
| Owner Override from Quarantine | Requires quarantine file reading and override workflow | Deferred to NODE-RECOVERY-CUSTODY-1 |
| Automatic re-reconciliation | Auto-detect reconnection and start reconciliation | Deferred (config flag exists but auto-trigger is not implemented) |
| Exclusion list for overridden items | Override tracking across reconciliation cycles | Deferred to NODE-RECOVERY-CUSTODY-1 |
| Quarantine UI endpoints | UI surface for viewing/managing quarantined items | Deferred |
| Fleet-level reconciliation | Out of scope per Phase 2 constraints | Never |
| State mutation after accept | Reconciliation service records decisions but does not mutate session/custody/registration state | Deferred to NODE-RECOVERY-CUSTODY-1 (state mutation is the recovery service's responsibility) |

### 7.2 Sprint: NODE-RECOVERY-CUSTODY-1

| Field | Value |
|-------|-------|
| **Subsystem** | Recovery Custody |
| **State machine implemented** | Recovery lifecycle (Healthy → Suspect → Reconciling → Owner Review → Recovered / Failed) |
| **State transitions implemented** | All transitions from §5.3 of execution contract |
| **Dependencies** | NODE-RECONCILIATION-FOUNDATION-1 |

#### Contract Types to Create

File: `librarian-contracts/src/custody_recovery/mod.rs`

```rust
pub mod state;
pub mod receipt;
pub mod evidence;

pub use state::RecoveryStatus;
pub use receipt::{RecoveryReceipt, RecoveryRequest};
pub use evidence::RecoveryEvidence;
```

##### `state.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecoveryStatus {
    Normal,
    Recovered,
    Suspect,
}

impl RecoveryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecoveryStatus::Normal => "normal",
            RecoveryStatus::Recovered => "recovered",
            RecoveryStatus::Suspect => "suspect",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "normal" => Some(RecoveryStatus::Normal),
            "recovered" => Some(RecoveryStatus::Recovered),
            "suspect" => Some(RecoveryStatus::Suspect),
            _ => None,
        }
    }
}
```

##### `receipt.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecoveryPhase {
    Healthy,
    Suspect,
    Reconciling,
    OwnerReview,
    Recovered,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecoveryRequest {
    pub recovery_id: String,
    pub node_id: String,
    pub initiated_at: String,
    pub initiated_by: String,
    pub trigger: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecoveryReceipt {
    pub receipt_id: String,
    pub recovery_id: String,
    pub node_id: String,
    pub receipt_type: String,
    pub previous_phase: RecoveryPhase,
    pub new_phase: RecoveryPhase,
    pub affected_artifacts: Vec<String>,
    pub payload: serde_json::Value,
    pub generated_at: String,
}
```

Recovery receipt types:
- `suspect_flag` — Integrity check failed or anomaly detected
- `recovery_started` — Recovery initiated (owner or auto)
- `recovery_report` — Reconciliation report produced
- `recovery_accepted` — Owner accepted recovered state
- `recovery_failed` — Owner rejected or system error
- `suspect_cleared` — Owner dismissed suspect flag

##### `evidence.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecoveryEvidence {
    pub evidence_id: String,
    pub recovery_id: String,
    pub evidence_type: String,
    pub envelope_ids: Vec<String>,
    pub session_ids: Vec<String>,
    pub description: String,
    pub recorded_at: String,
}
```

#### Service Methods to Create

File: `librarian-node/src/node/custody_recovery_service.rs`

| Method | Signature | Phase | Description |
|--------|-----------|-------|-------------|
| `new(persistence_path)` | `fn new(persistence_path: PathBuf) -> Self` | Construction | Initialize service |
| `with_custody(...)` | `fn with_custody(self, custody: Arc<Mutex<CustodyService>>) -> Self` | Construction | Wire custody dependency |
| `with_reconciliation(...)` | `fn with_reconciliation(self, reconciliation: Arc<Mutex<ReconciliationService>>) -> Self` | Construction | Wire reconciliation service dependency |
| `get_status()` | `fn get_status(&self) -> RecoveryPhase` | Query | Return current recovery phase |
| `flag_suspect(trigger, details)` | `fn flag_suspect(&mut self, trigger: &str, details: &str) -> Result<RecoveryReceipt, RecoveryError>` | Healthy→Suspect | Produce `SuspectFlagReceipt` |
| `clear_suspect(reason)` | `fn clear_suspect(&mut self, reason: &str) -> Result<RecoveryReceipt, RecoveryError>` | Suspect→Healthy | Produce `SuspectClearedReceipt` |
| `initiate_recovery()` | `fn initiate_recovery(&mut self) -> Result<RecoveryReceipt, RecoveryError>` | Suspect→Reconciling | Begin recovery cycle |
| `run_recovery_comparison()` | `fn run_recovery_comparison(&mut self) -> Result<RecoveryEvidence, RecoveryError>` | Reconciling | Run reconciliation, produce report |
| `present_report_to_owner()` | `fn present_report_to_owner(&mut self) -> Result<(), RecoveryError>` | Reconciling→OwnerReview | Transition to owner review |
| `accept_recovered_state()` | `fn accept_recovered_state(&mut self) -> Result<RecoveryReceipt, RecoveryError>` | OwnerReview→Recovered | Accept, flag affected artifacts as `recovered` |
| `reject_recovered_state(reason)` | `fn reject_recovered_state(&mut self, reason: &str) -> Result<RecoveryReceipt, RecoveryError>` | OwnerReview→Failed | Reject recovery |
| `flag_artifact_as_recovered(artifact_type, artifact_id)` | `fn flag_artifact_as_recovered(&mut self, artifact_type: &str, artifact_id: &str)` | Mutation | Set `recovery_status = "recovered"` on artifact metadata |
| `get_evidence_chain()` | `fn get_evidence_chain(&self) -> Vec<RecoveryReceipt>` | Query | Return all recovery receipts for current cycle |
| `get_affected_artifacts()` | `fn get_affected_artifacts(&self) -> Vec<(String, String, String)>` | Query | Return `(artifact_type, artifact_id, recovery_status)` tuples |

#### Endpoints

| Method | Endpoint | Service Method | Auth |
|--------|----------|---------------|------|
| `GET` | `/recovery/status` | `get_status()` | Owner |
| `POST` | `/recovery/initiate` | `initiate_recovery()` | Owner |
| `POST` | `/recovery/clear` | `clear_suspect()` | Owner |
| `GET` | `/recovery/evidence` | `get_evidence_chain()` | Owner |
| `GET` | `/recovery/affected` | `get_affected_artifacts()` | Owner |
| `POST` | `/recovery/accept` | `accept_recovered_state()` | Owner |
| `POST` | `/recovery/reject` | `reject_recovered_state()` | Owner |
| `GET` | `/recovery/report` | Returns current recovery report | Owner |

#### Tests Required

| # | Test Name | What It Tests | Type |
|---|-----------|---------------|------|
| C-01 | `test_suspect_flag_produces_receipt` | `flag_suspect()` produces `SuspectFlagReceipt` in custody | Unit |
| C-02 | `test_clear_suspect_produces_receipt` | `clear_suspect()` produces `SuspectClearedReceipt` in custody | Unit |
| C-03 | `test_recovery_lifecycle_full` | Full cycle: Healthy→Suspect→Reconciling→OwnerReview→Recovered | Integration |
| C-04 | `test_recovery_lifecycle_rejected` | Full cycle: Healthy→Suspect→Reconciling→OwnerReview→Failed | Integration |
| C-05 | `test_recovery_auto_initiate_after_delay` | Suspect→Reconciling after configurable delay (default 300s) | Unit |
| C-06 | `test_affected_artifacts_flagged_recovered` | After accept, artifacts carry `recovery_status = "recovered"` | Unit |
| C-07 | `test_evidence_chain_queryable` | All recovery receipts visible via `get_evidence_chain()` | Unit |
| C-08 | `test_recovery_preserves_original_envelopes` | Original corrupted envelopes are not deleted | Negative |
| C-09 | `test_recovery_no_workload_dependency` | RecoveryService constructor does not accept `WorkloadSessionService` | Negative |
| C-10 | `test_recovery_no_auto_accept` | Every transition to Recovered requires owner decision receipt | Negative |

#### Negative Tests (from NODE-PHASE-2-EXECUTION-CONTRACT-1 §6)

| # | Test Name | Status |
|---|-----------|--------|
| N-07 | `recovery_cannot_erase_history` | Implemented (C-08) |
| N-10 | `recovery_service_no_workload_dependency` | Implemented (C-09) |
| N-12 | `owner_decision_required_for_all_state_transitions` | Implemented (C-10) |
| N-14 | `recovered_state_is_flagged` | Implemented (C-06) |

#### Adversarial Tests (from NODE-PHASE-2-EXECUTION-CONTRACT-1 §8)

| # | Test Name | Status |
|---|-----------|--------|
| A-04 | `tampered_custody_records` | Covered by C-01 (suspect flag on integrity failure) |
| A-09 | `recovery_overwrite_good_state` | Recovery on healthy chain exits at Reconciling with "no discrepancies" |
| A-12 | `recovery_suspect_clear_without_investigation` | Covered by C-02 (clear suspect flag) |

#### Explicit Exclusions

| Feature | Reason | Sprint |
|---------|--------|--------|
| State mutation during recovery | Recovery accepts decisions but does not mutate session/custody state | Deferred to post-Phase 2 |
| Recovery modifying allocation or bootstrap state | Recovery scope is custody chain + session integrity only | Never |
| Cross-node recovery | Recovery is per-node | Never |
| Recovery creating sessions | No `SessionService` dependency in recovery service | Never |
| Recovery resuming workloads | No workload creation/activation/completion in recovery service | Never |
| Fleet-level recovery coordination | Out of scope | Never |

---

## Document Metadata

- **Generated by:** NODE-RECONCILIATION-ARCHITECTURE-1
- **Date:** 2026-07-16
- **Based on:** NODE-PHASE-2-EXECUTION-CONTRACT-1.md §1.2, §1.3, §5.2, §6, §7, §8, §9
- **Existing contracts reviewed:** `librarian-contracts/src/custody/`, `session/`, `capability_evidence/`, `node/`, `owner_workflows/`, `pattern_escalation/`
- **Existing services reviewed:** `librarian-node/src/node/custody_service.rs`, `session_service.rs`, `registration_service.rs`, `identity_service.rs`, `capability_evidence.rs`, `owner_workflow_service.rs`, `state.rs`
- **Source tree root:** `G:\openwork\librarian-runtime-node\`

---

## Acceptance Gates

| Gate | Criteria | Status |
|------|----------|--------|
| ARC-1 | Artifact comparison model defined (§1) | ✅ |
| ARC-2 | Conflict categories defined with classification (§2) | ✅ |
| ARC-3 | Reconciliation lifecycle state machine defined (§3) | ✅ |
| ARC-4 | Authority model defined (owner vs auto) (§4) | ✅ |
| ARC-5 | Receipt model defined for all transitions (§5) | ✅ |
| ARC-6 | Offline rules documented (§6) | ✅ |
| ARC-7 | Implementation sprints specified with contracts, methods, endpoints, tests, exclusions (§7) | ✅ |
| ARC-8 | No implementation code written | ✅ |
