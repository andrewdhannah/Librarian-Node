# QUALIFICATION-TRANSITION-CONTRACT-001.md — Qualification Transition Contract

**Version:** 1.0.0
**Status:** DRAFT (pending owner review)
**Sealed suite ID:** `QUALIFICATION-TRANSITION-SCHEMA-001`
**Last Updated:** 2026-08-07

**Semantic parent:** `REGISTRY-OBSERVATION-CONTRACT-001` (Canonical — observation boundary is prerequisite)
**Gate:** RUST-MIGRATION-M1 gate M1-4 (qualification semantics)

---

## 0. Purpose

Defines the **transition authority** for qualification state changes. This contract answers: **Who may cause a qualification transition, and what must exist before the transition is permitted?**

This is not a projection contract. It is an authority contract. The distinction matters: projection reads state; transitions change state. The authority to change state must be explicit, auditable, and bounded.

---

## 1. Core Invariants

```
QUALIFIED   ≠ AUTHORIZED    — qualification does not grant permission
QUALIFIED   ≠ AVAILABLE     — qualification does not control availability
QUALIFIED   ≠ EXECUTING     — qualification does not enable execution
```

These three invariants hold independently. No combination of them implies any other. A capability can be qualified without being authorized, available, or executing. Authorization decisions must not mutate qualification records. Availability decisions must not mutate qualification records.

---

## 2. Transition Authority

### 2.1 Permitted Transitions

| From | To | Authority Required | Evidence Required |
|------|----|--------------------|-------------------|
| `unreviewed` | `reviewed` | Manual (owner) or Automated (system with owner pre-authorization) | Review receipt |
| `reviewed` | `qualified` | Manual (owner) or Automated (qualification harness with owner pre-authorization) | Qualification evidence (all mandatory dimensions) |
| `qualified` | `deprecated` | Manual (owner) or Automated (policy expiry with owner notification) | Deprecation reason + evidence reference |
| `qualified` | `revoked` | Manual (owner) or Automated (security policy with owner notification) | Revocation evidence + authority evidence |
| `deprecated` | `revoked` | Manual (owner) or Automated (policy expiry) | Revocation evidence |

### 2.2 Forbidden Transitions

| Transition | Reason |
|------------|--------|
| `unreviewed → qualified` | Bypasses review step |
| `unreviewed → deprecated` | No qualification to deprecate |
| `unreviewed → revoked` | Nothing to revoke |
| `reviewed → unreviewed` | Regression not permitted |
| `deprecated → reviewed` | Deprecation is terminal for review cycle |
| `deprecated → qualified` | Deprecation is terminal for qualification cycle |
| `revoked → *` | Revocation is terminal |

### 2.3 Authority Separation

The following roles must not be the same entity for a single transition:

| Role | Responsibility | Cannot Also Be |
|------|---------------|----------------|
| Qualification executor | Performs the transition | Qualification authority (for same transition) |
| Qualification authority | Authorizes the transition | Qualification evidence provider (for same transition) |
| Qualification evidence provider | Supplies proof | Qualification auditor (for same transition) |
| Qualification auditor | Verifies transition validity | Qualification executor (for same transition) |

### 2.4 Authority Source

The authority to cause a transition must come from:

- **Manual transitions:** Owner identity (authenticated, not self-attested)
- **Automated transitions:** System identity with pre-authorized policy + evidence

The authority source must NOT accidentally become the Rust runtime. The Rust runtime executes transitions; it does not authorize them. Authorization comes from owner identity or pre-authorized policy.

---

## 3. Transition Semantics

### 3.1 Atomicity

Transitions are atomic. A transition either:
- Succeeds completely (state changes, event recorded, evidence linked)
- Fails completely (state unchanged, error recorded, no partial effect)

There are no ambiguous half-transitions. Partial persistence is forbidden.

### 3.2 Append-Only Lifecycle Events

Every transition produces an append-only lifecycle event in `qualification_lifecycle_events`. Events contain:

| Field | Description |
|-------|-------------|
| `event_id` | Unique identifier (QLE-YYYYMMDD-NNN) |
| `qualification_id` | Qualification record affected |
| `capability_id` | Capability affected |
| `from_state` | State before transition |
| `to_state` | State after transition |
| `transition_type` | `automatic` or `manual` |
| `security_classification` | Classification at transition time (frozen) |
| `transitioned_by` | Identity of transitioner |
| `transitioner_role` | Role (`system`, `evaluator`, `approver`, `owner`) |
| `authority_evidence_id` | Evidence of authorization (manual transitions) |
| `evidence_snapshot` | Snapshot of evidence at transition time |
| `created_at` | Timestamp |

### 3.3 Security Classification Provenance

Security classification is **frozen at transition time**. The `security_classification` field in the lifecycle event records the classification when the transition occurred, not the current classification. This preserves provenance and prevents retroactive reclassification from altering audit history.

### 3.4 Failure Semantics

When a transition fails:

| Failure Type | Behavior |
|--------------|----------|
| Evidence insufficient | Transition blocked; error recorded; state unchanged |
| Authority insufficient | Transition blocked; error recorded; state unchanged |
| Policy violation | Transition blocked; error recorded; state unchanged |
| System error | Transition blocked; error recorded; state unchanged |

No partial transitions. No rollback of successful transitions (append-only).

---

## 4. Relationship to Other Axes

### 4.1 Qualification ↔ Availability

Qualification and availability are **independent axes**:

| Qualification | Availability | Consistent? |
|---------------|--------------|-------------|
| `qualified` | `registered` | Yes |
| `qualified` | `disabled` | Yes (qualified but not available) |
| `unreviewed` | `registered` | Yes (not yet qualified) |
| `unreviewed` | `disabled` | Yes |
| `revoked` | `disabled` | Yes (revoked and unavailable) |

Qualification transitions do not affect availability. Availability transitions do not affect qualification. They are orthogonal governance axes.

### 4.2 Qualification ↔ Operational Mode

Qualification and operational mode are **separate contracts**:

- Qualification: "Is this capability verified?"
- Operational mode: "How should this capability behave?"

F-3 (operational-mode derivation) remains a separate contract. Qualification state is an input to operational-mode derivation, but qualification does not determine operational mode.

### 4.3 Qualification ↔ Permission

Qualification does **not** grant permission:

- A qualified capability is not automatically executable
- Permission is a separate authorization decision
- Permission decisions must not mutate qualification records
- Qualification records must not grant permission

This distinction becomes critical in M2 (scheduling and distributed execution).

---

## 5. Explicit Exclusions

| Item | Status | Note |
|------|--------|------|
| Evidence evaluation | Out of scope | Evidence contract (separate) |
| Policy evaluation | Out of scope | Policy context (separate) |
| Permission granting | Out of scope | Authorization contract (separate) |
| Operational-mode derivation | Out of scope | F-3 (separate) |
| Registry mutation | Out of scope | Observation ≠ mutation |
| Schema evolution | Out of scope | Schema frozen |

---

## 6. Authorization

This contract defines the transition authority boundary. It must be locked before any qualification implementation begins.

M1-D0 lock establishes this contract surface. M1-D1 (qualification types) may proceed after this lock.
