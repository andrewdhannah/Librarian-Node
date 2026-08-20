# CAPABILITY-ASSURANCE-CONTRACT-001.md — Capability Assurance Contract

**Version:** 1.0.0
**Status:** Reconstructed Canonical (owner-approved reconstruction, 2026-08-06)
**Sealed suite ID:** `CAPABILITY-ASSURANCE-SCHEMA-001`
**Last Updated:** 2026-08-06

---

## 0. Contract Origin (provenance declaration)

```
Contract Origin:      Reconstructed
Original Artifact:    Not recoverable from accessible repositories/history
                      (recovery search 2026-08-06 — see
                      conformance/divergences/M1-FINDING-CAPABILITY-ASSURANCE-CONTRACT-001.md)
Reconstruction Sources:
  - librarian-core/assets/schema/capability-registry-schema.sql
  - librarian-core/assets/schema/capability-registry-schema-phase3.sql
  - CR-I-* invariants (capability registry)
  - PI-* invariants (phase 3 qualification governance)
  - governance completion receipts (G:\Librarian AR-ENTITY-001, AR-PERMISSIONS-001,
    AR-DECISIONS-001, AR-MQR-SPRINT-1, AR-STORAGE-001)
Authority Basis:      Derived semantic contract approved through migration governance
```

> This document is **not** the recovered original artifact. It is a new canonical
> contract reconstruction derived from the available authoritative evidence above.
> The schema remains the provenance source; this contract is the migration boundary.

### Non-Authority Clause

> This contract defines assurance semantics; it does not grant capability
> availability, qualification, permission, or execution authority.

The three axes (`availability` / `qualification` / `authority`) are exactly where
future implementations could accidentally collapse distinctions. Declaring and
observing an axis is not granting it. Nothing in this contract authorizes a
capability to exist, to qualify, or to execute.

---

## 1. Assurance Axes (§1 — the section the schema references)

The frozen schema (`capability-registry-schema.sql`, line 77: *"See
CAPABILITY-ASSURANCE-CONTRACT-001 §1 for semantics"*) annotates three independent
assurance axes on `capabilities`. This section is the semantics authority for those axes.

### 1.1 The three axes are INDEPENDENT

`availability`, `qualification`, and `authority` are independent axes
(PI-002 pattern for qualification/classification applies analogously across all
three): every valid combination is expressible, and each carries its own
evidence burden. Examples:

| availability | qualification | authority | Meaning |
|--------------|---------------|-----------|---------|
| `registered` | `not_tested` | `not_submitted` | Known to the registry; nothing proven yet |
| `registered` | `passed` | `pending_review` | Qualification passed; authority not yet granted |
| `registered` | `passed` | `approved` | Qualified AND authorized (subject to CR-I-007 chain) |
| `disabled` | `passed` | `approved` | Qualified+authorized but administratively disabled |

### 1.2 `availability` axis

What does the registry know — is the capability present and reachable?

| Value | Meaning |
|-------|---------|
| `discovered` | Detected/imported but not formally registered |
| `registered` | Formally registered with the registry (normal operational state) |
| `disabled` | Registered but administratively disabled |
| `removed` | Removed from availability (history retained) |

### 1.3 `qualification` axis

Has the capability proven it works — against a qualification profile?

| Value | Meaning |
|-------|---------|
| `not_tested` | No qualification evaluation has occurred |
| `qualifying` | Qualification evaluation in progress |
| `passed` | Qualification passed against a profile (requires evidence chain, §2) |
| `failed` | Qualification failed |
| `stale` | Qualification out of date (e.g., profile version increment, CR-I-008) |
| `suspended` | Qualification suspended pending re-evaluation |

### 1.4 `authority` axis

Has an authority granted permission for this capability to be used?

| Value | Meaning |
|-------|---------|
| `not_submitted` | No authority submission made |
| `pending_review` | Submitted; awaiting authority decision |
| `approved` | Authority approval recorded (decision record, DECISIONS-001 precedent) |
| `rejected` | Authority rejected |
| `revoked` | Authority approval revoked |

**Boundary:** the `authority` axis records a governed decision — it does not
create the permission, it records it. PERMISSIONS-001 precedent: *permissions
reference recorded decisions — they do not create authority.*

### 1.5 Lifecycle `status` axis (governance status)

The `capabilities.status` lifecycle is a fourth, governance axis with its own
transition gate:

```
unreviewed → reviewed → qualified → deprecated / revoked
```

Status gates execution: a capability's registry presence alone grants nothing.

---

## 2. Qualified Availability Precondition (CR-I-007)

A capability MUST NOT transition to qualified availability (i.e., to
`qualification = passed` while operationally reachable) unless **all** of the
following are present and resolvable:

1. A valid **qualification profile** (`qualification_profiles`, active version)
2. A **qualification record** (`capability_qualifications`, `passed`)
3. An **evidence reference** (`capability_versions.qualification_evidence_id`
   and/or `capability_qualifications.evidence_reference` → Evidence Plane, CR-I-010)
4. **Governing policy context** (`policy_bindings` → active `policies`, CR-I-009)

The registry view `view_capabilities_qualified_without_assurance` surfaces
capabilities that violate this invariant (expected empty at steady state).

---

## 3. Evidence Is a Reference, Never Inline

Qualification evidence is a REFERENCE into the Evidence Plane — never embedded
in the capability row or version body:

```
Capability
   └── QualificationRecord
         └── EvidenceReference (EV-XXXXX)
               └── Evidence Plane (owns proof)
```

**Evidence ≠ Approval** — a referenced evidence record proves a fact; it does
not grant permission.

---

## 4. Authority Restrictions

- The assurance axes are **observational state**: they may be read and reported.
- State transitions on the axes are **governed operations** (qualified
  availability per CR-I-007; S-level elevation per PI-005 requires a new
  qualification event, never an attribute update).
- **Self-attestation is prohibited** (PI-004): a capability cannot assert its
  own security classification or operational mode.

Preserved non-collapse invariants:

```
Registry             ≠ Authority
Capability           ≠ Permission
Qualification        ≠ Authorization
Evidence             ≠ Approval
```

---

## 5. Equivalence Rules

- Rust projections of the assurance axes MUST match the registry state
  field-for-field (same values, same semantics).
- No transformation of axis semantics: the wire representation of each axis
  value is the value itself.
- Deterministic: the same registry state produces the same projection.

---

## 6. References

- Finding record: `conformance/divergences/M1-FINDING-CAPABILITY-ASSURANCE-CONTRACT-001.md`
- Schema (provenance source): `librarian-core/assets/schema/capability-registry-schema.sql` (Phase 2)
- Phase 3 invariants: `librarian-core/assets/schema/capability-registry-schema-phase3.sql` (PI-001..005)
- M1 work order: `work-orders/RUST-MIGRATION-M1.md`
- Governance precedents: PERMISSIONS-001 (permissions record decisions, do not create authority), DECISIONS-001 (durable authority records), MQR-SPRINT-1 (qualification as consumer of the governance substrate)
