# RUNTIME-REGISTRY-OBSERVATION-CONTRACT-001.md — Registry Observation Projection Contract (M1-B read boundary)

**Version:** 1.0.0
**Status:** Canonical (owner-approved lock, 2026-08-07)
**Sealed suite ID:** `REGISTRY-OBSERVATION-SCHEMA-001` (parent-aligned — the RUNTIME projection adapter contract belongs to the registry observation suite, not a new suite)
**Last Updated:** 2026-08-07

**Semantic parent:** `REGISTRY-OBSERVATION-CONTRACT-001` (Canonical, locked 2026-08-06)
**Boundary precedent:** `RUNTIME-API-CONTRACT-001` (M1-B discipline: observation boundary, not execution authority)
**Gate:** RUST-MIGRATION-M1 gate M1-2 (registry observation contract) — first M1-B artifact per owner directive

### Contract lineage

```
REGISTRY-OBSERVATION-CONTRACT-001   (semantic contract — what registry observation IS)
        |
        v
RUNTIME REGISTRY PROJECTION ADAPTER (this contract — how a Rust node exposes those
                                     semantics at runtime)
```

"Runtime" describes the consumer context, not the semantic contract. Both
documents share suite `REGISTRY-OBSERVATION-SCHEMA-001`; there is exactly one
registry observation contract lineage.

---

## 0. Purpose

Defines the **read boundary** through which consumers query a Librarian Rust
node for evidence-backed registry observation projections. This is the
node-side surface for `REGISTRY-OBSERVATION-CONTRACT-001`: it fixes *how* a
node exposes the registry state it observes, without the node becoming the
owner of registry truth.

> The success criterion is not "the Rust node can read the registry." It is:
>
> **A consumer can query a Rust node and receive trustworthy, evidence-backed
> registry state — and no query path can change that state.**

The contract is the boundary, not the code. Router/handler code follows this
contract only after it is locked (M1-B-0 surface lock, then implementation).

### Core invariant (carried from REGISTRY-OBSERVATION-CONTRACT-001, preserved verbatim)

> The registry projection is evidence-backed state exposure, not a source of
> authority. It reports what the registry knows, from the governed registry
> database; it confers nothing.

Invariant chain (all three hold independently):

```
Registry        ≠ Authority      — projecting state does not create authority
Observation     ≠ Mutation       — seeing state is not changing state
Projection      ≠ Ownership      — reporting state is not owning state
Transport       ≠ Semantic       — adapters consume projections; they do not
                                   define observation semantics
```

### Source authority statement

> The governed capability-registry schema is the observation source. Rust
> structures represent projections of that state and do not become an
> alternate registry authority.

---

## 1. Scope

### 1.1 The read boundary exposes (module-level Rust read surface)

Mirrors `REGISTRY-OBSERVATION-CONTRACT-001` §1.1, node-side. The read surface
is a **module-level Rust surface**: function-level, read-only projections over
the governed registry state. It is NOT an HTTP route surface and NOT an MCP
surface.

> Transport adapters (MCP, HTTP) consume registry observation projections;
> they do not define registry observation semantics. MCP and HTTP are
> consumers of the projection boundary, not the boundary itself.

Projections exposed:

- **Capability projection** — identity + lifecycle + assurance axes for a
  capability (from `capabilities`)
- **Version projection** — append-only version history (from `capability_versions`)
- **Dependency projection** — dependency graph (from `capability_dependencies`)
- **Type taxonomy projection** — capability types (from `capability_types`)
- **Registry overview** — counts/summaries derived from the above

Every projection is carried in an observation envelope (see §3) with
provenance: what node observed, what registry state was read, when.

### 1.2 The read boundary explicitly does NOT

- Mutate, insert, update, or delete any registry row (no write path exists)
- Transition lifecycle state (`unreviewed → reviewed → qualified → …`)
- Change assurance axes (`availability`, `qualification`, `authority`)
- Create or revoke policy bindings
- Evaluate policies or make authorization decisions
- Execute capabilities
- Implement MCP or HTTP behavior (transport adapters are separate consumers
  of these projections; they are not part of this contract)
- Define registry observation semantics beyond this lineage (one suite, one
  semantic source)

### 1.3 Boundary shape (correct)

```
Observation request
        |
        v
Read governed registry state (SQLite, read-only transaction, consistent snapshot)
        |
        v
Project deterministic facts (frozen table fields, explicit ordering)
        |
        v
Return evidence-backed observation envelope
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

### 2.1 Hierarchy (fixed)

```
Governed Registry Storage
          |
          v
Registry Observation Projection Module   (this contract — the read boundary)
          |
          +----------------+
          |                |
          v                v
       MCP Adapter     HTTP/API Adapter (future)
```

The projection module is the semantic boundary. Transport adapters hang off it;
they never sit between the registry and the projections, and they never define
registry observation semantics.

| Actor | MAY | MUST NOT |
|-------|-----|----------|
| Observation layer (Rust node read boundary) | read registry rows (read-only tx), project facts, report evidence | write rows, transition state, resolve authority |
| Registry state machine (governed operations) | transition lifecycle, update `active_version`, apply governed changes | expose mutation through the observation layer |
| Transport adapters (MCP / future HTTP) | consume projections, expose them to consumers | define observation semantics, mutate registry state, bypass the projection module |

**Node ≠ registry owner.** The governed registry database is the source of
registry truth; the Rust node hosts an observation surface over it. Hosting
and projecting state does not make the node the registry, and does not grant
it transition authority. No changes to ownership semantics are possible
through this surface.

**Source authority:** the governed capability-registry schema is the
observation source. Rust structures represent projections of that state and do
not become an alternate registry authority.

### 2.2 Data source distinction (explicit)

The read boundary reads the **governed capability-registry database** — the
SQLite database hosting the capability-registry schema
(`librarian-core/assets/schema/capability-registry-schema.sql` Phase 2 +
`-phase3.sql`). It does NOT read `librarian-core::registry` (`RegistryStore`),
which persists routing-authority state (manifests, Owner decisions, execution
profiles, router projections). Those are different surfaces: routing authority
vs. capability existence. Mixing them would collapse `Registry ≠ Authority`.

---

## 3. Read Surface (module-level projections)

Minimal and read-only. Projection functions and their deterministic ordering:

| Projection | Reads | Returns (deterministic) |
|------------|-------|--------------------------|
| `capability(id)` | `capabilities` | `CapabilityIdentity` + assurance axes (availability, qualification, authority) |
| `capability_versions(id)` | `capability_versions` | append-only versions, ascending by integer (CR-I-001) |
| `capability_dependencies(id)` | `capability_dependencies` | dependency references, ordered by `(capability_id, dependency_id)` (CR-I-005) |
| `capability_types()` | `capability_types` | taxonomy, ordered by `capability_type_id` |
| `registry_overview()` | all of the above | counts/summaries derived from the same rows |

All responses are JSON-serializable and use the M1-A contract types for the
projection payload (`librarian-contracts` `node::capability_registry` family —
suite surface locked M1-A). Enum strategy: serde rename_all `snake_case`;
timestamps RFC 3339 UTC.

### 3.1 Observation envelope (every projection response)

```
{
  "node_id":             "<observing node>",
  "registry_identity":   {
                           "value":      "<implementation-defined identity value>",
                           "source":     "<authoritative origin of the identity>",
                           "derivation": "<how the value was obtained>"
                         },
  "projection_observed_at": "RFC 3339 UTC (variable field)",
  "projection":            { ... M1-A payload, fully deterministic ... }
}
```

**`registry_identity` semantics** (structured provenance, same pattern used
elsewhere — never an unbound field, never an invented source):

| Sub-field | Meaning |
|-----------|---------|
| `value` | Implementation-defined identity value |
| `source` | Authoritative origin of the identity |
| `derivation` | How the value was obtained |

Allowed sources (M1-B):
- existing registry metadata table (e.g., `capability_registry_meta`)
- governed database identity metadata
- migration-defined registry identity artifact

Forbidden sources:
- node-generated UUID
- runtime process ID
- timestamp-derived identity
- transport-generated identity

**Fail-closed rule:** a projection MUST fail closed if the registry identity
cannot be established — no projection is served with a placeholder,
node-generated, or absent identity. Identity is part of the evidence-backed
envelope, not an optional decoration.

**`projection_observed_at` semantics** (freshness boundary):

- Records the observation time of the projection.
- Does NOT represent registry mutation time.
- Does NOT imply freshness of registry state without registry evidence.

This prevents later consumers from treating observation time as state
freshness. `projection_observed_at` is the only variable field (same
discipline as `RUNTIME-API-CONTRACT-001` §4.2).

The projection payload itself is deterministic: same registry state → same
bytes (excluding `projection_observed_at`), across runs and implementations.

No projection generates new evidence events — it reports existing registry
state (receipt/evidence generation remains owned by the startup protocol and
governed operations; `REGISTRY-OBSERVATION-CONTRACT-001` §4).

---

## 4. Determinism, Consistency & Equivalence

- **Transaction boundary (strengthened):** a projection operation MUST observe
  a consistent registry snapshot. Partial reads across registry versions are
  invalid. Each projection function executes against a single read-only
  transaction; a capability and its versions/dependencies are read from the
  same snapshot.
- Same registry state → same projection (fields, values, ordering).
- Ordering is explicit (see §3 table) and stable across reads.
- Rust projections MUST match the registry table fields field-for-field
  (`REGISTRY-OBSERVATION-CONTRACT-001` §5).
- Registry view outputs and Rust projections MUST agree on deterministic facts
  where both exist (e.g., invariant-enforcement views report violations; Rust
  observation reports the same underlying rows).
- Operational-mode derivation equivalence (`SQL derivation == Rust
  projection`, F-3) is a SEPARATE artifact (M1-D); it is not introduced by this
  contract and no independent decision logic may appear here.

---

## 5. Authority Restrictions

### 5.1 The read boundary

- MAY observe registry state
- MAY report registry state
- MAY expose evidence (projections, deterministic facts, envelope provenance)

### 5.2 The read boundary MUST NOT

- Authorize any action
- Mutate registry or governance state
- Transition capability lifecycle state
- Evaluate policies or resolve authorization
- Bypass or re-run governed operations
- Create, revoke, or alter authority
- Let transport adapters define observation semantics (adapters consume
  projections; they do not re-define them)

### 5.3 Enforcing invariant

The read boundary is an observation surface, not an execution authority.
Enforcement: projections are computed from a read-only transaction over the
governed registry database (single consistent snapshot per projection); the
observation state is sealed at construction (mirroring
`RUNTIME-API-CONTRACT-001` §5.3 and the M0B `RuntimeApiState` pattern); there
is no write path, no transition method, and no code path that invokes governed
registry operations from a projection query.

---

## 6. Locked References

- Semantic parent: `contracts/capability/REGISTRY-OBSERVATION-CONTRACT-001.md` (Canonical, `REGISTRY-OBSERVATION-SCHEMA-001`)
- Boundary precedent: `contracts/runtime-api/RUNTIME-API-CONTRACT-001.md` (Canonical, `RUNTIME-API-SCHEMA-001`)
- Assurance axes semantics: `contracts/capability/CAPABILITY-ASSURANCE-CONTRACT-001.md`
- Identity model: `contracts/capability/CAPABILITY-IDENTITY-CONTRACT-001.md`
- M1-A type surface: `conformance/contract-surface/contract-surface-manifest.json` (M1-A types, lock `db4d281`)
- Schema (source evidence): `librarian-core/assets/schema/capability-registry-schema.sql` + `-phase3.sql`
- M1 work order: `work-orders/RUST-MIGRATION-M1.md` (gate M1-2)
