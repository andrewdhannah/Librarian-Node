# QUALIFICATION-STATE-CONTRACT-001.md — Qualification State Contract

**Version:** 1.0.0
**Status:** Canonical (owner-approved lock, 2026-08-06)
**Sealed suite ID:** `QUALIFICATION-STATE-SCHEMA-001`
**Last Updated:** 2026-08-06

---

## 0. Purpose

Defines the qualification state model: what it means for a capability to move
from untested to qualified, what evidence is required, and how transitions are
recorded. This contract is the migration boundary for the qualification state —
the contract is the boundary, not the code.

> **Qualification ≠ Authorization** — a passed qualification proves the
> capability works; it does not grant permission to execute (authority is the
> separate axis, `CAPABILITY-ASSURANCE-CONTRACT-001` §1.4).

---

## 1. Qualification Lifecycle

### 1.1 Governance status lifecycle (`capabilities.status`)

```
unreviewed → reviewed → qualified → deprecated / revoked
```

- `qualified` requires the CR-I-007 assurance chain (§3).
- `deprecated` / `revoked` are terminal governance states.
- Transitions are **governed operations, not free column writes** — the state
  machine rejects invalid transitions.

### 1.2 Qualification axis lifecycle (`capabilities.qualification`)

```
not_tested → qualifying → passed | failed | stale | suspended
```

- `passed` requires an evidence-backed qualification record (§3).
- `stale` arises on profile version increment (CR-I-008) or evidence expiry.

---

## 2. Qualification Record Chain

The qualification state is expressed through four linked artifacts:

```
Qualification Profile (what is required)
        |
        v
Qualification Record (the result)
        |
        v
Lifecycle Event (how the transition happened, append-only)
        |
        v
Evidence Reference (proof, owned by the Evidence Plane)
```

### 2.1 Qualification profile (`qualification_profiles`)

- Reusable requirement definitions: `checks` (JSON array
  `[{id, description, required}]`), `success_criteria` (JSON), `version`.
- Profile lifecycle: `active` → `superseded` / `deprecated`; `supersedes`
  pointer records lineage.
- **CR-I-008:** a profile version increment queues all capabilities bound to
  that profile for requalification (qualification becomes `stale`).

### 2.2 Qualification record (`capability_qualifications`)

- Result of a qualification evaluation against a profile.
- `qualification_status`: `qualifying` | `passed` | `failed` | `stale` | `superseded`
- `confidence`: 0.0–1.0 score.
- Temporal boundaries: `qualified_at`, `expires_at` (NULL = does not expire),
  `assessed_at`.
- Provenance: `assessor_identity`, `assessor_type` (`manual` | `automated` | `external`).
- `evidence_reference` → Evidence Plane (EV-XXXXX) — **reference, not inline**.

### 2.3 Lifecycle events (`qualification_lifecycle_events`) — append-only

- Immutable audit trail for every qualification state transition.
- `from_state` → `to_state`, `transition_type` (`automatic` | `manual`).
- **Frozen security classification at transition time** (`S0`–`S5` or NULL
  pre-classification) — audit replay reads this, not current registry state.
- Authority: `transitioned_by`, `transitioner_role`
  (`system` | `evaluator` | `approver` | `owner`),
  `authority_evidence_id` (EV-XXXXX for manual transitions),
  `evidence_snapshot` (JSON: evidence state at transition time).

### 2.4 Evidence records (`qualification_evidence_records`)

- 0..N evidence records per qualification across **five dimensions**:
  `identity` | `capability` | `security_level` | `qualification` | `constraints`
- `evidence_type`: `test_result` | `review_approval` | `benchmark` | `audit_log` | `receipt`
- `evidence_reference` (EV-XXXXX or external), `evidence_body` (JSON),
  `evidence_hash` (SHA-256 of body, non-empty), `expires_at`
- Provenance: `producer_identity`, `producer_role`
  (`evaluator` | `system` | `automated_harness` | `external`)

---

## 3. CR-I-007 — Qualified Availability Precondition

A capability MUST NOT be in qualified availability unless **all** are present
and resolvable:

1. Valid qualification profile (active version)
2. Passed qualification record
3. Evidence reference (resolvable in the Evidence Plane, CR-I-010)
4. Governing policy context (active policy binding, CR-I-009)

Violations are surfaced by `view_capabilities_qualified_without_assurance`
(expected empty at steady state).

---

## 4. Invalid Transitions Are Rejected

- The state machine defines legal transitions; anything else is rejected
  (application-layer enforcement, mirroring CR-I-001's application-layer rule
  for append-only versions).
- **PI-005:** S-level elevation (e.g., S2 → S4) requires a NEW qualification
  event — never an attribute update.
- **PI-004:** self-attestation of security classification or operational mode
  is prohibited.

---

## 5. CapabilitySecurityContext (observational boundary — F-2)

Per `conformance/divergences/M1-FINDING-SECURITY-CLASSIFICATION-SCHEMA-001.md`,
the schema diverges: the Phase-3 derivation view reads `capabilities.security_classification`
while the Phase-2 DDL does not define the column. **This contract does not
resolve storage placement.** It defines the observational boundary only:

```rust
CapabilitySecurityContext
{
    classification,       // S0..S5 (or unclassified) — observed value
    source,               // where the classification fact originates
    derivation,           // how it was obtained (assessment / transition / inherited)
    evidence_reference    // EV-XXXXX into the Evidence Plane (PI-004: no self-attestation)
}
```

- `CapabilitySecurityContext` is an **observational projection** — it reports
  classification facts from their authoritative origin (frozen transition
  S-level, assessment, etc.) without storing or asserting a new column.
- **Classification provenance must remain observable even if storage
  representation changes.** The projection MUST preserve the distinction
  between the four provenance kinds, whatever layout stores them:

  | Kind | Meaning |
  |------|---------|
  | `declared` | classification asserted at registration |
  | `derived` | classification computed from registry facts |
  | `inherited` | classification inherited from version/type context |
  | `policy_constraint` | classification derived from policy constraints |

  An implementation MAY choose any storage representation — it MUST NOT choose
  one that loses these distinctions.
- Storage resolution (persisted / derived / version-inherited / projected —
  question A/B/C/D in the finding) is deferred to M1 implementation planning
  as a governed schema decision. No silent DDL change.

---

## 6. Authority Restrictions

- MAY observe and report qualification state.
- MUST NOT transition qualification state from the observation surface.
- MUST NOT grant authorization based on qualification (Qualification ≠ Authorization).
- MUST NOT accept self-attested classification (PI-004).

---

## 7. Equivalence Rules

- Rust qualification projections MUST match the registry tables field-for-field
  (profiles, records, lifecycle events, evidence records).
- Lifecycle event replay is deterministic: the append-only event log is the
  authoritative transition history.
- Evidence references resolve in the Evidence Plane; the projection reports the
  reference, never fabricates evidence.

---

## 8. References

- Finding record: `conformance/divergences/M1-FINDING-SECURITY-CLASSIFICATION-SCHEMA-001.md`
- Schema (source evidence): `librarian-core/assets/schema/capability-registry-schema.sql`
  (tables: `qualification_profiles`, `capability_qualifications`; CR-I-007/008/010)
- Phase 3 schema: `librarian-core/assets/schema/capability-registry-schema-phase3.sql`
  (tables: `qualification_lifecycle_events`, `qualification_evidence_records`; PI-002/004/005)
- Assurance axes: `contracts/capability/CAPABILITY-ASSURANCE-CONTRACT-001.md`
- M1 work order: `work-orders/RUST-MIGRATION-M1.md`
