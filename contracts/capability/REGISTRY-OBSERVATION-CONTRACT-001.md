# REGISTRY-OBSERVATION-CONTRACT-001.md — Registry Observation Contract

**Version:** 1.0.0
**Status:** Canonical (owner-approved lock, 2026-08-06)
**Sealed suite ID:** `REGISTRY-OBSERVATION-SCHEMA-001`
**Last Updated:** 2026-08-06

---

## 0. Purpose

Defines the observation boundary through which a Rust node projects registry
state. The registry answers **"what does Librarian know exists"** — never "what
is allowed to execute." This contract is the migration boundary for registry
observation; the contract is the boundary, not the code.

> **Registry ≠ Authority** — hosting and projecting registry state does not
> create authority. The registry remains the source of state; Rust implements
> the observation boundary.

> **The registry projection is evidence-backed state exposure, not a source of
> authority.** It reports what the registry knows, from the governed registry
> database; it confers nothing.

Invariant chain (all three hold independently, and no changes to ownership
semantics are possible through this surface):

```
Registry        ≠ Authority      — projecting state does not create authority
Observation     ≠ Mutation       — seeing state is not changing state
Projection      ≠ Ownership      — reporting state is not owning state
```

---

## 1. Scope

### 1.1 The observation contract exposes (read-only projections)

- **Capability projection** — identity + lifecycle + assurance axes for a
  capability (from `capabilities`)
- **Version projection** — append-only version history (from `capability_versions`)
- **Dependency projection** — dependency graph (from `capability_dependencies`)
- **Type taxonomy projection** — capability types (from `capability_types`)
- **Registry overview** — counts/summaries derived from the above

### 1.2 The observation contract explicitly does NOT

- Mutate, insert, update, or delete any registry row
- Transition lifecycle state (`unreviewed → reviewed → qualified → …`)
- Change assurance axes (`availability`, `qualification`, `authority`)
- Create or revoke policy bindings
- Evaluate policies or make authorization decisions
- Execute capabilities

### 1.3 Boundary shape (correct)

```
Observation request
        |
        v
Read registry state (SQLite, read-only transaction)
        |
        v
Project deterministic facts
        |
        v
Return evidence-backed projection
```

### 1.4 Boundary shape (forbidden)

```
Observation request
        |
        v
Modify registry state
        |
        v
Transition capability lifecycle
```

---

## 2. Ownership Boundary

| Actor | MAY | MUST NOT |
|-------|-----|----------|
| Observation layer (Rust projection) | read registry rows, project facts, report evidence | write rows, transition state, resolve authority |
| Registry state machine (governed operations) | transition lifecycle, update `active_version`, apply governed changes | expose mutation through the observation layer |
| Consumers (API/MCP) | query projections | mutate registry state (no write path exists) |

**Observation ≠ transition authority:** an observer can see every state; no
observer may change any state. Lifecycle transitions remain owned by the
governed registry operations — never reachable from the observation surface.
No changes to ownership semantics are possible through this surface: who may
transition vs. who may observe is fixed by this contract and cannot be altered
by an observation request.

---

## 3. Projection Determinism

- Same registry state → same projection (fields, values, ordering).
- Ordering is explicit: versions ascending by integer; dependencies by
  `(capability_id, dependency_id)`; taxonomies by `capability_type_id`.
- Serialization: JSON, alphabetical key order (serde_json default; byte order
  is NOT part of the equivalence surface).
- Enum strategy: serde rename_all `snake_case`.
- Timestamps: RFC 3339 UTC (ISO 8601 TEXT throughout the schema).

---

## 4. Evidence-Backed Projection

Every projection response carries provenance so the consumer can answer:
*what state was observed, and from where did it come?*

- Projections are read from the governed registry database (SQLite) in a
  read-only transaction.
- Observed facts carry the registry identity (node/project context) and an
  observation timestamp.
- No projection generates new evidence events — it reports existing state
  (receipt/evidence generation remains owned by the startup protocol and
  governed operations, mirroring RUNTIME-API-CONTRACT-001 §4.3 retrieval
  semantics).

---

## 5. Equivalence Rules

- Rust projections MUST match the registry tables field-for-field (identity,
  versions, dependencies, types, assurance axes).
- Registry view outputs and Rust projections MUST agree on deterministic facts
  where both exist (e.g., invariant-enforcement views report violations; Rust
  observation reports the same underlying rows).
- Deterministic: same registry state, same projection, across runs and
  implementations (C2 evidence pattern).

---

## 6. References

- Schema (source evidence): `librarian-core/assets/schema/capability-registry-schema.sql`
- Assurance axes semantics: `contracts/capability/CAPABILITY-ASSURANCE-CONTRACT-001.md`
- Identity model: `contracts/capability/CAPABILITY-IDENTITY-CONTRACT-001.md`
- M1 work order: `work-orders/RUST-MIGRATION-M1.md`
- M0B observation precedent: `contracts/runtime-api/RUNTIME-API-CONTRACT-001.md`
