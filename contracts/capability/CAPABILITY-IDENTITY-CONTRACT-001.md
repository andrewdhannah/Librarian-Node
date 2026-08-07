# CAPABILITY-IDENTITY-CONTRACT-001.md — Capability Identity Contract

**Version:** 1.0.0
**Status:** Canonical (owner-approved lock, 2026-08-06)
**Sealed suite ID:** `CAPABILITY-IDENTITY-SCHEMA-001`
**Last Updated:** 2026-08-06

---

## 0. Purpose

Defines what a capability **is** in the registry: its identity, its versioning,
its type, and its dependency relationships. This is the M1 migration boundary
for the identity model — the contract is the boundary, not the code.

> The identity contract answers "**what does Librarian know exists**" — it says
> nothing about what is allowed to execute (that is the authority axis,
> `CAPABILITY-ASSURANCE-CONTRACT-001` §1.4).

---

## 1. Identity Model

### 1.1 Capability identity

```rust
CapabilityIdentity
{
    capability_id,      // immutable; the registry key ('frontend-design')
    name,               // human-readable display name (mutable presentation)
    type,               // skill | workflow | policy | validator | template
    version,            // active version integer (or none)
    lifecycle_state,    // unreviewed | reviewed | qualified | deprecated | revoked
}
```

### 1.2 Identity immutability

- `capability_id` is **immutable** — assigned at registration, never changed,
  never reused after removal.
- The `capabilities.id` column is the primary key and the only identity anchor
  for versions, dependencies, qualifications, policy bindings, and evidence
  references.
- Mutable fields (`name`, `description`, `summary`, `tags`, `category`) are
  presentation/metadata — they do not participate in identity.

**Explicit invariant:** capability identity persists independently of capability
qualification state. Qualification is a separate lifecycle
(`QUALIFICATION-STATE-CONTRACT-001`); a qualification transition never mutates
identity, and identity operations never change qualification state. This
prevents treating qualification as an identity mutation.

### 1.3 Source provenance (identity context)

`capabilities` carries source provenance that is part of identity context
(not identity itself):

| Field | Values |
|-------|--------|
| `source_type` | `builtin` / `imported` / `anthropic` / `community` / `user` |
| `source_reference` | URL or origin identifier |
| `source_author` | author or maintainer identity |

---

## 2. Versioning Model

### 2.1 Append-only versions (CR-I-001)

`capability_versions` is **APPEND-ONLY**: no UPDATE, no DELETE, enforced at the
application layer. Once written, a version row is immutable.

- Primary key: `(capability_id, version)`
- `version` is a positive integer (`CHECK (version > 0)`)
- Every version carries `content_hash` — SHA-256 of `body`, non-empty
  (`CHECK (content_hash != '')`, CR-I-002)

### 2.2 The only mutable pointer

`capabilities.active_version` is the **only mutable version pointer**. Pointing
it at a different version changes which version is "current" — it never mutates
a version row.

### 2.3 Deprecation and mutation discipline

- **Deprecation creates state, not a new version body:** `status → deprecated`
  is a lifecycle transition on the capability row (governance status axis,
  `CAPABILITY-ASSURANCE-CONTRACT-001` §1.5), not a version mutation.
- A **content change** creates a NEW version row (append) and optionally
  re-points `active_version` — it never edits an existing version.

### 2.4 Version ⇄ evidence binding (CR-I-010)

`capability_versions.qualification_evidence_id` references a resolvable
evidence record in the Evidence Plane. This creates the frozen, verifiable
link between execution and instruction:

```
capability_id + version + content_hash  →  Evidence Plane reference
```

---

## 3. Type Model

- A capability has **exactly one type** (`capability_types` taxonomy):
  `skill` (default) | `workflow` | `policy` | `validator` | `template`
- A type has one default qualification profile and one default policy
  (`capability_types.default_profile_id`, `default_policy_id`)
- Types are governed artifacts (owner awareness required); category:
  `standard` | `system` | `experimental` | `external`

---

## 4. Dependency Model

`capability_dependencies` — dependencies are **references, not embedded
objects**. Resolution loads the full dependency chain before execution.

| Field | Semantics |
|-------|-----------|
| `capability_id` | the capability that has the dependency |
| `dependency_id` | the capability it depends on |
| `required` | `1` required / `0` optional |
| `relationship_type` | `requires` / `extends` / `refines` / `conflicts` |

- **CR-I-005:** cycles MUST be detected and rejected at resolution time.
- A capability cannot depend on itself (`CHECK (capability_id != dependency_id)`).
- Reverse lookups are supported (what depends on this capability?).

---

## 5. Deterministic Serialization

- Serialization format: JSON.
- Object key order: **alphabetical** (serde_json default; no preserve_order) —
  byte order is NOT part of the equivalence surface.
- Enum strategy: serde rename_all `snake_case` for identity values.
- `capability_id` pattern: `^[A-Za-z0-9_-]+$` (matches schema examples like
  `frontend-design`).
- The same registry state MUST produce the same serialized identity across
  runs and implementations (C2 determinism requirement).

---

## 6. Authority Restrictions

- Identity projection is **read-only observation**: MAY report identity,
  versions, types, dependencies.
- Identity observation does **NOT** grant: qualification, permission, or
  execution authorization.

```
Capability ≠ Permission     — presence grants nothing
Registry   ≠ Authority      — knowing something exists ≠ allowing it to run
```

---

## 7. Equivalence Rules

- Rust identity projection MUST match the registry tables field-for-field
  (identity, versions, dependencies, types).
- Deterministic serialization: identical registry state → identical projection.
- Version ordering is by integer `version`, ascending.

---

## 8. References

- Schema (source evidence): `librarian-core/assets/schema/capability-registry-schema.sql`
  (tables: `capabilities`, `capability_versions`, `capability_dependencies`,
  `capability_types`; CR-I-001/002/005/010)
- Assurance axes semantics: `contracts/capability/CAPABILITY-ASSURANCE-CONTRACT-001.md`
- M1 work order: `work-orders/RUST-MIGRATION-M1.md`
- Conformance spec: `docs/architecture/RUST-CORE-CONFORMANCE-SPECIFICATION.md`
