# M1-FINDING-OPERATIONAL-MODE-DERIVATION-001 — Contract Candidate (Positive Finding)

**Status:** Pending (promoted to M1 contract candidate `OPERATIONAL-MODE-DERIVATION-CONTRACT-001`; extraction at M1-0)
**Date:** 2026-08-06
**Protocol:** RUST-MIGRATION-M1 planning; `docs/architecture/RUST-CORE-CONFORMANCE-SPECIFICATION.md` §9 (Divergence Protocol)
**Recorded by:** RUST-MIGRATION-M1 planning checkpoint (work order `RUST-MIGRATION-M1.md`, DRAFT)

---

## Summary

**FINDING:** Positive — operational mode is a deterministic pure derivation, a strong M1 contract candidate.

**Classification:** Contract candidate (promoted by owner, 2026-08-06)

**Promotion:** Target contract `OPERATIONAL-MODE-DERIVATION-CONTRACT-001`.

**Invariant:** `SQL derivation == Rust projection` — Rust reproduces the derivation, it does **not own** operational mode. The SQL view remains the semantic authority; the Rust projection is equivalent evidence.

---

## Detailed Record

| Field | Value |
|-------|-------|
| **Source** | `view_operational_mode_derivation` (`capability-registry-schema-phase3.sql`, lines 184–241) |
| **Nature** | Deterministic pure function: `f(security_classification, qualification_state, evidence, constraints) → (operational_mode, mode_explanation)` — same inputs always produce the same mode (schema line 181: "The derivation is deterministic — same inputs always produce same mode") |
| **Output domain** | `explain_only` / `review_assist` / `recommend_only` / `autonomous_assist`, each with a `mode_explanation` string |
| **PI-003** | operational_mode is DERIVED, never stored directly — the view implements this invariant |
| **Why it fits M1** | Gives Rust a deterministic semantic projection testable against the SQL view (C2 evidence pattern: fixture → SQL view output vs Rust projection output → equivalence record) |

---

## Contract Target (suggested shape for M1-0)

**OPERATIONAL-MODE-DERIVATION-CONTRACT-001**

| Aspect | Definition |
|--------|------------|
| **Inputs** | Security classification (S0–S5), qualification state, evidence state (freshness, dimensions), policy constraints, lifecycle state |
| **Outputs** | Operational mode, explanation, derivation inputs (the facts that produced the mode), evidence references |
| **Authority** | None. The derivation makes no authorization decision — it reports a derived mode with explanation |
| **Equivalence** | SQL derivation == Rust projection (fixture-validated, not Rust-owned semantics) |

---

## Consequences

1. **M1-0 deliverable:** extract `OPERATIONAL-MODE-DERIVATION-CONTRACT-001` during contract surface lock, with fixture set covering the mode lattice (explain_only / review_assist / recommend_only / autonomous_assist) and edge cases (stale evidence, S4/S5 + non-fresh, revoked, indeterminate).
2. **Equivalence evidence:** deterministic projection validation — Rust projection output equals SQL view output on identical fixtures (F-3 in work order `RUST-MIGRATION-M1.md`).
3. **Boundary preserved:** the view (SQL) is the semantic authority; Rust matches the projection. This is the same pattern that caught the Swift startup receipt boundary in M0A — semantic drift surfaced before code reproduced it.

---

## Next Action

Include `OPERATIONAL-MODE-DERIVATION-CONTRACT-001` in the M1-0 contract surface lock (contract extraction), with the fixture set and SQL-vs-Rust equivalence evidence planned at M1-6.
