# RUST-MIGRATION-M1-B — Registry Observation Projection Module: Snapshot Source Map

**Status:** APPROVED (owner review, 2026-08-07 — all six decision points approved as drafted)
**Epic:** EPIC-RUST-MIGRATION-1
**Phase:** 1 — M1-B (registry observation projections)
**Predecessor:** M1-B-0 contract lock `232edfb` (`RUNTIME-REGISTRY-OBSERVATION-CONTRACT-001.md`, Canonical)
**Gate:** RUST-MIGRATION-M1 gate M1-2 (registry observation contract)
**Last Updated:** 2026-08-07

**Owner approval (verbatim positions):**
1. `capability_versions.body` — **Exclude.** `content_hash` is the evidence anchor; exposing the full body would expand the observation surface without being required by the locked identity/observation semantics.
2. `capability_dependencies.created_at` — **Do not carry.** The locked `CapabilityDependency` surface governs the projection; do not grow the Rust type merely because the storage schema contains additional metadata.
3. Unknown enum values — **Fail closed.** An unknown persisted value is schema/semantic drift, not an invitation for coercion; the projection must produce an explicit invariant-violation error rather than manufacture a valid semantic state.
4. Nine new types + naming guard — **Confirm** (`AvailabilityAxis`, `AuthorityAxis`, `TypeCategory`, `CapabilityObservation`, `CapabilityVersionRecord`, `CapabilityTypeDefinition`, `RegistryOverview`, `RegistryIdentity`, `RegistryObservationEnvelope`); the `capability_types` table / `CapabilityType` enum naming guard prevents a subtle semantic collision between a registry table and the identity-level enum.
5. `registry_identity` — **SHA-256 over sorted meta pairs.** Deterministic, content-derived identity without inventing an authority-bearing identifier; sorting must be explicit and deterministic before hashing.
6. Overview zero-count groups — **Present.** A fixed group set with zero counts is the stronger observation contract (omitting empty groups would make the projection data-presence-dependent and less deterministic).

**Approved implementation sequence:** source map → commit work order → manifest/drift guard for the nine types → projection implementation → equivalence evidence.
**Retained hard boundaries:** no schema/DDL changes · no `RegistryStore` · no registry mutation · no qualification execution · no policy evaluation · no HTTP · no MCP · no independent operational-mode decision logic · no authority methods.
**Evidence target:** `Governed Registry Snapshot ≡ Rust Projection Types` — the SQL schema provides the frozen storage semantics and Rust provides the evidence-backed projection, not a competing registry authority.

---

## 0. Scope of This Artifact

Full read of the frozen capability-registry schema + concrete projection source
map for the five locked projections (**capability, versions, dependencies,
types, overview**). For each projection this map pins: source table(s), exact
columns, joins/relationships, ordering, nullability, enum/string
representation, identity source, provenance fields, SQL derivations, and
fields deliberately not exposed. It also resolves how `capability_registry_meta`
supplies `registry_identity`, and documents the SQLite transaction semantics
required for the consistent-snapshot rule.

**Boundary (unchanged):**
- Schema is the frozen source-of-structure; no SQL behavior is silently
  promoted into a new semantic contract.
- F-2 (`security_classification` divergence) remains governed by finding
  `M1-FINDING-SECURITY-CLASSIFICATION-SCHEMA-001` — no classification column
  becomes a projection source.
- F-3 (operational-mode derivation) remains deferred to M1-D.
- No DDL changes, no new columns, no `RegistryStore` substitution, no MCP,
  no HTTP, no implementation code.

---

## 1. Frozen Schema Inventory (sources)

### 1.1 `capabilities` (Phase 2, lines 57–99)

| Column | Null | Default | Constraint / value set | Role |
|--------|------|---------|------------------------|------|
| `id` | NO (PK) | — | TEXT, `'frontend-design'` | identity |
| `name` | NO | — | TEXT | presentation |
| `type` | NO | `'skill'` | `skill\|workflow\|policy\|validator\|template` | identity (taxonomy) |
| `description` | NO | — | TEXT (agent-facing) | content — **not exposed** |
| `summary` | YES | — | TEXT | content — **not exposed** |
| `source_type` | YES | — | `builtin\|imported\|anthropic\|community\|user` | provenance — **not exposed (M1-B)** |
| `source_reference` | YES | — | TEXT (URL/origin) | provenance — **not exposed (M1-B)** |
| `source_author` | YES | — | TEXT | provenance — **not exposed (M1-B)** |
| `status` | NO | `'unreviewed'` | `unreviewed\|reviewed\|qualified\|deprecated\|revoked` (CR-I-004) | lifecycle state |
| `active_version` | YES | — | INTEGER → `capability_versions.version` (CR-I-003) | mutable pointer |
| `availability` | NO | `'registered'` | `discovered\|registered\|disabled\|removed` | assurance axis |
| `qualification` | NO | `'not_tested'` | `not_tested\|qualifying\|passed\|failed\|stale\|suspended` | assurance axis |
| `authority` | NO | `'not_submitted'` | `not_submitted\|pending_review\|approved\|rejected\|revoked` | assurance axis |
| `execution_policy` | YES | — | JSON | policy — **not exposed (M1-B)** |
| `trigger_rules` | YES | — | JSON | policy — **not exposed (M1-B)** |
| `constraint_rules` | YES | — | JSON | policy — **not exposed (M1-B)** |
| `required_context` | YES | — | JSON | policy — **not exposed (M1-B)** |
| `tags` | YES | — | JSON array | discovery — **not exposed (M1-B)** |
| `category` | YES | — | TEXT | grouping — **not exposed (M1-B)** |
| `created_at` | NO | `datetime('now')` | ISO 8601 TEXT | registry provenance — **not exposed (M1-B)** |
| `updated_at` | NO | `datetime('now')` | ISO 8601 TEXT | registry provenance — **not exposed (M1-B)** |

Indexes: `idx_capabilities_status`, `idx_capabilities_type`, `idx_capabilities_source_type` (query support; no projection impact).

### 1.2 `capability_versions` (Phase 2, lines 126–158)

| Column | Null | Default | Constraint / value set | Role |
|--------|------|---------|------------------------|------|
| `capability_id` | NO (PK part) | — | FK → `capabilities(id)` | identity |
| `version` | NO (PK part) | — | INTEGER, `CHECK (version > 0)` (CR-I-001) | identity |
| `body` | NO | — | TEXT (full instruction content) | content — **not exposed** |
| `content_hash` | NO | — | TEXT, `CHECK (content_hash != '')`, SHA-256 hex (64 chars) | evidence anchor (CR-I-002) |
| `changelog` | YES | — | TEXT | provenance |
| `author` | YES | — | TEXT | provenance |
| `review_notes` | YES | — | TEXT | provenance |
| `qualification_evidence_id` | YES | — | `EV-XXXXX` → Evidence Plane (CR-I-010) | evidence reference |
| `profile_id` | YES | — | FK → `qualification_profiles(profile_id)` | qualification reference |
| `created_at` | NO | `datetime('now')` | ISO 8601 TEXT | registry provenance |

**APPEND-ONLY** (CR-I-001: no UPDATE/DELETE at application layer). PK `(capability_id, version)`.

### 1.3 `capability_dependencies` (Phase 2, lines 171–192)

| Column | Null | Default | Constraint / value set | Role |
|--------|------|---------|------------------------|------|
| `capability_id` | NO (PK part) | — | FK → `capabilities(id)` | identity |
| `dependency_id` | NO (PK part) | — | FK → `capabilities(id)` | identity |
| `required` | NO | `1` | INTEGER (0/1) | bool |
| `relationship_type` | NO | `'requires'` | `requires\|extends\|refines\|conflicts` | enum |
| `created_at` | NO | `datetime('now')` | ISO 8601 TEXT | registry provenance |

`CHECK (capability_id != dependency_id)` (CR-I-005). Index `idx_capability_dependencies_dependency` (reverse lookup support).

### 1.4 `capability_types` (Phase 2, lines 213–230)

| Column | Null | Default | Constraint / value set | Role |
|--------|------|---------|------------------------|------|
| `capability_type_id` | NO (PK) | — | TEXT, `'design-review'` | identity |
| `name` | NO | — | TEXT | presentation |
| `description` | NO | — | TEXT | presentation |
| `category` | NO | `'standard'` | `standard\|system\|experimental\|external` | enum |
| `default_profile_id` | YES | — | FK → `qualification_profiles(profile_id)` | reference |
| `default_policy_id` | YES | — | FK → `policies(policy_id)` | reference |
| `created_at` | NO | `datetime('now')` | ISO 8601 TEXT | registry provenance |
| `updated_at` | NO | `datetime('now')` | ISO 8601 TEXT | registry provenance |

### 1.5 `capability_registry_meta` (Phase 2, lines 526–544; Phase 3, lines 248–254)

| key | value (stored) |
|-----|----------------|
| `schema_version` | `'2'` — **quirk: `INSERT OR IGNORE` keeps `'2'` after Phase 3 runs** (key already exists; Phase 3's `'3'` is ignored) |
| `phase2_migrated_at` | stored timestamp |
| `phase2_migration` | `'CAPABILITY-REGISTRY-PHASE2-MIGRATION-1'` |
| `subsystem` | `'CAPREG'` |
| `created_at` | stored timestamp (migration time) |
| `phase3_migrated_at` | stored timestamp (if Phase 3 applied) |
| `phase3_migration` | `'CAPABILITY-REGISTRY-PHASE3-QUALIFICATION-EXECUTION-1'` (if applied) |

**`registry_identity` source** — resolved in §3.

### 1.6 Not M1-B sources (boundary note)

- `qualification_profiles`, `capability_qualifications`, `qualification_lifecycle_events`,
  `qualification_evidence_records`, `policies`, `policy_bindings` — qualification/policy
  state, **M1-C and later gates** (M1-3/M1-4). Not read by the five M1-B projections.
- All views (`view_capabilities_broken_active_version`, `view_capabilities_invalid_status`,
  `view_capabilities_qualified_without_assurance`, `view_capabilities_stale_qualification`,
  `view_policy_bindings_inactive`, `view_qualification_resolvability`,
  `view_operational_mode_derivation`) — invariant-enforcement and M1-C/M1-D surfaces.
  **Not projection sources.** `view_operational_mode_derivation` additionally references
  `capabilities.security_classification` (absent from DDL) — governed by F-2, not resolvable.

---

## 2. Projection Source Map

### 2.1 `capability(id)` projection

| Source table | Join | Ordering |
|---|---|---|
| `capabilities` | none (single-row lookup by `id`) | n/a (keyed) |

| Pinned field | Column | Null | Enum/representation | Rust contract type |
|---|---|---|---|---|
| `capability_id` | `id` | NO | TEXT | `CapabilityId` (M1-A) |
| `name` | `name` | NO | TEXT | `String` |
| `capability_type` | `type` | NO | `skill\|workflow\|policy\|validator\|template` | `CapabilityType` (M1-A) |
| `version` | `active_version` | YES | INTEGER > 0; NULL = none; **0 → unrepresentable (CHECK violation, CR-I-003)** → null + violation note | `Option<CapabilityVersion>` (M1-A) |
| `lifecycle_state` | `status` | NO | `unreviewed\|reviewed\|qualified\|deprecated\|revoked` | `QualificationState` (M1-A) |
| `availability` | `availability` | NO | `discovered\|registered\|disabled\|removed` | **NEW** `AvailabilityAxis` |
| `qualification` | `qualification` | NO | `not_tested\|qualifying\|passed\|failed\|stale\|suspended` | `QualificationAxis` (M1-A) |
| `authority` | `authority` | NO | `not_submitted\|pending_review\|approved\|rejected\|revoked` | **NEW** `AuthorityAxis` |

Payload = locked `CapabilityIdentity` (5 fields) + assurance axes (3 fields) →
**NEW** `CapabilityObservation` (`REGISTRY-OBSERVATION-CONTRACT-001` §1.1 "identity +
lifecycle + assurance axes"). Assurance axes are **columns**, not joins — CR-I-007
enforcement (qualified-without-assurance) is a view surface, not part of this payload.

**Deliberately not exposed:** `description`, `summary`, `source_type`,
`source_reference`, `source_author`, `execution_policy`, `trigger_rules`,
`constraint_rules`, `required_context`, `tags`, `category`, `created_at`,
`updated_at`. (Content, policy, and discovery material; identity+lifecycle+assurance
only per contract §3.)

**SQL derivation:** none — all fields are direct column reads. `created_at`/
`updated_at` are stored defaults, not read-time derivations.

### 2.2 `capability_versions(id)` projection

| Source table | Join | Ordering |
|---|---|---|
| `capability_versions` | FK `capability_id` (implicit) | `version` **ASC** (CR-I-001 append-only; REGISTRY-OBSERVATION §3) |

| Pinned field | Column | Null | Enum/representation | Rust contract type |
|---|---|---|---|---|
| `capability_id` | `capability_id` | NO | TEXT | `CapabilityId` (M1-A) |
| `version` | `version` | NO | INTEGER > 0 | `CapabilityVersion` (M1-A) |
| `content_hash` | `content_hash` | NO | SHA-256 hex, 64 chars, non-empty | `String` (evidence anchor; CR-I-002) |
| `changelog` | `changelog` | YES | TEXT | `Option<String>` |
| `author` | `author` | YES | TEXT | `Option<String>` |
| `review_notes` | `review_notes` | YES | TEXT | `Option<String>` |
| `qualification_evidence_id` | `qualification_evidence_id` | YES | `EV-XXXXX` | `Option<String>` (evidence REFERENCE — Evidence Plane owns proof) |
| `profile_id` | `profile_id` | YES | FK TEXT | `Option<String>` (reference; no profile evaluation) |
| `created_at` | `created_at` | NO | ISO 8601 TEXT | `String` (registry provenance) |

→ **NEW** `CapabilityVersionRecord`.

**Deliberately not exposed:** `body` (full instruction content — observation exposes
existence + evidence anchor, not payload; the Evidence Plane anchors
`capability_id + version + content_hash`, so `content_hash` IS the exposure).
**Flagged for owner confirmation (§6).**

**SQL derivation:** none.

### 2.3 `capability_dependencies(id)` projection

| Source table | Join | Ordering |
|---|---|---|
| `capability_dependencies` | FK `capability_id` (implicit) | `(capability_id, dependency_id)` **ASC** (REGISTRY-OBSERVATION §3) |

| Pinned field | Column | Null | Enum/representation | Rust contract type |
|---|---|---|---|---|
| `capability_id` | `capability_id` | NO | TEXT | `CapabilityId` (M1-A) |
| `dependency_id` | `dependency_id` | NO | TEXT | `CapabilityId` (M1-A) |
| `required` | `required` | NO | INTEGER 0/1 → bool | `bool` |
| `relationship_type` | `relationship_type` | NO | `requires\|extends\|refines\|conflicts` | `CapabilityRelationshipType` (M1-A) |

Payload = `CapabilityDependency` (M1-A) **verbatim** — locked type, no field growth.
`created_at` is registry provenance **not carried** (keeps the locked surface intact).
Outgoing dependencies only (the observed capability's own rows; reverse lookup is a
query-support index, not a projection).

**Deliberately not exposed:** `created_at` (provenance retained in registry, not in
payload). **Flagged for owner confirmation (§6).**

**SQL derivation:** none.

### 2.4 `capability_types()` projection

| Source table | Join | Ordering |
|---|---|---|
| `capability_types` | none | `capability_type_id` **ASC** (REGISTRY-OBSERVATION §3) |

| Pinned field | Column | Null | Enum/representation | Rust contract type |
|---|---|---|---|---|
| `capability_type_id` | `capability_type_id` | NO | TEXT | `String` |
| `name` | `name` | NO | TEXT | `String` |
| `description` | `description` | NO | TEXT | `String` |
| `category` | `category` | NO | `standard\|system\|experimental\|external` | **NEW** `TypeCategory` |
| `default_profile_id` | `default_profile_id` | YES | FK TEXT | `Option<String>` (reference) |
| `default_policy_id` | `default_policy_id` | YES | FK TEXT | `Option<String>` (reference) |

→ **NEW** `CapabilityTypeDefinition`.

**Naming guard (explicit):** `capability_types` TABLE rows are **not** the
`CapabilityType` M1-A enum (skill/workflow/policy/validator/template = the
`capabilities.type` COLUMN value set). Two distinct concepts; the projection returns
taxonomy rows; the M1-A enum types the capability's single type. Do not conflate.

**Deliberately not exposed:** `created_at`, `updated_at` (taxonomy timestamps;
provenance retained in registry). **Flagged for owner confirmation (§6).**

**SQL derivation:** none.

### 2.5 `registry_overview()` projection

Sources: all four tables above, **same snapshot** (single transaction, §4).
Counts are deterministic functions of the snapshot.

| Count | Source | Ordering of groups |
|---|---|---|
| `capability_count` | `capabilities` (count) | — |
| `by_status` | `capabilities.status` | `QualificationState` enum `as_str` order |
| `by_availability` | `capabilities.availability` | `AvailabilityAxis` enum `as_str` order |
| `by_qualification` | `capabilities.qualification` | `QualificationAxis` enum `as_str` order |
| `by_authority` | `capabilities.authority` | `AuthorityAxis` enum `as_str` order |
| `by_type` | `capabilities.type` | `CapabilityType` enum `as_str` order |
| `version_count` | `capability_versions` (count) | — |
| `dependency_count` | `capability_dependencies` (count) | — |
| `dependency_by_relationship` | `capability_dependencies.relationship_type` | `CapabilityRelationshipType` enum `as_str` order |
| `type_count` | `capability_types` (count) | — |
| `types_by_category` | `capability_types.category` | `TypeCategory` enum `as_str` order |

→ **NEW** `RegistryOverview`. Group keys use the serialized enum names; groups with
zero count are **present** (deterministic; same snapshot → same group set).

**Not included:** any qualification/policy/evidence counts (M1-C); any operational
mode or resolvability fact (M1-D / F-3); any `security_classification` fact (F-2).

**SQL derivation:** counts only (aggregate); no CASE logic, no view dependency.

---

## 3. `registry_identity` Resolution (from `capability_registry_meta`)

Per `RUNTIME-REGISTRY-OBSERVATION-CONTRACT-001` §3.1 — allowed source (a):
**existing registry metadata table** (`capability_registry_meta`).

| Sub-field | Pinned mapping |
|---|---|
| `value` | Deterministic, stable, content-derived: SHA-256 over the **sorted** `(key, value)` pairs of `capability_registry_meta` (pattern precedent: `RegistryStore::compute_registry_id` — precedent only, NOT a substitution) |
| `source` | `"capability_registry_meta"` (literal, fixed) |
| `derivation` | `"sha256 over sorted capability_registry_meta key/value pairs"` (literal, fixed) |

**Fail-closed rule (enforced):** if the meta table is absent, empty, or missing any
of the required keys (`subsystem`, `created_at`, `schema_version`), the projection
operation **fails** — no projection is served with a placeholder, generated,
node-derived, or absent identity.

**Forbidden (confirmed):** node-generated UUID, runtime process ID, timestamp-derived
identity (observation-time), transport-generated identity.

**Quirk handling (explicit):** `schema_version` remains `'2'` after Phase 3
(`INSERT OR IGNORE`). This is frozen schema behavior — the identity hashes the actual
stored pairs, so the quirk is captured as-is and deterministic. **Do not "fix" the
quirk** (a silent change would alter the identity and diverge from the governed DB).

`registry_identity` is read in the **same snapshot** as the projection payload (§4).

---

## 4. SQLite Transaction Semantics (Consistent Snapshot)

Contract §4: *"A projection operation MUST observe a consistent registry snapshot.
Partial reads across registry versions are invalid."*

**Required transactional property (documented; no driver decision made):**

1. **One read-only transaction per projection function.** Every constituent
   SELECT of a projection (e.g., all four table reads of `registry_overview`, or a
   `capabilities` read + its meta read for identity) executes **within a single
   transaction on a single connection**.
2. **Snapshot consistency:** SQLite's default transaction model gives every read in
   a deferred transaction the same snapshot (a SHARED lock is taken at first read and
   held until commit/rollback; no intervening write from another connection can
   appear mid-transaction). This guarantees a capability, its versions, and its
   dependencies — where read together — come from one consistent state.
3. **Read-only:** the transaction issues SELECT only; it is committed (or rolled
   back — equivalent for read-only) at the end of the projection function.
4. **No per-subquery transactions:** opening a new transaction per sub-query would
   allow mixed-version observations (structurally valid, semantically invalid) —
   the exact failure mode the contract forbids.
5. **No driver/async decision here:** the carried decision (rusqlite today; async/
   concurrent driver reconsidered only when concurrency requirements exist) stands.
   WAL note (informational): in WAL mode readers do not block writers and each
   read transaction still observes a consistent snapshot — the property holds
   regardless of journal mode; the module must still use one transaction per
   projection.

**Verification hook (for the implementation increment):** a test asserting that two
consecutive reads of the same projection function observe identical facts, and that
the identity read and the payload read share the snapshot.

---

## 5. Boundary Confirmations

- **F-2:** `security_classification` is NOT a pinned source column (absent from
  DDL; finding governs). No `CapabilitySecurityContext` material in M1-B payloads.
- **F-3:** no operational mode / mode explanation / resolvability facts in any
  M1-B projection (deferred to M1-D). No `view_operational_mode_derivation` read.
- **No DDL changes, no new columns, no schema mutation.**
- **No `RegistryStore` substitution** (routing-authority registry is a different
  surface, §2.2 of the contract).
- **No MCP, no HTTP, no implementation code** in this increment.
- One registry observation lineage; suite `REGISTRY-OBSERVATION-SCHEMA-001`.

---

## 6. Decision Points (for owner review)

All six approved as drafted by owner review 2026-08-07 (see header approval record).

1. **`capability_versions.body` excluded** — version projection exposes
   existence + `content_hash` (evidence anchor), not instruction payload.
   **APPROVED: excluded.**
2. **`capability_dependencies.created_at` not carried** — payload is the locked
   M1-A `CapabilityDependency` verbatim (no surface growth). **APPROVED: not carried.**
3. **Unknown enum values → fail closed** — any `capabilities` axis/`type` column
   holding an unrepresentable value (e.g., no CHECK constraints on the axis
   columns) fails the projection with an explicit invariant-violation error (no
   silent default, no coercion). **APPROVED: fail closed.**
4. **Required new contract types** (added at M1-B implementation, entering the
   manifest per M1-A pattern): `AvailabilityAxis`, `AuthorityAxis`, `TypeCategory`,
   `CapabilityObservation`, `CapabilityVersionRecord`, `CapabilityTypeDefinition`,
   `RegistryOverview`, `RegistryIdentity`, `RegistryObservationEnvelope`. **APPROVED:
   set + the two-`type` naming guard confirmed.**
5. **`registry_identity` value composition** — SHA-256 over sorted meta pairs
   (content-derived, like the registry_id precedent). **APPROVED: SHA-256 over
   sorted meta pairs; sorting explicit and deterministic before hashing.**
6. **Overview zero-count groups present** — deterministic group sets. **APPROVED: present.**

---

## 7. References

- Contract (locked): `contracts/runtime-api/RUNTIME-REGISTRY-OBSERVATION-CONTRACT-001.md` (Canonical, `232edfb`)
- Semantic parent: `contracts/capability/REGISTRY-OBSERVATION-CONTRACT-001.md` (Canonical)
- Schema Phase 2: `librarian-core/assets/schema/capability-registry-schema.sql` (frozen)
- Schema Phase 3: `librarian-core/assets/schema/capability-registry-schema-phase3.sql` (frozen)
- Findings: `conformance/divergences/M1-FINDING-SECURITY-CLASSIFICATION-SCHEMA-001.md` (F-2), `M1-FINDING-OPERATIONAL-MODE-DERIVATION-001.md` (F-3)
- M1-A type surface: `conformance/contract-surface/contract-surface-manifest.json` (lock `db4d281`)
- M1 work order: `work-orders/RUST-MIGRATION-M1.md`
