# M1-C Completion Record — Governed Registry Observation Adapter Boundary

**Receipt ID:** M1-C-COMP-20260807
**Date:** 2026-08-07
**Repository:** Librarian-Node
**Epic:** EPIC-RUST-MIGRATION-1
**Predecessor:** M1-C0 adapter contract lock `b123f16` (`REGISTRY-OBSERVATION-ADAPTER-CONTRACT-001.md`, Canonical)

---

## Verdict

**M1-C PASS — Governed Registry Observation Adapter Boundary Established.**

Both MCP and HTTP transports consume the governed projection module through locked adapter contracts. The same projection functions produce equivalent observations across transports. No transport creates authority, mutation, or parallel registry access.

This artifact seals the observation adapter boundary. It does not imply completion of qualification execution, authorization, operational-mode decision authority, or registry mutation. Those are materially different authority questions with their own contract boundaries.

---

## Acceptance Gates

| Gate | Requirement | Result |
|------|-------------|--------|
| M1-C-1 | Adapter contract canonicalized (`REGISTRY-OBSERVATION-ADAPTER-CONTRACT-001.md`) | ✅ PASS — `b123f16` |
| M1-C-2 | MCP adapter implemented (5 `registry.observe_*` tools) | ✅ PASS — `5f5e088` |
| M1-C-3 | HTTP adapter implemented (6 `GET /registry/observe/*` routes) | ✅ PASS — `b9f92cf` |
| M1-C-4 | Projection equivalence verified (MCP output ≡ Projection output) | ✅ PASS — 13/13 |
| M1-C-5 | Projection equivalence verified (HTTP output ≡ Projection output) | ✅ PASS — 10/10 |
| M1-C-6 | Cross-adapter equivalence verified (MCP ≡ HTTP from same fixture) | ✅ PASS — 3/3 |
| M1-C-7 | Transport identity isolation verified (no session/connection ID in projection) | ✅ PASS — test suite |
| M1-C-8 | Registry immutability verified (no state transition through adapter surface) | ✅ PASS — negative-path evidence |
| M1-C-9 | No authority path introduced (no mutation, no qualification, no policy) | ✅ PASS — code review |
| M1-C-10 | Contracts green (204/204) | ✅ PASS |
| M1-C-11 | Integration tests green (21/21) | ✅ PASS |

**All 11 gates: PASS**

---

## Commit Sequence

| Commit | Content | Gate |
|--------|---------|------|
| `97c70a7` | M1-0 capability contract surface lock | M1-0 |
| `232edfb` | M1-B-0 registry observation contract lock | M1-B-1 |
| `ba66aa6` | M1-A implementation types | M1-B-2, M1-B-3 |
| `a833ee8` | M1-B projection module boundary | M1-B-4 through M1-B-9 |
| `8fd37ad` | M1-B completion seal | Seal |
| `fc4e87b` | M1-C adapter qualification (planning) | Planning |
| `b123f16` | M1-C0 adapter contract surface lock | M1-C-1 |
| `5f5e088` | M1-C1 MCP observation adapter | M1-C-2 |
| `b9f92cf` | M1-C2 HTTP observation adapter | M1-C-3 through M1-C-9 |
| `<seal>` | M1-C completion record | Seal |

---

## Equivalence Statement

**Governed Registry Snapshot ≡ Projection Module ≡ MCP Observation ≡ HTTP Observation.**

```
       Projection Module
              |
       ┌──────┴──────┐
       │              │
    MCP Adapter   HTTP Adapter
       │              │
       └──────┬──────┘
              |
    Equivalent Observations
```

- The projection module is the single source of observation truth.
- Both MCP and HTTP adapters call the same projection methods.
- Projection payloads are byte-identical across transports (modulo `projection_observed_at`).
- Registry identity is content-derived and identical across all observation paths.
- The variable `projection_observed_at` field is treated per contract: each observation records its own timestamp; the field is excluded from equivalence comparison.

---

## Evidence Inventory

### Projection Module (M1-B, sealed)

5 read projections behind consistent-snapshot transactions:

| Method | Projection Payload |
|--------|-------------------|
| `capability(id)` | `CapabilityObservation` (8 fields) |
| `capability_versions(id)` | `Vec<CapabilityVersionRecord>` (ascending, body excluded) |
| `capability_dependencies(id)` | `Vec<CapabilityDependency>` (locked struct) |
| `capability_types()` | `Vec<CapabilityTypeDefinition>` (ordered by CategoryOrder) |
| `registry_overview()` | `RegistryOverview` (11 keys, zero-count groups) |

### MCP Adapter (M1-C1)

5 `registry.observe_*` tools mapping 1:1 to projection methods:

| Tool | Projection Method | Tests |
|------|-------------------|-------|
| `registry.observe_capability` | `capability(id)` | Equivalence, isolation, immutability |
| `registry.observe_versions` | `capability_versions(id)` | Equivalence, isolation, immutability |
| `registry.observe_dependencies` | `capability_dependencies(id)` | Equivalence, isolation, immutability |
| `registry.observe_types` | `capability_types()` | Equivalence, isolation, immutability |
| `registry.observe_overview` | `registry_overview()` | Equivalence, isolation, immutability |

MCP test suite: 13/13 passing

### HTTP Adapter (M1-C2)

6 `GET /registry/observe/*` routes mapping 1:1 to projection methods:

| Route | Projection Method | Tests |
|-------|-------------------|-------|
| `GET /registry/observe/capabilities` | `capability_types()` | Equivalence, immutability |
| `GET /registry/observe/capabilities/{id}` | `capability(id)` | Equivalence, immutability |
| `GET /registry/observe/versions/{id}` | `capability_versions(id)` | Equivalence, immutability |
| `GET /registry/observe/dependencies/{id}` | `capability_dependencies(id)` | Equivalence, immutability |
| `GET /registry/observe/types` | `capability_types()` | Equivalence, immutability |
| `GET /registry/observe/overview` | `registry_overview()` | Equivalence, immutability |

HTTP test suite: 10/10 passing

### Cross-Adapter Equivalence

3 tests verifying MCP and HTTP produce identical projections from the same governed fixture:

| Test | Projection | Evidence |
|------|-----------|----------|
| `mcp_and_http_equivalent_capability_observation` | `CapabilityObservation` | Identical payload + identity |
| `mcp_and_http_equivalent_overview_observation` | `RegistryOverview` | Identical payload + identity |
| `mcp_and_http_equivalent_types_observation` | `Vec<CapabilityTypeDefinition>` | Identical payload + identity |

### Registry Immutability (Negative-Path Evidence)

The adapter surface cannot cause registry state transitions:

| Test | Scope | Evidence |
|------|-------|----------|
| `observation_adapter_does_not_modify_registry` | MCP | Registry byte-identical after all 5 MCP observations |
| `http_observation_does_not_modify_registry` | HTTP | Registry byte-identical after all 6 HTTP observations |
| `no_write_path_registry_unchanged_after_all_projections` | Projection | Registry byte-identical after all projection reads |

**Architectural assertion:** An MCP or HTTP consumer cannot cause a registry state transition through the M1-C adapter surface. The adapter is read-only; the projection module is read-only; the registry connection is `PRAGMA query_only = ON`.

### Test Results

| Suite | Count | Status |
|-------|-------|--------|
| Contracts | 204 | ✅ PASS |
| Projection equivalence | 13 | ✅ PASS |
| MCP adapter | 13 | ✅ PASS |
| HTTP adapter | 10 | ✅ PASS |
| Integration | 21 | ✅ PASS |
| **Total** | **261** | **✅ PASS** |

---

## What M1-C Demonstrated

| Property | Status | Evidence |
|----------|--------|----------|
| Semantic centralization | ✅ | Both transports consume the same projection functions |
| Transport isolation | ✅ | Neither MCP nor HTTP contributes identity to registry semantics |
| Read-only exposure | ✅ | Neither adapter creates a mutation or authority path |
| Cross-transport equivalence | ✅ | Same governed fixture produces equivalent projections |
| Failure semantics | ✅ | Stable error codes survive transport mapping |
| No parallel registry implementation | ✅ | Both adapters reach the same projection module |
| Registry immutability | ✅ | Negative-path evidence: no state transition through adapter surface |

---

## What M1-C Did NOT Demonstrate

The following are explicitly out of scope for M1-C and remain deferred:

| Item | Status | Reference |
|------|--------|-----------|
| Capability qualification execution | Not demonstrated | Separate authority contract |
| Authorization/permission granting | Not demonstrated | Separate authority contract |
| Operational-mode decision authority | Not demonstrated | M1-D, F-3 |
| Registry mutation | Not demonstrated | Observation ≠ mutation |
| F-2 security classification storage resolution | Not resolved | `M1-FINDING-SECURITY-CLASSIFICATION-SCHEMA-001` |
| F-3 operational-mode implementation | Not resolved | M1-D |
| Schema evolution | Not resolved | Schema frozen |
| JSON-file service migration | Not resolved | Separate migration contract |

**Important distinction:** M1 is not a claim that Rust can qualify or authorize capabilities. M1 has established the observation side of that boundary. Qualification execution and authority are materially different questions with their own contract boundaries.

---

## Boundary Integrity

| Property | Status |
|----------|--------|
| Contract-first ordering | ✅ Preserved |
| Surface lock before implementation | ✅ Preserved |
| Single projection module (no parallel paths) | ✅ Preserved |
| Transport ≠ Semantic | ✅ Preserved |
| Adapter ≠ Authority | ✅ Preserved |
| Observation ≠ Mutation | ✅ Preserved |
| MCP ≡ HTTP (cross-adapter equivalence) | ✅ Preserved |
| Registry immutability | ✅ Preserved |

---

## Explicit Exclusions

| Item | Status | Note |
|------|--------|------|
| Qualification execution | Out of scope | Separate authority contract |
| Authorization/permission | Out of scope | Separate authority contract |
| Operational-mode decisions | Out of scope | M1-D |
| Registry mutation | Out of scope | Observation ≠ mutation |
| Schema evolution | Out of scope | Schema frozen |
| JSON-file service migration | Out of scope | Separate migration contract |
| Pagination | Deferred | Scale/concurrency phase |

---

## Authorization

M1-C is complete. The observation adapter boundary is sealed.

The next architectural question is whether and how Rust should implement qualification semantics — a materially different authority question that deserves its own contract boundary.
