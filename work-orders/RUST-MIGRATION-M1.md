# RUST-MIGRATION-M1 — Capability Registry & Qualification Semantics (Planning)

**Status:** DRAFT — Planning Only (awaiting owner review; no implementation files touched)
**Epic:** EPIC-RUST-MIGRATION-1
**Phase:** 1 — Capability/registry semantics (M1A) + MCP observation boundary (M1B)
**Predecessor:** `RUST-MIGRATION-M0B-COMPLETION.md` (SEALED, C2 Evidence Compatible, commit `385a437`)

---

## Objective

M0A/M0B translated and preserved an already-defined execution boundary. **M1 reconstructs the semantic ownership model behind Librarian** — it is a contract extraction and boundary definition phase, NOT a registry implementation plan.

The migration stops being mechanical translation here. M1 engages: registry ownership, capability identity, qualification state, evidence relationships, and authority boundaries. The M1 work order must answer five questions before any contract is locked:

1. **What is a capability?** (identity model: immutable identity, append-only versions, typed taxonomy, dependencies as references or embedded objects)
2. **Who owns its lifecycle?** (registry ownership model: who/what may transition state, and who only observes)
3. **What evidence is required before it becomes usable?** (qualification state model: profile → record → lifecycle event → evidence reference)
4. **What authority permits or prevents its use?** (policy relationship boundary — not policy evaluation)
5. **How does the Rust node expose this state without becoming the authority source?** (observation projection + MCP adapter)

### Core Invariant (carried from M0B, preserved verbatim)

> The Rust runtime exposes evidence-backed registry state; it does not create authority by hosting that state.

Corresponding non-collapse invariant set (all four hold independently):

```
Registry ≠ Authority      — the registry records what exists, not what may execute
Capability ≠ Permission   — a capability's presence grants nothing
Qualification ≠ Authorization — a passed qualification is not approval to run
Evidence ≠ Approval       — evidence is proof, not permission
```

### Sub-Phase Split

| Sub-phase | Boundary |
|-----------|----------|
| **M1A** | Capability registry semantics (identity, ownership, qualification state, policy relationship) |
| **M1B** | MCP observation adapter (deferred from M0B) |

M1A and M1B are separate sub-boundaries with separate contracts and gates. MCP is an observation adapter over M1A state — M1B does not blend into a single "registry+MCP" feature.

---

## Decisions (proposed positions for owner review — NOT yet locked)

| Decision | Proposed position |
|----------|-------------------|
| M1 scope | Registry semantics (M1A) + MCP observation adapter (M1B). MCP integration was deferred from M0B to M1 (M0B non-goal). |
| Conformance target | **C2 Evidence Compatible.** C3 (Qualification Compatible) NOT claimed until qualification execution paths exist (subsystem 5 territory). |
| Contract-first | SQL schema is evidence of intended structure; **the contract is the migration boundary**. Repeat the M0 pattern: schema understanding → contract extraction → surface lock → fixtures → implementation → evidence → seal. Do NOT touch the database first. |
| Registry semantics | The registry answers "**what does Librarian know exists**", not "what is allowed to execute". Explicitly prevents collapsing Registry ≠ Authority. |
| Evidence ownership | Evidence stays external — the Evidence Plane owns proof. Shape: `Capability → QualificationRecord → EvidenceReference → Evidence Plane`. No inline `evidence_blob` on capability or version. |
| Policy boundary | M1 establishes: policy exists → policy is referenced → policy context is observable. It does **not** evaluate policies or make authorization decisions. Authorization belongs to a later phase. |
| Contract source (F-1) | Do NOT recreate immediately — provenance gap recorded. Reconstruct as **Reconstructed Canonical** only with owner approval, with declared sources (schema invariants, CR-I-*, PI-*, governance receipts). Never "existing contract" (finding `M1-FINDING-CAPABILITY-ASSURANCE-CONTRACT-001`). |
| S-level storage (F-2) | Do NOT add the column silently, do NOT resolve during planning. M1 defines the observational boundary `CapabilitySecurityContext { classification, source, derivation, evidence_reference }`; storage (persisted / derived / version-inherited / projected) is a governed implementation-planning decision (finding `M1-FINDING-SECURITY-CLASSIFICATION-SCHEMA-001`). |
| Operational mode (F-3) | **Promoted** to contract candidate `OPERATIONAL-MODE-DERIVATION-CONTRACT-001`. Invariant: `SQL derivation == Rust projection` — Rust matches the projection, does not own operational mode (finding `M1-FINDING-OPERATIONAL-MODE-DERIVATION-001`). |
| SQLite driver | Carried decision: rusqlite today; async/concurrent driver reconsideration deferred until concurrency requirements exist. |

---

## Scope (M1 DOES)

### M1A — Capability registry semantics

1. **Capability identity model** (tables: `capabilities`, `capability_versions`, `capability_types`, `capability_dependencies`)
   - Lock: is identity immutable? Are versions append-only (CR-I-001, content_hash CR-I-002)? Does deprecation create a new version or mutate state? Are dependencies references (CR-I-005 cycle rejection at resolution)?
   - Likely contract shape: `CapabilityIdentity { capability_id, name, type, version, lifecycle_state }`
2. **Registry ownership model**
   - The registry records known existence; lifecycle transitions (`unreviewed → reviewed → qualified → deprecated/revoked`) are governed operations, not free mutation. Who may transition vs. who may only observe is an ownership boundary decision.
3. **Qualification state model** (tables: `qualification_profiles`, `capability_qualifications`, `qualification_lifecycle_events`, `qualification_evidence_records`)
   - Contract shapes: qualification profile, qualification record, lifecycle event (append-only, transitioner_role in system/evaluator/approver/owner), evidence reference.
   - Invalid transitions rejected (state machine, not free column writes).
   - CR-I-007: qualified availability requires profile + record + evidence reference + policy context, all resolvable. CR-I-008: profile version increment queues requalification.
4. **Policy relationship boundary** (tables: `policies`, `policy_bindings`)
   - Policy exists → referenced → context observable. CR-I-009: bindings reference active policies only.
   - No policy evaluation, no authorization decisions.

### M1B — MCP observation adapter

```
MCP request
    ↓
runtime observation
    ↓
registry projection (M1A state)
    ↓
evidence response
```

- MCP exposes registry evidence; it cannot modify capability state (no mutation path, no lifecycle transition through MCP).
- Own contract + surface lock, mirroring `RUNTIME-API-CONTRACT-001` discipline.

---

## Scope (M1 DOES NOT)

- **Policy evaluation / authorization decisions** — the Rust node does not decide authorization in M1 (authority boundary preserved; PERMISSIONS-001 precedent: permissions reference recorded decisions, they do not create authority)
- **Qualification execution paths** — no harness, no C3 claim; MQR-SPRINT-1 precedent: qualification is a consumer of the governance substrate, not new primitives
- **Scheduler / work compiler / node execution** (runtime services, M2+)
- **Registry record-level mutation API** — observation only
- **No new governance concepts** (discipline from ENTITY-001/PERMISSIONS-001/DECISIONS-001/STORAGE-001/MQR-SPRINT-1: zero new governance concepts, zero new authority models)
- **Database-first implementation** — contracts precede schema-touching code
- **C3/C4 conformance claims**

---

## Contract Sources (grounding)

| Source | Role |
|--------|------|
| `librarian-core/assets/schema/capability-registry-schema.sql` (Phase 2, 544 lines) | Frozen canonical table/column/invariant definition (CR-I-001..010, assurance axes, enforcement views) |
| `librarian-core/assets/schema/capability-registry-schema-phase3.sql` (254 lines) | Frozen Phase 3: lifecycle events, evidence dimensions, PI-001..005, operational-mode derivation view |
| `docs/architecture/RUST-CORE-CONFORMANCE-SPECIFICATION.md` §7 | Subsystem registry: row 3 Registry, row 7 API layer (MCP thread) — each subsystem C0 → C2 minimum |
| `CAPABILITY-ASSURANCE-CONTRACT-001` (provenance gap — finding `M1-FINDING-CAPABILITY-ASSURANCE-CONTRACT-001.md`) | Schema line 77: "See CAPABILITY-ASSURANCE-CONTRACT-001 §1 for semantics" of the three assurance axes. Not recoverable from accessible artifacts; reconstruction only with provenance declaration |
| `work-orders/ENTITY-001-GOVERNANCE-ENTITY-REGISTRY-1.md` | Ownership model precedent (entities referenceable, lifecycle active/suspended/retired, no new governance concepts) |
| `work-orders/PERMISSIONS-001-GOVERNANCE-PERMISSIONS-1.md` | Authority separation precedent: permissions reference decisions, do not create authority |
| `work-orders/DECISIONS-001-GOVERNANCE-DECISION-RECORDS-1.md` | Durable authority records precedent (approved/rejected/deferred/superseded) |
| `work-orders/STORAGE-001-GOVERNANCE-PERSISTENCE-MATURITY-1.md` | Numbered migrations precedent (`schema_version` + `migration_log`) |
| `work-orders/MQR-SPRINT-1-MODEL-QUALIFICATION-VALIDATION-1.md` | Qualification-as-consumer precedent: map to existing contract types, no new primitives |
| `G:\Librarian` receipts `AR-ENTITY-001`, `AR-PERMISSIONS-001`, `AR-DECISIONS-001`, `AR-MQR-SPRINT-1`, `AR-STORAGE-001` | Golden completions of the governance work orders (frozen reference `e42a6c6` et al.) |

---

## Findings (from planning grounding)

Dispositions owner-approved 2026-08-06; each finding recorded in `conformance/divergences/`:

| # | Finding | Record | Disposition (approved) |
|---|---------|--------|------------------------|
| F-1 | `CAPABILITY-ASSURANCE-CONTRACT-001` referenced by frozen schema (line 77) as semantics authority for the three assurance axes; not present in any accessible artifact (recovery search 2026-08-06: `G:\Librarian-Node` full-text, `G:\Librarian` git history, `G:\OpenWork\thelibrarian` — plain checkout, no git history, no contracts dir) | `M1-FINDING-CAPABILITY-ASSURANCE-CONTRACT-001.md` | Contract provenance gap. Do NOT silently recreate. If reconstruction approved: `contracts/capability/CAPABILITY-ASSURANCE-CONTRACT-001.md` marked **Status: Reconstructed Canonical** with declared sources (schema invariants, CR-I-*, PI-*, governance receipts) — never "existing contract." Non-blocking for non-assurance M1 scope |
| F-2 | Phase-3 `view_operational_mode_derivation` references `capabilities.security_classification` (lines 192–207); column absent from Phase-2 `capabilities` DDL; PI-003/PI-005 imply classification semantics live at transition, not current state | `M1-FINDING-SECURITY-CLASSIFICATION-SCHEMA-001.md` | Schema divergence. Do NOT silently add the column. Do NOT resolve during M1 planning. M1 defines the observational contract boundary `CapabilitySecurityContext { classification, source, derivation, evidence_reference }`; storage question (persisted / derived / version-inherited / projected) deferred to implementation planning as a governed schema decision |
| F-3 | Phase-3 operational mode is a **deterministic pure function**: f(security_classification, qualification_state, evidence, constraints) → mode + explanation (`view_operational_mode_derivation`); PI-003: derived, never stored | `M1-FINDING-OPERATIONAL-MODE-DERIVATION-001.md` | **Promoted** (positive finding) to contract candidate `OPERATIONAL-MODE-DERIVATION-CONTRACT-001`. Invariant: `SQL derivation == Rust projection` — Rust matches the projection, it does not own operational mode. Inputs: security classification, qualification state, evidence state, policy constraints, lifecycle state. Outputs: mode, explanation, derivation inputs, evidence references. No authority decision. Extraction at M1-0 |

---

## Acceptance Gates (DRAFT — for owner review)

| Gate | Verification |
|------|--------------|
| M1-0 | **Contract surface lock** — capability contracts defined, lifecycle enums defined, registry response contracts defined, evidence references defined, policy relationship defined — all before code; contract-surface manifest + drift guard updated (M0B discipline) |
| M1-1 | **Capability identity contract** — capability identity loadable from registry state; identity immutable; versions represented (append-only, content_hash); deterministic serialization |
| M1-2 | **Registry observation contract** — registry state projectable (read-only); no mutation paths exist; ownership boundaries preserved (observation ≠ transition authority) |
| M1-3 | **Qualification state contract** — qualification lifecycle represented (unreviewed → reviewed → qualified → deprecated/revoked); evidence references validated (resolvable, CR-I-010); invalid transitions rejected |
| M1-4 | **Policy relationship contract** — policy context exposed and observable (exists → referenced → context); no authorization decisions made |
| M1-5 | **MCP observation boundary** — MCP exposes registry evidence; MCP cannot mutate authority state (no capability/qualification transition through MCP) |
| M1-6 | **Evidence seal** — fixtures exist; deterministic projections validated (Rust projection ≡ SQL view facts, F-3); completion record created per M0A/M0B discipline |

Conformance target: **C2 (Evidence Compatible)** — same level as M0A/M0B; C3/C4 not claimed.

---

## Semantic Recovery Rules

Captures the M0A startup-receipt divergence pattern for M1: semantic drift is surfaced **before** code reproduces it. Approved 2026-08-06 with the finding dispositions.

| Situation | Rule |
|-----------|------|
| Missing historical contract | Reconstruct only with provenance declaration — mark **Reconstructed Canonical** with declared sources; never "existing contract" (F-1) |
| Schema mismatch | Divergence record before modification — no silent DDL or column changes (F-2) |
| Derived view behavior | Rust must match the projection, not replace authority — `SQL derivation == Rust projection`; the view stays the semantic authority (F-3) |
| Unknown ownership | Block implementation until ownership is assigned — no scope creep into unowned semantics |

---

## Implementation Ordering (M0 pattern — planning only)

```
Schema understanding        (done — planning grounding above)
        ↓
Contract extraction         (M1-0: contract documents + surface lock)
        ↓
Surface lock                (manifest re-baseline + drift guard)
        ↓
Fixture creation            (deterministic registry fixtures)
        ↓
Implementation              (M1A projection module; M1B MCP adapter)
        ↓
Evidence                    (evidence/phase0/rust-core/m1a/, m1b/)
        ↓
Seal                        (RUST-MIGRATION-M1-COMPLETION.md)
```

---

## Deliverables

1. Work order: `work-orders/RUST-MIGRATION-M1.md` (this file — DRAFT)
2. Finding records (recorded 2026-08-06, pending owner approval of final M1 commit):
   - `conformance/divergences/M1-FINDING-CAPABILITY-ASSURANCE-CONTRACT-001.md` (contract provenance gap)
   - `conformance/divergences/M1-FINDING-SECURITY-CLASSIFICATION-SCHEMA-001.md` (schema divergence)
   - `conformance/divergences/M1-FINDING-OPERATIONAL-MODE-DERIVATION-001.md` (promoted contract candidate)
3. M1A contracts in `contracts/` (extracted at M1-0 — not yet created):
   - Capability identity + registry observation + qualification state (incl. `CapabilitySecurityContext` boundary per F-2) + policy relationship
   - `OPERATIONAL-MODE-DERIVATION-CONTRACT-001` (promoted per F-3)
   - `CAPABILITY-ASSURANCE-CONTRACT-001` as **Reconstructed Canonical** only with owner approval (F-1)
4. M1B contract: MCP observation adapter (mirrors `RUNTIME-API-CONTRACT-001` discipline)
5. `librarian-node` registry observation module (read-only projections; module integration, no fork)
6. MCP adapter (observation only; no mutation path)
7. Deterministic registry fixtures + projection validation (F-3 equivalence: Rust projection ≡ SQL view)
8. Evidence: `evidence/phase0/rust-core/m1a/` + `evidence/phase0/rust-core/m1b/`
9. Completion record: `work-orders/RUST-MIGRATION-M1-COMPLETION.md`

## Dependencies

- M0B SEALED (commit `385a437`): shared startup adapter, `runtime_api` observation pattern, contract-surface manifest + drift guard (8/8)
- SQLite substrate already present (governance.db; STORAGE-001 numbered-migration precedent) — M1 touches registry tables via contracts, not database-first
- TheLibrarian `15c5ef2` frozen reference (schema origin); governance completions in `G:\Librarian` (`e42a6c6` et al.)

## Estimated Effort

2 sprints (~2 weeks): M1A (registry semantics) + M1B (MCP observation adapter).

---

## Divergence Protocol

Same as M0A/M0B (`docs/architecture/RUST-CORE-CONFORMANCE-SPECIFICATION.md` §9): Rust bug → fix + regression test; Swift bug → record, do not port; contract clarification → file + re-baseline. Evidence append-only. Additions for M1: schema-vs-contract divergences (F-2 class) recorded under `conformance/divergences/` before implementation proceeds.
