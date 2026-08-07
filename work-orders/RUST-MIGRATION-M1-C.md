# RUST-MIGRATION-M1-C — Governed Registry Observation Adapters: Planning

**Status:** APPROVED (owner review, 2026-08-07 — all decisions approved as drafted)
**Epic:** EPIC-RUST-MIGRATION-1
**Phase:** 1 — M1-C (adapter qualification)
**Predecessor:** M1-B seal `8fd37ad` (observation projection boundary established)
**Gate:** RUST-MIGRATION-M1 gate M1-3 (adapter contract)
**Last Updated:** 2026-08-07

**Owner approval (verbatim positions):**
1. MCP tool naming — **Approve `registry.observe_*`.** The `observe_` prefix preserves the semantic boundary. It avoids implying ownership (`manage_*`), authority (`control_*`), or mutation (`update_*`). Five tools: `registry.observe_capability`, `registry.observe_versions`, `registry.observe_dependencies`, `registry.observe_types`, `registry.observe_overview`.
2. HTTP route prefix — **Approve `/registry/observe/*`.** Routes: `GET /registry/observe/capabilities`, `GET /registry/observe/capabilities/{id}`, `GET /registry/observe/versions/{id}`, `GET /registry/observe/dependencies/{id}`, `GET /registry/observe/types`, `GET /registry/observe/overview`. Avoid `/api/registry/*`, `/registry/*`, `/admin/registry/*`.
3. Existing JSON-file services — **Remain parallel initially.** Do not migrate in M1-C. Replacing them would combine adapter migration with storage/service migration, violating migration discipline. M1-C non-goal: existing JSON-backed services are not removed, rewritten, or redirected during M1-C.
4. Error format — **Approve structured JSON.** `{ "code": "...", "message": "...", "registry_identity": "...", "observed_at": "..." }` where available. Error codes: `REGISTRY_IDENTITY_UNAVAILABLE`, `PROJECTION_SNAPSHOT_FAILED`, `REGISTRY_NOT_INITIALIZED`, `UNSUPPORTED_OBSERVATION`, `CAPABILITY_NOT_FOUND`, `INVARIANT_VIOLATION`.
5. Pagination — **Deferred.** No pagination in M1-C. Pagination introduces additional semantics (ordering guarantees, cursor identity, snapshot continuation, partial observation behavior) that belong to a later scale/concurrency phase.

**Approved implementation sequence:** adapter contracts → owner review → canonical lock → manifest/drift guard → MCP adapter → HTTP adapter → evidence + seal.
**Hard boundaries retained from M1-B:** no schema/DDL changes · no `RegistryStore` · no registry mutation · no qualification execution · no policy evaluation · no independent operational-mode decisions · no authority methods.
**New M1-C boundaries:** adapter output ≡ projection output · transport identity ≠ registry identity · adapters MUST consume projection module, MUST NOT bypass it.
**Evidence target:** `Projection Module Output ≡ MCP Adapter Output ≡ HTTP Adapter Output` — not old JSON service equivalence.

---

## 0. Scope of This Artifact

Planning work order for M1-C. Defines the adapter contract surface connecting existing transport layers (MCP, HTTP) to the governed projection module (M1-B).

**This is not "adding connectivity."** MCP and networking substrates already exist. M1-C is a governed adapter qualification phase: existing and future transport layers consume the projection module through a locked contract surface.

**M1-C non-goal:** Existing JSON-backed registry services (`RegistryCandidateService`, `RegistryOwnerService`, `RegistryEnforcementService`, etc.) are not removed, rewritten, or redirected during M1-C. Migration of those services requires a separate ownership and equivalence decision.

---

## 1. Current State

### 1.1 Projection Module (M1-B, sealed)

`RegistryObservationState` in `librarian-node/src/registry_observation.rs` — 5 read projections:

| Method | Returns | Envelope |
|--------|---------|----------|
| `capability(id)` | `CapabilityObservation` | `RegistryObservationEnvelope<CapabilityObservation>` |
| `capability_versions(id)` | `Vec<CapabilityVersionRecord>` | `RegistryObservationEnvelope<Vec<CapabilityVersionRecord>>` |
| `capability_dependencies(id)` | `Vec<CapabilityDependency>` | `RegistryObservationEnvelope<Vec<CapabilityDependency>>` |
| `capability_types()` | `Vec<CapabilityTypeDefinition>` | `RegistryObservationEnvelope<Vec<CapabilityTypeDefinition>>` |
| `registry_overview()` | `RegistryOverview` | `RegistryObservationEnvelope<RegistryOverview>` |

All projections:
- Read from `capability-registry.sqlite` (frozen governed schema)
- Use consistent-snapshot transactions (`PRAGMA query_only`, single `BEGIN…COMMIT`)
- Wrap output in `RegistryObservationEnvelope<T>` with `node_id`, `registry_identity`, `projection_observed_at`
- Serialize via `serde_json` (JSON)
- Fail closed on unknown enum values, absent/empty/missing-key identity, unrepresentable active_version

### 1.2 Existing MCP Surface

`RegistryMcpService` in `librarian-node/src/node/registry_mcp_service.rs` — 5 MCP tools:

| Tool | Access Pattern | Data Source |
|------|---------------|-------------|
| `registry.inspect_node` | Read-only | `NodeIdentityService` (JSON file) |
| `registry.query_candidates` | Read-only | `RegistryCandidateService` (JSON file) |
| `registry.retrieve_evidence` | Read-only | `RegistryCandidateService` (JSON file) |
| `registry.submit_review` | Write (owner action) | `OwnerWorkflowService` |
| `registry.request_action` | Write (owner action) | `OwnerWorkflowService` |

**None of these tools consume the projection module.** All data flows through JSON-file-backed in-memory services.

### 1.3 Existing HTTP Surface

Registry-related HTTP routes in `librarian-node/src/server.rs`:

| Route Group | Routes | Data Source |
|-------------|--------|-------------|
| `/registry/candidate/*` | discover, evidence/collect, submit, review, list, get, evidence, expire | `RegistryCandidateService` (JSON file) |
| `/registry/enforcement/*` | policy get/put, events | `RegistryEnforcementService` (JSON file) |
| `/registry/owner/*` | action create/approve/reject, pending, history | `RegistryOwnerService` (JSON file) |
| `/registry/apply/*` | propose, approve, apply, verify, reject, pending, history | `RegistryApplyService` (JSON file) |
| `/registry/mcp/*` | catalog, execute | `RegistryMcpService` |
| `/registry/*` | health, cleanup, version | Multiple JSON-file services |
| `/node/capabilities/*` | capabilities, evidence, lifecycle, unverified | `CapabilityEvidenceBridge` (JSON file) |

**None of these routes consume the projection module.** All registry data flows through JSON-file-backed services.

### 1.4 Gap

The governed projection module (M1-B) and the transport adapters (MCP/HTTP) are **disconnected**. The projection module reads from governed SQLite; the adapters read from JSON files. M1-C bridges this gap by qualifying adapters to consume projection output.

---

## 2. Adapter Contract Surface

### 2.1 What Adapters Expose

**Projection envelopes only.** An adapter exposes the exact output of the projection module — `RegistryObservationEnvelope<T>` serialized as JSON. No direct SQLite access. No alternate serialization semantics. No subset/superset of the projection payload.

| Adapter Exposure | Source | Constraint |
|------------------|--------|------------|
| `CapabilityObservation` | `capability(id)` | Full envelope, field-for-field |
| `Vec<CapabilityVersionRecord>` | `capability_versions(id)` | Full envelope, ascending, body excluded |
| `Vec<CapabilityDependency>` | `capability_dependencies(id)` | Full envelope, locked struct |
| `Vec<CapabilityTypeDefinition>` | `capability_types()` | Full envelope, ordered by CategoryOrder |
| `RegistryOverview` | `registry_overview()` | Full envelope, 11 keys, zero-count groups present |

### 2.2 Transport Identity Separation

**Invariant:** Transport identity ≠ registry identity.

| Identity Layer | Source | Semantics |
|----------------|--------|-----------|
| `registry_identity` | `capability_registry_meta` (SHA-256) | Content-derived; tracks governed registry state |
| `node_id` | `RegistryObservationState::new()` | Sealed at construction; identifies the observing node |
| `projection_observed_at` | `SystemTime::now()` | RFC 3339 UTC; when the projection was read |
| MCP session identity | MCP transport layer | Transient; identifies the MCP session |
| Network connection identity | HTTP/TCP layer | Transient; identifies the network connection |

**MUST NOT:** transport metadata (session ID, connection ID, request context) appears in `registry_identity`, `node_id`, or any projection field. Transport identity is metadata about the adapter, not about the registry.

### 2.3 MCP Tool Definitions

Five tools mapping 1:1 to projection methods:

| MCP Tool Name | Projection Method | Input | Output |
|---------------|-------------------|-------|--------|
| `registry.observe_capability` | `capability(id)` | `{"capability_id": "..."}` | `RegistryObservationEnvelope<CapabilityObservation>` |
| `registry.observe_versions` | `capability_versions(id)` | `{"capability_id": "..."}` | `RegistryObservationEnvelope<Vec<CapabilityVersionRecord>>` |
| `registry.observe_dependencies` | `capability_dependencies(id)` | `{"capability_id": "..."}` | `RegistryObservationEnvelope<Vec<CapabilityDependency>>` |
| `registry.observe_types` | `capability_types()` | `{}` | `RegistryObservationEnvelope<Vec<CapabilityTypeDefinition>>` |
| `registry.observe_overview` | `registry_overview()` | `{}` | `RegistryObservationEnvelope<RegistryOverview>` |

**Naming convention:** `registry.observe_*` — communicates observation semantics, not data-access semantics. Avoid `registry.get_*`, `registry.list_*`, `registry.query_*`.

### 2.4 HTTP Route Definitions

Six routes mapping 1:1 to projection methods:

| HTTP Route | Method | Projection Method | Input | Output |
|------------|--------|-------------------|-------|--------|
| `GET /registry/observe/capabilities` | GET | `capability_types()` | None | `RegistryObservationEnvelope<Vec<CapabilityTypeDefinition>>` |
| `GET /registry/observe/capabilities/{id}` | GET | `capability(id)` | Path: `id` | `RegistryObservationEnvelope<CapabilityObservation>` |
| `GET /registry/observe/versions/{id}` | GET | `capability_versions(id)` | Path: `id` | `RegistryObservationEnvelope<Vec<CapabilityVersionRecord>>` |
| `GET /registry/observe/dependencies/{id}` | GET | `capability_dependencies(id)` | Path: `id` | `RegistryObservationEnvelope<Vec<CapabilityDependency>>` |
| `GET /registry/observe/types` | GET | `capability_types()` | None | `RegistryObservationEnvelope<Vec<CapabilityTypeDefinition>>` |
| `GET /registry/observe/overview` | GET | `registry_overview()` | None | `RegistryObservationEnvelope<RegistryOverview>` |

**Route prefix:** `/registry/observe/*` — communicates observation projection, not registry administration. Avoid `/api/registry/*`, `/registry/*`, `/admin/registry/*`.

### 2.5 Error Contract

Structured JSON error response:

```json
{
  "code": "REGISTRY_OBSERVATION_FAILED",
  "message": "human readable explanation"
}
```

When projection identity is available (partial failure):

```json
{
  "code": "REGISTRY_OBSERVATION_FAILED",
  "message": "human readable explanation",
  "registry_identity": "...",
  "observed_at": "..."
}
```

**Stable error codes:**

| Code | Meaning |
|------|---------|
| `REGISTRY_IDENTITY_UNAVAILABLE` | Registry identity cannot be computed (absent/empty meta) |
| `PROJECTION_SNAPSHOT_FAILED` | Consistent-snapshot read failed |
| `REGISTRY_NOT_INITIALIZED` | Registry database not available at expected path |
| `UNSUPPORTED_OBSERVATION` | Requested observation type not supported |
| `CAPABILITY_NOT_FOUND` | Requested capability ID not in registry |
| `INVARIANT_VIOLATION` | Fail-closed: unknown enum, missing key, unrepresentable value |

**Restrictions:** error metadata may describe observation failure; error metadata must not introduce transport identity as registry identity.

### 2.6 Pagination

**Deferred.** No pagination in M1-C. The current projection contract defines deterministic bounded observations. Pagination introduces additional semantics (ordering guarantees, cursor identity, snapshot continuation, partial observation behavior) that belong to a later scale/concurrency phase.

Pagination will be reconsidered when registry cardinality requirements demonstrate the need for partitioned observation.

---

## 3. Adapter Invariants

### 3.1 Transport Separation

| Invariant | Statement |
|-----------|-----------|
| Transport Identity ≠ Registry Identity | MCP session ID / network connection ID must not enter registry semantics |
| Session Identity ≠ Capability Identity | MCP session is transient; capability identity is governed |
| Connection Identity ≠ Authority | Network connection does not grant registry authority |

### 3.2 Adapter Constraints

**Adapters MUST:**
- Consume projection module output (`RegistryObservationState` methods)
- Preserve projection semantics (field names, types, ordering)
- Preserve failure behavior (fail-closed error codes and messages)
- Preserve provenance fields (`registry_identity`, `projection_observed_at`, `node_id`)
- Serialize projection output as JSON matching `serde_json::to_value(&envelope)`
- Return full `RegistryObservationEnvelope<T>` (no truncation, no subsetting)

**Adapters MUST NOT:**
- Query registry storage directly (SQLite access is forbidden for adapters)
- Reconstruct projections from raw data
- Apply qualification logic
- Evaluate policies
- Mutate registry state
- Transform, filter, or reorder projection fields
- Add fields not present in the projection envelope
- Remove fields present in the projection envelope
- Reinterpret enum values
- Cache projection output across snapshot boundaries
- Create authority through transport exposure

---

## 4. Evidence Target

**Projection Module Output ≡ MCP Adapter Output ≡ HTTP Adapter Output.**

Not: `Old JSON Service ≡ New Registry Projection` (that is a future migration question).

For every adapter tool/route:
1. The projection module produces an `RegistryObservationEnvelope<T>`
2. The adapter serializes it as JSON
3. The serialized JSON MUST be byte-identical to `serde_json::to_value(&envelope)` (modulo transport framing)

No semantic drift between local module and transport consumer. The adapter is a passthrough, not a transformer.

---

## 5. Implementation Sequence

### M1-C0 — Adapter Contracts (planning only)

**Scope:** Lock the adapter contract surface.
- MCP tool definitions (names, inputs, outputs) — locked above
- HTTP route definitions (paths, methods, request/response) — locked above
- Error handling contract (codes, messages) — locked above
- Transport identity separation rules — locked above
- Manifest/drift guard for contract surface
- Owner review and approval

**No implementation code.** This is a planning artifact.

### M1-C1 — MCP Adapter Implementation

**Scope:** Wire MCP tools to projection module.
- Implement `registry.observe_*` tools
- Connect to `RegistryObservationState`
- Verify adapter output ≡ projection output (equivalence tests)
- Evidence: MCP tool responses byte-identical to projection JSON

### M1-C2 — HTTP Adapter Implementation

**Scope:** Wire HTTP routes to projection module.
- Implement `GET /registry/observe/*` routes
- Connect to `RegistryObservationState`
- Verify adapter output ≡ projection output (equivalence tests)
- Evidence: HTTP responses byte-identical to projection JSON

### M1-C3 — Evidence + Seal

**Scope:** Complete evidence package and seal M1-C.
- Adapter equivalence test receipts
- Transport identity separation verification
- Fail-closed error behavior verification
- Completion record

---

## 6. Hard Boundaries (retained + new)

| Boundary | Status | Phase |
|----------|--------|-------|
| No schema/DDL changes | Retained | M1-B |
| No `RegistryStore` | Retained | M1-B |
| No registry mutation | Retained | M1-B |
| No qualification execution | Retained | M1-B |
| No policy evaluation | Retained | M1-B |
| No independent operational-mode decisions | Retained | M1-B |
| No authority methods | Retained | M1-B |
| Adapter output ≡ projection output | New | M1-C |
| Transport identity ≠ registry identity | New | M1-C |
| Adapters MUST consume projection module | New | M1-C |
| Adapters MUST NOT bypass projection module | New | M1-C |

---

## 7. Non-Goals (M1-C)

| Item | Status | Note |
|------|--------|------|
| Migration of JSON-file-backed services | Explicit non-goal | Requires separate ownership and equivalence decision |
| Pagination | Deferred | Scale/concurrency phase |
| New transport substrates | Out of scope | MCP and HTTP already exist |
| Registry authority introduction | Forbidden | Observation ≠ authority |

---

## 8. Authorization

M1-C planning is complete. The adapter contract surface is locked.

M1-C0 (contract manifest/drift guard) may proceed when the Owner authorizes it. No implementation before M1-C0 lock.
