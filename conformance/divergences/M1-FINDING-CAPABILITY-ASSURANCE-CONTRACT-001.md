# M1-FINDING-CAPABILITY-ASSURANCE-CONTRACT-001 — Contract Provenance Gap

**Status:** Pending (recovery attempted; reconstruction candidate — owner approval required before reconstruction)
**Date:** 2026-08-06
**Protocol:** RUST-MIGRATION-M1 planning; `docs/architecture/RUST-CORE-CONFORMANCE-SPECIFICATION.md` §9 (Divergence Protocol)
**Recorded by:** RUST-MIGRATION-M1 planning checkpoint (work order `RUST-MIGRATION-M1.md`, DRAFT)

---

## Summary

**FINDING:** Contract provenance gap

**Classification:** Contract provenance gap

**Impact:** M1 contract extraction blocked for **assurance axes only** — the three independent assurance axes on `capabilities` (`availability`, `qualification`, `authority`) reference a semantics authority that does not exist in any accessible artifact.

**Resolution:** Recover original semantics OR explicitly reconstruct under owner approval. Reconstruction must be marked **Reconstructed Canonical** with declared provenance — never labeled "existing contract."

---

## Detailed Record

| Field | Value |
|-------|-------|
| **Reference** | `librarian-core/assets/schema/capability-registry-schema.sql` line 77: `-- See CAPABILITY-ASSURANCE-CONTRACT-001 §1 for semantics` (annotating the three assurance axes) |
| **What is missing** | The contract document `CAPABILITY-ASSURANCE-CONTRACT-001` (assurance-axis semantics: `availability` discovered/registered/disabled/removed; `qualification` not_tested/qualifying/passed/failed/stale/suspended; `authority` not_submitted/pending_review/approved/rejected/revoked) |
| **Schema establishes** | The contract existed conceptually (schema header + invariants CR-I-007/008/010 encode its consequences) |
| **Recovery attempt (2026-08-06)** | See Evidence below — document not found in any accessible source, history, or receipt |
| **Disposition** | Do NOT silently recreate. Do NOT label a reconstruction as an "existing contract." If reconstruction proceeds: `contracts/capability/CAPABILITY-ASSURANCE-CONTRACT-001.md`, **Status: Reconstructed Canonical**, Source: schema invariants + CR-I-* + PI-* + governance receipts |

---

## Evidence (recovery search, 2026-08-06)

| Search target | Method | Result |
|---------------|--------|--------|
| `G:\Librarian-Node` (all files) | Full-text search `CAPABILITY-ASSURANCE` | Only match in source: the schema itself (lines 15–17, 77). Remaining matches are compiled binaries under `target/` (string embedded in artifacts, not source) |
| `G:\Librarian` (Librarian-Platform-Equivalence, frozen `e42a6c6`) | `git log --all -S CAPABILITY-ASSURANCE` | Zero commits |
| `G:\OpenWork\thelibrarian` (TheLibrarian, frozen `15c5ef2`) | Directory is a plain checkout — **not a git repository**; docs + fixtures only | No `contracts/` directory; no match in `docs/` or `fixtures/` |
| `G:\Librarian\receipts\` (AR-ENTITY-001, AR-PERMISSIONS-001, AR-DECISIONS-001, AR-MQR-SPRINT-1, AR-STORAGE-001) | Full-text search | No reference to the contract document |

**Conclusion:** The contract exists only as a conceptual reference inside the frozen schema. Original semantics are not recoverable from accessible artifacts.

---

## Consequences

1. **M1-0 contract surface lock** cannot cite `CAPABILITY-ASSURANCE-CONTRACT-001` as an existing source. The assurance-axes contract must either be reconstructed (marked Reconstructed Canonical) or the contract surface must source-attribute to the frozen schema header + invariants.
2. **Audit honesty:** any reconstruction declares its provenance (schema invariants, CR-I-*, PI-*, governance receipts) — it does not claim to be the original document.
3. **Non-blocking for non-assurance M1 scope:** capability identity, versions, dependencies, ownership, and policy relationship contracts are not blocked by this gap.

---

## Next Action

On owner approval: reconstruct `contracts/capability/CAPABILITY-ASSURANCE-CONTRACT-001.md` as **Reconstructed Canonical** with declared sources, as part of M1-0 (contract surface lock). Until then, M1 remains planning-only.
