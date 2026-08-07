# OPERATIONAL-MODE-DERIVATION-CONTRACT-001.md — Operational Mode Derivation Contract

**Version:** 1.0.0
**Status:** Canonical (owner-approved lock, 2026-08-06)
**Sealed suite ID:** `OPERATIONAL-MODE-DERIVATION-SCHEMA-001`
**Last Updated:** 2026-08-06

---

## 0. Purpose

Defines the deterministic derivation of a capability's **operational mode** from
registry facts. Promoted from finding
`conformance/divergences/M1-FINDING-OPERATIONAL-MODE-DERIVATION-001.md`
(owner-approved, 2026-08-06).

> **Invariant: `SQL derivation == Rust projection`**
>
> The SQL view (`view_operational_mode_derivation`) is the semantic reference
> because it is the **existing frozen derivation artifact** — not because SQL
> is inherently authoritative. Rust reproduces the derivation as a pure
> projection — it does **not** own operational mode. This prevents the common
> migration failure mode where the new implementation gradually becomes the
> authority because it is easier to query.

> **Canonical derivation definition:** the canonical derivation definition is
> the approved derivation artifact (currently the frozen SQL view). Rust
> implementations MUST produce equivalent projections and MUST NOT introduce
> independent decision logic. This leaves room for future migration if the
> derivation artifact itself evolves: the approved artifact is the source of
> truth, whichever form it takes.

---

## 1. Derivation Nature

- **Deterministic pure function** — same inputs always produce the same mode
  (schema line 181: "The derivation is deterministic — same inputs always
  produce same mode").
- **Derived, never stored** (PI-003): operational_mode is computed from stored
  facts at projection time; it is never a stored column.
- **No authority decision** — the derivation reports a mode with an
  explanation; it authorizes nothing.

```
f(security_classification, qualification_state, evidence_state,
  policy_constraints, lifecycle_state)
        |
        v
(operational_mode, mode_explanation, derivation_inputs, evidence_references)
```

---

## 2. Inputs

| Input | Source | Notes |
|-------|--------|-------|
| Security classification | S0–S5 (observed via `CapabilitySecurityContext`, unclassified → S0 default) | `QUALIFICATION-STATE-CONTRACT-001` §5 |
| Qualification state | `capabilities.qualification` axis | passed / qualifying / not_tested / failed / stale / suspended |
| Evidence state | freshness (`fresh`/`stale`/`no_evidence`) + dimensions present | `view_qualification_resolvability` |
| Policy constraints | `capabilities.execution_policy` (JSON) | passed through as `constraints` |
| Lifecycle state | `capabilities.status` + `capabilities.qualification` revocation | revoked → explain_only |

---

## 3. Outputs

| Output | Domain | Notes |
|--------|--------|-------|
| `operational_mode` | `explain_only` / `review_assist` / `recommend_only` / `autonomous_assist` | the derived mode |
| `mode_explanation` | string | why this mode (deterministic, from the derivation) |
| `derivation_inputs` | the facts consumed | reproducibility: same inputs → same mode |
| `evidence_references` | EV-XXXXX refs | evidence that supports the derived mode |

---

## 4. Derivation Function (faithful to `view_operational_mode_derivation`)

The Rust projection MUST reproduce the SQL view's CASE semantics exactly
(`capability-registry-schema-phase3.sql` lines 201–217):

| Priority | Condition | Derived mode |
|----------|-----------|--------------|
| 1 | `qualification = revoked` OR `status = revoked` | `explain_only` |
| 2 | evidence freshness = `stale` | `review_assist` |
| 3 | `qualification = passed` AND classification in (S4, S5) AND evidence not `fresh` | `review_assist` |
| 4 | `qualification = passed` (otherwise) | `autonomous_assist` |
| 5 | `qualification` in (`qualifying`, `not_tested`) | `recommend_only` |
| 6 | `qualification = failed` | `recommend_only` |
| 7 | `qualification = stale` | `review_assist` |
| 8 | `qualification = suspended` | `explain_only` |
| 9 | else (indeterminate) | `recommend_only` |

Matching explanation strings (schema lines 219–237) are part of the
deterministic surface: the same inputs MUST produce the same explanation.

---

## 5. Equivalence Rules

- **SQL derivation == Rust projection**: on identical fixtures, the Rust
  projection output MUST equal the SQL view output (mode + explanation).
- Fixture set (planned at M1-6 evidence seal) covers the mode lattice and edge
  cases: revoked, stale evidence, S4/S5 + non-fresh, fully qualified, in
  progress, failed, suspended, indeterminate.
- Deterministic: same inputs → same mode and explanation, across runs and
  implementations.

---

## 6. Authority Restrictions

- The derivation reports; it does not decide.
- Operational mode is an **observation** — it never authorizes execution,
  never mutates state, never transitions qualification.
- PI-003 honored: mode is never stored; only the derivation inputs are facts.

---

## 7. References

- Finding record (promotion): `conformance/divergences/M1-FINDING-OPERATIONAL-MODE-DERIVATION-001.md`
- Schema view (canonical derivation artifact — currently the frozen SQL view): `librarian-core/assets/schema/capability-registry-schema-phase3.sql`
  (`view_operational_mode_derivation` lines 184–241; `view_qualification_resolvability` lines 147–172; PI-003)
- Security context boundary: `contracts/capability/QUALIFICATION-STATE-CONTRACT-001.md` §5
- M1 work order: `work-orders/RUST-MIGRATION-M1.md`
