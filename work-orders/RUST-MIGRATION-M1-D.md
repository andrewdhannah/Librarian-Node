# RUST-MIGRATION-M1-D — Qualification Semantics: Planning

**Status:** DRAFT (pending owner review)
**Epic:** EPIC-RUST-MIGRATION-1
**Phase:** 1 — M1-D (qualification semantics)
**Predecessor:** M1-C3 seal `e0b7706` (observation adapter boundary established)
**Gate:** RUST-MIGRATION-M1 gate M1-4 (qualification semantics)
**Last Updated:** 2026-08-07

---

## 0. Scope of This Artifact

Planning work order for M1-D. Defines the qualification semantics boundary before any implementation.

**This is materially different from everything completed so far.** Up to this point, Rust has been reconstructing and exposing state. Qualification introduces state transition semantics — the ability to cause `unreviewed → reviewed → qualified → deprecated/revoked`. That is a fundamentally different authority question.

**M1-D non-goal:** Immediate implementation of a `qualify()` method. First establish the contract for what qualification means, who is authorized to perform it, what evidence must exist, and how failures are handled.

---

## 1. Current State

### 1.1 What M1 Established (Observation Boundary)

M1 demonstrated that Rust can expose governed registry state through MCP and HTTP without creating authority:

```
       Projection Module
              |
       ┌──────┴──────┐
       │              │
    MCP Adapter   HTTP Adapter
       │              │
       └──────┬──────┘
              |
    Equivalent Observations
```

**Invariants preserved:**
- Registry ≠ Authority
- Projection ≠ Ownership
- Observation ≠ Mutation
- Transport ≠ Semantic
- MCP Identity ≠ Registry Identity
- HTTP Identity ≠ Registry Identity

### 1.2 What M1 Did NOT Establish

M1 established the observation side of the boundary. The following are explicitly out of scope for M1 and require separate contract boundaries:

| Item | Status | Note |
|------|--------|------|
| Capability qualification execution | Not demonstrated | M1-D scope |
| Authorization/permission granting | Not demonstrated | Separate authority contract |
| Operational-mode decisions | Not demonstrated | F-3, separate contract |
| Registry mutation | Not demonstrated | Observation ≠ mutation |
| F-2 security classification storage | Not resolved | `M1-FINDING-SECURITY-CLASSIFICATION-SCHEMA-001` |
| F-3 operational-mode implementation | Not resolved | Separate contract |

### 1.3 The Frozen Schema as Starting Point

The frozen capability-registry schema provides evidence of intended structure:

| Table | Role | Qualification Relevance |
|-------|------|------------------------|
| `capabilities` | Identity + lifecycle state | `status` column: `unreviewed|reviewed|qualified|deprecated|revoked` |
| `capability_versions` | Append-only version history | Version pointer (`active_version`) |
| `capability_qualifications` | Qualification records | FK to `capability_versions`, status, confidence, evidence |
| `qualification_lifecycle_events` | Transition audit trail | Append-only event log |
| `qualification_evidence_references` | Evidence provenance | Dimension, type, hash, producer |

**Critical observation (F-1):** The schema is evidence of intended structure, not automatically the migration contract. The schema shows what columns exist, not what authority is required to transition them.

---

## 2. Primary Planning Questions

### 2.1 What Exactly Is Permitted to Cause Transitions?

The core question: **What is permitted to cause `unreviewed → reviewed → qualified → deprecated/revoked`?**

This must be answered before any implementation. The answer establishes:

- Whether transitions are manual (owner-initiated), automated (system-initiated), or hybrid
- What authority is required for each transition
- What evidence must exist before each transition
- Who/what is actually authorized to perform a transition

### 2.2 Separation of Concerns

Qualification requires separating at least four things:

```
Capability State          ≠  The current lifecycle state
    ≠
Qualification Evidence    ≠  Proof that qualification occurred
    ≠
Policy Context            ≠  Rules governing when qualification is valid
    ≠
Authorization to Execute  ≠  Who/what is permitted to cause transitions
```

These must not be conflated. A `qualify()` method that combines all four would create an authority leak.

### 2.3 Required Evidence References

For each transition, what evidence must exist?

| Transition | Required Evidence | Question |
|------------|-------------------|----------|
| `unreviewed → reviewed` | Review approval | Who reviews? Manual or automated? |
| `reviewed → qualified` | Qualification evidence | What dimensions? What confidence threshold? |
| `qualified → deprecated` | Deprecation reason | Who decides? What notice period? |
| `qualified → revoked` | Revocation evidence | What severity? What immediate effect? |
| `deprecated → revoked` | Revocation evidence | Same as above? |

### 2.4 Qualification Profile Validity

The schema references `profile_id` in `capability_qualifications`. What does a profile define?

- Which evidence dimensions are required?
- What confidence thresholds apply?
- What is the validity period?
- Can profiles change after qualification?

### 2.5 Policy Context

What policy rules govern qualification?

- Are there time-based constraints (e.g., qualification expires)?
- Are there dependency constraints (e.g., dependency must be qualified)?
- Are there security classification constraints (e.g., S3+ requires manual review)?
- How do policy changes affect existing qualifications?

### 2.6 Security Classification Provenance

F-2 identified a divergence between the schema's `security_classification` column and the qualification evidence model. How is classification:

- Stored (column, policy, derived)?
- Updated (who authorized)?
- Used in qualification decisions?
- Provenanced (audit trail)?

### 2.7 Append-Only Lifecycle Events

The schema includes `qualification_lifecycle_events` as an append-only audit trail. What must each event contain?

- Event type (transition)
- From/to state
- Transition type (automatic/manual)
- Security classification at transition time
- Transitioner identity
- Authority evidence ID
- Evidence snapshot

### 2.8 Failure/Rollback Semantics

What happens when qualification fails?

- Is partial qualification possible (some dimensions pass, some fail)?
- Can a qualification be rolled back?
- What happens to dependent capabilities?
- What is the blast radius of a failed qualification?

### 2.9 Relationship Between Qualification and Availability

The schema has separate columns: `qualification` (axis) and `availability` (axis). How do they interact?

- Can a capability be `qualified` but `disabled`?
- Can a capability be `registered` but `failed` qualification?
- Who controls each axis?
- Are there consistency constraints?

### 2.10 Relationship Between Qualification and Permission

Qualification and permission are separate concerns. How are they related?

- Does qualification grant permission?
- Can permission exist without qualification?
- Can qualification exist without permission?
- Who controls permission?

---

## 3. Proposed Milestone Discipline

```
M1-C3  Observation boundary              SEALED
   │
   ▼
M1-D0  Qualification semantics planning   ← current
   │
   ├── semantic recovery (what does the schema intend?)
   ├── authority analysis (who can cause transitions?)
   ├── transition contract (what evidence is required?)
   ├── evidence contract (what dimensions, thresholds, profiles?)
   └── failure/rollback analysis (what happens on failure?)
   │
   ▼
M1-D0 contract lock
   │
   ▼
M1-D1  Qualification types (Rust types matching contract)
   │
   ▼
M1-D2  Qualification module (projection/transition logic)
   │
   ▼
M1-D3  Evidence + seal
```

---

## 4. Authority Analysis Framework

### 4.1 Transition Authority Matrix

| Transition | Manual Authority | Automated Authority | Evidence Required |
|------------|------------------|---------------------|-------------------|
| `unreviewed → reviewed` | Owner review | System review (if automated) | Review receipt |
| `reviewed → qualified` | Owner approval | Qualification harness | Qualification evidence |
| `qualified → deprecated` | Owner decision | Deprecation policy | Deprecation reason |
| `qualified → revoked` | Owner decision | Security policy | Revocation evidence |
| `deprecated → revoked` | Owner decision | Policy expiry | Revocation evidence |

### 4.2 Authority Separation

The following must not be the same entity:

- **Qualification executor** — performs the transition
- **Qualification authority** — authorizes the transition
- **Qualification evidence provider** — supplies proof
- **Qualification auditor** — verifies the transition was valid

### 4.3 Authorization Boundaries

Who/what is authorized to perform each transition?

- Manual transitions: owner identity required
- Automated transitions: system identity required, with evidence
- Emergency transitions: elevated authority required, with audit trail

---

## 5. Evidence Contract Framework

### 5.1 Evidence Dimensions

The schema references five qualification evidence dimensions:

| Dimension | Description | Required For |
|-----------|-------------|--------------|
| `identity` | Capability identity verification | All qualifications |
| `capability` | Capability function verification | All qualifications |
| `security_level` | Security classification verification | S2+ capabilities |
| `qualification` | Qualification process verification | All qualifications |
| `constraints` | Constraint compliance verification | Policy-constrained capabilities |

### 5.2 Evidence Types

| Type | Description | Trust Level |
|------|-------------|-------------|
| `test_result` | Automated test output | High (if harness is trusted) |
| `review_approval` | Human review approval | High (if reviewer is authorized) |
| `benchmark` | Performance measurement | Medium |
| `audit_log` | System audit trail | Medium |
| `receipt` | External receipt | Variable |

### 5.3 Evidence Freshness

How long is evidence valid?

- Is there an expiration period?
- Does evidence become stale?
- How is freshness verified?
- What happens when evidence expires?

---

## 6. Open Questions (pending owner review)

1. **Transition authority:** Should transitions be manual-only, automated-only, or hybrid?
2. **Evidence thresholds:** What confidence threshold qualifies a capability?
3. **Profile validity:** How long is a qualification profile valid?
4. **Policy context:** How do policy changes affect existing qualifications?
5. **Security classification:** How is F-2 resolved?
6. **Failure semantics:** Is partial qualification possible?
7. **Rollback:** Can qualifications be rolled back?
8. **Blast radius:** What happens to dependent capabilities when qualification fails?
9. **Availability interaction:** How do qualification and availability axes interact?
10. **Permission relationship:** Does qualification grant permission?

---

## 7. Authorization

This work order is a planning artifact. M1-D0 contract lock begins only after owner review and approval of the qualification semantics contract.

No implementation code until M1-D0 is locked.
