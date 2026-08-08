# QUALIFICATION-EVIDENCE-CONTRACT-001.md — Qualification Evidence Contract

**Version:** 1.0.0
**Status:** DRAFT (pending owner review)
**Sealed suite ID:** `QUALIFICATION-EVIDENCE-SCHEMA-001`
**Last Updated:** 2026-08-07

**Semantic parent:** `QUALIFICATION-TRANSITION-CONTRACT-001` (DRAFT — transition authority is prerequisite)
**Gate:** RUST-MIGRATION-M1 gate M1-4 (qualification semantics)

---

## 0. Purpose

Defines the **evidence requirements** for qualification transitions. This contract answers: **What evidence must exist before a transition is permitted, and how is that evidence validated?**

Evidence is the proof that qualification occurred. Without evidence, a transition is unauthorized. The evidence contract ensures that qualification is verifiable, not merely asserted.

---

## 1. Core Invariant

```
No evidence  → No transition
Invalid evidence → No transition
Expired evidence → No transition (unless policy allows staleness)
```

Evidence is mandatory, contract-defined (not implementation-defined), and verifiable.

---

## 2. Evidence Dimensions

### 2.1 Required Dimensions

Five evidence dimensions are defined by the schema:

| Dimension | Description | Required For |
|-----------|-------------|--------------|
| `identity` | Capability identity verification | All qualifications |
| `capability` | Capability function verification | All qualifications |
| `security_level` | Security classification verification | S2+ capabilities |
| `qualification` | Qualification process verification | All qualifications |
| `constraints` | Constraint compliance verification | Policy-constrained capabilities |

### 2.2 Dimension Requirements

| Dimension | Minimum Evidence | Verification Method |
|-----------|------------------|---------------------|
| `identity` | Identity proof (hash, signature) | Hash comparison or signature verification |
| `capability` | Capability proof (test result, review) | Evidence type validation |
| `security_level` | Classification evidence | Classification source verification |
| `qualification` | Qualification process evidence | Process audit trail |
| `constraints` | Constraint compliance evidence | Constraint evaluation |

### 2.3 Dimension Completeness

A qualification is **evidence-complete** when:
- All required dimensions have evidence references
- All evidence references are valid (not expired, not revoked)
- All evidence satisfies the minimum requirements for its dimension

A qualification is **evidence-incomplete** when:
- Any required dimension lacks evidence
- Any evidence reference is invalid
- Any evidence fails minimum requirements

Evidence-incomplete qualifications cannot transition to `qualified`.

---

## 3. Evidence Types

### 3.1 Type Definitions

| Type | Description | Trust Level | Verification |
|------|-------------|-------------|--------------|
| `test_result` | Automated test output | High (if harness is trusted) | Harness identity + result hash |
| `review_approval` | Human review approval | High (if reviewer is authorized) | Reviewer identity + signature |
| `benchmark` | Performance measurement | Medium | Measurement hash + methodology |
| `audit_log` | System audit trail | Medium | Log integrity + chain |
| `receipt` | External receipt | Variable | Receipt source verification |

### 3.2 Type Selection

The evidence type is **contract-defined** for each dimension:

| Dimension | Allowed Types | Rationale |
|-----------|---------------|-----------|
| `identity` | `test_result`, `review_approval` | Identity must be verified, not measured |
| `capability` | `test_result`, `review_approval`, `benchmark` | Capability can be tested or reviewed |
| `security_level` | `review_approval`, `audit_log` | Classification requires authority |
| `qualification` | `test_result`, `review_approval`, `audit_log` | Process evidence varies |
| `constraints` | `test_result`, `audit_log` | Constraints are evaluatable |

### 3.3 Type Provenance

Every evidence record must include:

| Field | Description |
|-------|-------------|
| `evidence_id` | Unique identifier (QER-YYYYMMDD-NNN) |
| `qualification_id` | Qualification record affected |
| `dimension` | Evidence dimension |
| `evidence_type` | Evidence type |
| `evidence_reference` | External reference (EV-XXXXX or URL) |
| `evidence_body` | Inline evidence (optional, for small evidence) |
| `evidence_hash` | SHA-256 of evidence body |
| `captured_at` | When evidence was captured |
| `expires_at` | When evidence expires (NULL = no expiry) |
| `producer_identity` | Who produced the evidence |
| `producer_role` | Role of producer (`evaluator`, `system`, `automated_harness`, `external`) |

---

## 4. Evidence Freshness

### 4.1 Expiration

Evidence can expire. The `expires_at` field defines when evidence is no longer valid:

- `expires_at = NULL`: Evidence does not expire
- `expires_at = timestamp`: Evidence expires at timestamp

### 4.2 Staleness

Evidence is **fresh** when:
- `expires_at` is NULL, or
- `expires_at > current_time`

Evidence is **stale** when:
- `expires_at <= current_time`

### 4.3 Staleness Policy

| Policy | Behavior |
|--------|----------|
| Reject stale | Stale evidence cannot support transitions |
| Allow stale with warning | Stale evidence supports transitions with audit trail |
| Require refresh | Stale evidence must be refreshed before transition |

The staleness policy is **contract-defined**, not implementation-defined.

### 4.4 Evidence Refresh

When evidence expires:
- The evidence record remains (append-only)
- A new evidence record must be captured
- The new record references the old record
- The qualification's evidence references are updated

---

## 5. Evidence Validation

### 5.1 Validation Rules

| Rule | Description |
|------|-------------|
| Existence | Evidence record must exist |
| Reference integrity | `evidence_reference` must be resolvable |
| Hash integrity | `evidence_hash` must match `evidence_body` |
| Freshness | Evidence must not be stale (unless policy allows) |
| Dimension completeness | All required dimensions must have evidence |
| Type validity | Evidence type must be allowed for the dimension |
| Producer validity | Producer identity must be verifiable |

### 5.2 Validation Failure

When evidence validation fails:
- The evidence record is marked invalid
- The qualification cannot transition to `qualified`
- An audit event is recorded
- No partial transitions

---

## 6. Relationship to Qualification State

Evidence is **owned by the Evidence Plane**. The qualification record carries references, not inline evidence:

```
Qualification Record
    │
    ├── evidence_reference: "EV-00001" (reference, not proof)
    │
    └── Evidence Record (separate)
         ├── evidence_id: "EV-00001"
         ├── evidence_body: {...} (proof)
         ├── evidence_hash: "abc123..." (integrity)
         └── producer_identity: "evaluator-1" (provenance)
```

This separation ensures:
- Evidence can be verified independently
- Evidence can be refreshed without mutating qualification records
- Evidence provenance is preserved
- Qualification records remain lightweight

---

## 7. Explicit Exclusions

| Item | Status | Note |
|------|--------|------|
| Evidence evaluation logic | Out of scope | Evaluation is implementation-defined |
| Evidence storage | Out of scope | Storage is implementation-defined |
| Evidence retrieval | Out of scope | Retrieval is implementation-defined |
| Evidence revocation | Out of scope | Revocation is a future concern |

---

## 8. Authorization

This contract defines the evidence requirements boundary. It must be locked before any qualification implementation begins.

M1-D0 lock establishes this contract surface. M1-D1 (qualification types) may proceed after this lock.
