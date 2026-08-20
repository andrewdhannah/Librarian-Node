# M1-B Completion Record — Governed Registry Observation Projection Boundary

**Receipt ID:** M1-B-COMP-20260807
**Date:** 2026-08-07
**Repository:** Librarian-Node
**Epic:** EPIC-RUST-MIGRATION-1
**Predecessor:** M1-B-0 contract lock `232edfb` (`RUNTIME-REGISTRY-OBSERVATION-CONTRACT-001.md`, Canonical)

---

## Verdict

**M1-B PASS — Governed Registry Observation Projection Boundary Established.**

The Rust runtime exposes evidence-backed registry state through a governed observation projection boundary. It does not become a registry authority, ownership authority, qualification authority, or policy authority. The projection layer reads from the frozen governed capability-registry SQLite schema, produces locked contract types, and does not introduce competing registry authority.

This artifact seals the observation projection boundary. It does not imply completion of all registry exposure work. Transport adapters (MCP, HTTP) are deferred; any future adapter must consume the governed projection module.

---

## Acceptance Gates

| Gate | Requirement | Result |
|------|-------------|--------|
| M1-B-1 | Contract canonicalized (`RUNTIME-REGISTRY-OBSERVATION-CONTRACT-001.md`) | ✅ PASS — `232edfb` |
| M1-B-2 | Surface lock complete (9 M1-B types pinned in manifest) | ✅ PASS — `ba66aa6` |
| M1-B-3 | M1-A type surface available (`CapabilityDependency`, axis enums, `CapabilityType`) | ✅ PASS — `db4d281` |
| M1-B-4 | Source map and schema alignment verified (work order planning artifact) | ✅ PASS — work order `99c7f0b` |
| M1-B-5 | Projection module implemented (5 read projections, sealed state) | ✅ PASS — `a833ee8` |
| M1-B-6 | Consistent snapshot boundary verified (single `BEGIN…COMMIT`, `PRAGMA query_only`) | ✅ PASS — `a833ee8` |
| M1-B-7 | Projection equivalence verified (13 tests, field-for-field against governed snapshot) | ✅ PASS — 13/13 |
| M1-B-8 | Fail-closed identity and error handling verified (absent/empty/missing-key, unknown enums) | ✅ PASS — test suite |
| M1-B-9 | No authority path introduced (no mutation, no HTTP, no MCP) | ✅ PASS — code review |

**All 9 gates: PASS**

---

## Commit Sequence

| Commit | Content | Gate |
|--------|---------|------|
| `97c70a7` | M1-0 capability contract surface lock | M1-0 |
| `232edfb` | M1-B-0 registry observation contract lock | M1-B-1 |
| `ba66aa6` | M1-A implementation types (contract types, manifest, drift guard 18 tests) | M1-B-2, M1-B-3 |
| `a833ee8` | M1-B governed registry observation projection boundary | M1-B-4 through M1-B-9 |
| `<seal>` | M1-B completion record | Seal |

---

## Evidence Inventory

### Projection Module (`librarian-node/src/registry_observation.rs`)

| Component | Description | Boundary |
|-----------|-------------|----------|
| `RegistryObservationState` | Sealed state (node_id + db_path); `new()` only, no authority methods | Module-level only |
| `with_snapshot` | One read-only SQLite connection (`SQLITE_OPEN_READ_ONLY`, `PRAGMA query_only = ON`) inside a single `BEGIN…COMMIT` transaction | Consistent-snapshot (work order §4) |
| `capability(id)` | Single capability observation: 5 identity fields + 3 assurance axes | Read-only |
| `capability_versions(id)` | Version records ascending by integer version; body never selected | Read-only |
| `capability_dependencies(id)` | Locked `CapabilityDependency` structs via `new()`; ordered `(capability_id, dependency_id)` | Read-only |
| `capability_types()` | Type taxonomy with `CapabilityTypeDefinition`; ordered by `CategoryOrder` | Read-only |
| `registry_overview()` | 11-count overview with fixed group sets; zero-count groups present | Read-only |
| `registry_identity()` | SHA-256 over sorted `capability_registry_meta` key/value pairs | Content-derived, fail-closed |

### Equivalence Tests (`librarian-node/tests/registry_observation.rs`)

| Test | Property Verified |
|------|-------------------|
| `capability_projection_matches_fixture_field_for_field` | 8 fields match locked contract type exactly |
| `versions_projection_is_ascending_and_exposes_hash_never_body` | Body never exposed; content_hash present; ascending order |
| `dependencies_projection_is_locked_verbatim_and_ordered` | `CapabilityDependency::new()` validates; created_at never carried |
| `type_taxonomy_projection_is_ordered_and_not_the_type_enum` | Ordered by `CategoryOrder`, not `CapabilityType`; `default_policy_id` present |
| `overview_counts_and_fixed_groups_match_fixture` | 11 keys locked; forbidden fact-kind keys absent |
| `consecutive_reads_observe_identical_snapshot_facts` | Identity and payload move together on governed change |
| `payload_is_deterministic_except_observed_at` | Same state → same bytes (modulo timestamp) |
| `fail_closed_unknown_axis_value` | Unknown availability/authority/qualification value → invariant-violation error |
| `fail_closed_registry_identity_absent_or_empty` | Absent/empty meta table → fail-closed error |
| `fail_closed_registry_identity_missing_required_key` | Missing subsystem/created_at/schema_version → fail-closed error |
| `fail_closed_unrepresentable_active_version` | active_version=0 → fail-closed error (CR-I-003) |
| `fail_closed_missing_capability` | Non-existent capability ID → "not found" error |
| `no_write_path_registry_unchanged_after_all_projections` | Registry byte-identical before/after all projections |

### Test Results

| Suite | Count | Status |
|-------|-------|--------|
| Projection equivalence | 13 | ✅ PASS |
| Contracts (librarian-contracts) | 203 | ✅ PASS |
| Integration (librarian-node) | 21 | ✅ PASS |

---

## Equivalence Statement

**Frozen governed registry schema snapshot ≡ Rust observation projection.**

- The SQL schema provides the frozen storage semantics and is the frozen semantic reference.
- Rust provides the evidence-backed projection, not a competing registry authority.
- The projection layer is read-only and does not mutate registry state.
- Registry identity is content-derived from governed metadata, not runtime-generated.
- Unknown or invalid registry state fails closed — no coercion, no placeholders.

---

## Boundary Integrity

| Property | Status |
|----------|--------|
| Contract-first ordering | ✅ Preserved |
| Surface lock before implementation | ✅ Preserved |
| Registry storage remains source of state | ✅ Preserved |
| Rust is projection layer only | ✅ Preserved |
| Registry ≠ Authority | ✅ Preserved |
| Observation ≠ Mutation | ✅ Preserved |
| Projection ≠ Ownership | ✅ Preserved |
| Transport ≠ Semantic boundary | ✅ Preserved |
| Schema unchanged | ✅ Preserved |

---

## Explicit Exclusions

The following are out of scope for M1-B and remain deferred:

| Item | Target | Note |
|------|--------|------|
| MCP observation adapter | M1-C / M1 continuation | Transport deferred; see transport clarification below |
| HTTP observation routes | M1-C / M1 continuation | Transport deferred; see transport clarification below |
| Operational-mode derivation | Separate F-3 contract implementation (M1-D) | SQL derivation remains the frozen semantic reference |
| Qualification execution | Out of scope (observation only) | Contract exists; implementation deferred |
| Registry mutation | Out of scope (observation only) | Observation ≠ mutation |
| Schema evolution | Out of scope (schema frozen) | Schema is the frozen semantic source |
| Policy evaluation | Out of scope (observation only) | No policy authority introduced |

### Transport Clarification

MCP and networking substrates exist independently. Registry observation adapters over those transports are deferred. Any future adapter **MUST** consume the governed projection module and **MUST NOT** bypass it. This preserves the transport ≠ semantic invariant: the projection boundary is the only way to access governed registry state; transport layers are consumers, not alternative paths.

---

## Known Deferred Items

| Item | Status | Reference |
|------|--------|-----------|
| F-2 `security_classification` storage decision | Observational contract exists; storage representation decision deferred | `M1-FINDING-SECURITY-CLASSIFICATION-SCHEMA-001` |
| F-3 operational-mode derivation | Contract exists; implementation deferred | SQL derivation remains the frozen semantic reference |
| Qualification execution | Out of scope for M1-B (observation only) | Separate qualification authority contract |
| 48 legacy `governance::*` test-target failures | Pre-existing baseline, not introduced by M1-B, separate remediation scope | — |
| Custody test-target compile issues | Pre-existing baseline, not introduced by M1-B, separate remediation scope | `ProvenanceQuery.limit`, `RetentionPolicy.node_id/auto_archive/created_at` |

---

## Authorization

M1-B is complete. The observation projection boundary is sealed.

M1-C (adapter boundary definition) should proceed when the Owner authorizes it. M1-C should define the adapter boundary (MCP/network consumers of projections) before any transport exposure changes. Implementation follows planning.
