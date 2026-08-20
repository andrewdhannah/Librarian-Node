# M1-FINDING-SECURITY-CLASSIFICATION-SCHEMA-001 — Schema Divergence

**Status:** Pending (boundary contract defined in M1; storage decision deferred to implementation planning)
**Date:** 2026-08-06
**Protocol:** RUST-MIGRATION-M1 planning; `docs/architecture/RUST-CORE-CONFORMANCE-SPECIFICATION.md` §9 (Divergence Protocol)
**Recorded by:** RUST-MIGRATION-M1 planning checkpoint (work order `RUST-MIGRATION-M1.md`, DRAFT)

---

## Summary

**FINDING:** Schema divergence — `capabilities.security_classification` referenced but never defined

**Classification:** Schema divergence

**Question:** Is `security_classification`:
- **A)** persisted capability state (a column on `capabilities` that the Phase-2 DDL predates)
- **B)** derived registry metadata (computed from registry facts)
- **C)** inherited from capability version (bound at version level)
- **D)** projected from qualification/security assessment (produced by an external assessment, referenced not stored)

**Recommendation:** Do **not** resolve during M1 planning. Do **not** silently add the column. M1 defines the contract boundary around it; storage is decided in implementation planning.

---

## Detailed Record

| Field | Value |
|-------|-------|
| **Phase-2 DDL** | `capabilities` (schema `capability-registry-schema.sql`) — no `security_classification` column. Assurance axes are `availability`, `qualification`, `authority` |
| **Phase-3 view** | `view_operational_mode_derivation` (`capability-registry-schema-phase3.sql`, lines 192–207) references `c.security_classification` and `COALESCE(..., 'S0')` — expects the column to exist |
| **Phase-3 invariants** | PI-002: qualification_state and security_classification are independent axes; PI-003: operational_mode is DERIVED, never stored; PI-004: self-attestation prohibited; PI-005: S-level elevation requires a new qualification event, not an attribute update |
| **Phase-3 lifecycle events** | `qualification_lifecycle_events.security_classification` — the **frozen S-level at transition time** (CHECK S0–S5, may be NULL pre-classification). Audit replay reads this, not current registry state |
| **Implication** | PI-003/PI-005 imply security classification semantics exist **elsewhere** (assessment/transition provenance) while the view reads it as current capability state — the divergence is between a stored-state read and a transition-frozen fact |

---

## Consequences

1. **Do not silently add the column.** Schema evolution is a governed act (STORAGE-001 numbered-migration discipline); the Phase-3 view does not authorize a DDL change by implication.
2. **M1 contract boundary** — define `CapabilitySecurityContext` as an observational contract type (NOT a stored column decision):

```
CapabilitySecurityContext
{
    classification,       -- S0..S5 (or unclassified)
    source,               -- where the classification fact originates
    derivation,           -- how it was obtained (assessment / transition / inherited)
    evidence_reference    -- EV-XXXXX into the Evidence Plane (PI-004: no self-attestation)
}
```

3. **Storage decision** (A/B/C/D above) is explicitly deferred to M1 implementation planning, where the schema divergence record governs: no DDL change without a divergence record and owner approval.

---

## Next Action

- M1 contract phase: include `CapabilitySecurityContext` in the qualification-state contract surface (observational projection).
- M1 implementation planning: resolve storage question A/B/C/D as a governed schema decision with divergence record — not silently.
