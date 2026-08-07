# REGISTRY-OBSERVATION-ADAPTER-CONTRACT-001.md — Registry Observation Adapter Contract (M1-C transport boundary)

**Version:** 1.0.0
**Status:** Canonical (owner-approved lock, 2026-08-07)
**Sealed suite ID:** `REGISTRY-OBSERVATION-SCHEMA-001` (suite-aligned — the adapter contract belongs to the registry observation suite)
**Last Updated:** 2026-08-07

**Semantic parent:** `REGISTRY-OBSERVATION-CONTRACT-001` (Canonical, locked 2026-08-06)
**Implementation parent:** `RUNTIME-REGISTRY-OBSERVATION-CONTRACT-001` (Canonical, locked 2026-08-07 — M1-B projection boundary)
**Gate:** RUST-MIGRATION-M1 gate M1-3 (adapter contract) — M1-C0 artifact per owner directive

### Contract lineage

```
REGISTRY-OBSERVATION-SCHEMA-001               (frozen schema)
        |
        ├── REGISTRY-OBSERVATION-CONTRACT-001 (semantic — what registry observation IS)
        |
        └── REGISTRY-OBSERVATION-ADAPTER-CONTRACT-001 (adapter — how transports consume observations)
```

"Adapter" describes the consumer context, not the semantic contract. Both
documents share suite `REGISTRY-OBSERVATION-SCHEMA-001`; there is exactly one
registry observation contract lineage.

---

## 0. Purpose

Defines the **adapter boundary** through which MCP and HTTP transports consume
evidence-backed registry observation projections. This is the transport-side
surface for `RUNTIME-REGISTRY-OBSERVATION-CONTRACT-001`: it fixes *how* external
consumers access the projection module, without creating a new authority channel.

> The success criterion is not "MCP can call a tool" or "HTTP can serve a route."
> It is:
>
> **An external consumer can access governed registry state through a transport
> adapter — and no transport path can change that state, create authority, or
> bypass the projection boundary.**

The contract is the boundary, not the code. Adapter code follows this contract
only after it is locked (M1-C0 surface lock, then implementation).

### Core invariants (carried from REGISTRY-OBSERVATION-CONTRACT-001, preserved verbatim)

> The registry projection is evidence-backed state exposure, not a source of
> authority. It reports what the registry knows, from the governed registry
> database; it confers nothing.

Invariant chain (all five hold independently):

```
Registry        ≠ Authority      — projecting state does not create authority
Observation     ≠ Mutation       — seeing state is not changing state
Projection      ≠ Ownership      — reporting state is not owning state
Transport       ≠ Semantic       — adapters consume projections; they do not
                                   define observation semantics
Adapter         ≠ Authority      — transport exposure does not create registry authority
```

### Source authority statement

The projection module (`RegistryObservationState`) is the single source of
observation truth for transport adapters. Adapters MUST consume projection
output; Adapters MUST NOT bypass the projection module, query registry storage
directly, or reconstruct projections from raw data.

---

## 1. Scope

### 1.1 The adapter contract exposes (transport consumption of projections)

- **MCP tools** — five `registry.observe_*` tools mapping 1:1 to projection methods
- **HTTP routes** — five `GET /registry/observe/*` routes mapping 1:1 to projection methods
- **Error contract** — structured JSON error responses with stable machine-readable codes
- **Transport identity separation** — MCP session / network connection identity must not enter registry semantics

### 1.2 The adapter contract explicitly does NOT

- Introduce new projection methods beyond the five locked in M1-B
- Transform, filter, or reorder projection fields
- Add fields not present in the projection envelope
- Remove fields present in the projection envelope
- Reinterpret enum values (e.g., "passed" → "ok")
- Cache projection output across snapshot boundaries
- Migrate existing JSON-file-backed services
- Add pagination (deferred to scale/concurrency phase)
- Create registry authority through transport exposure

### 1.3 Boundary shape (correct)

```
External Consumer (MCP client / HTTP client)
        |
        v
Transport Adapter (MCP tools / HTTP routes)
        |  MUST consume projection envelope
        |  MUST NOT bypass projection module
        v
RegistryObservationState (projection module)
        |
        v
Governed Registry (SQLite, frozen schema)
```

### 1.4 Boundary shape (wrong — authority leak)

```
External Consumer
        |
        v
Transport Adapter
        |  queries SQLite directly  ← FORBIDDEN
        |  reconstructs projection  ← FORBIDDEN
        v
Governed Registry
```

---

## 2. MCP Tool Surface

### 2.1 Tool Definitions

Five tools mapping 1:1 to projection methods:

| Tool Name | Projection Method | Input | Output |
|-----------|-------------------|-------|--------|
| `registry.observe_capability` | `capability(id)` | `{"capability_id": "..."}` | `RegistryObservationEnvelope<CapabilityObservation>` |
| `registry.observe_versions` | `capability_versions(id)` | `{"capability_id": "..."}` | `RegistryObservationEnvelope<Vec<CapabilityVersionRecord>>` |
| `registry.observe_dependencies` | `capability_dependencies(id)` | `{"capability_id": "..."}` | `RegistryObservationEnvelope<Vec<CapabilityDependency>>` |
| `registry.observe_types` | `capability_types()` | `{}` | `RegistryObservationEnvelope<Vec<CapabilityTypeDefinition>>` |
| `registry.observe_overview` | `registry_overview()` | `{}` | `RegistryObservationEnvelope<RegistryOverview>` |

### 2.2 Naming Convention

`registry.observe_*` — communicates observation semantics, not data-access semantics.

**Forbidden naming patterns:**
- `registry.get_*` — implies generic retrieval
- `registry.query_*` — implies arbitrary querying
- `registry.manage_*` — implies authority
- `registry.inspect_*` — ambiguous
- `registry.read_*` — implies data-access semantics

### 2.3 Request Contract

- Request: JSON object with named parameters (matching projection method signatures)
- `capability_id`: string, required for capability/versions/dependencies tools
- No additional parameters for types/overview tools

### 2.4 Response Contract

- Response: Full `RegistryObservationEnvelope<T>` JSON (matching projection module output exactly)
- Content-Type: `application/json`
- No transport framing beyond JSON serialization

### 2.5 Error Response

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

**Restrictions:**
- Error metadata may describe observation failure
- Error metadata MUST NOT introduce transport identity as registry identity
- Error metadata MUST NOT contain MCP session IDs, network connection IDs, or request context

---

## 3. HTTP Route Surface

### 3.1 Route Definitions

Five routes mapping 1:1 to projection methods:

| Route | Method | Projection Method | Input | Output |
|-------|--------|-------------------|-------|--------|
| `GET /registry/observe/capabilities` | GET | (list overview types) | None | `RegistryObservationEnvelope<Vec<CapabilityTypeDefinition>>` |
| `GET /registry/observe/capabilities/{id}` | GET | `capability(id)` | Path: `id` | `RegistryObservationEnvelope<CapabilityObservation>` |
| `GET /registry/observe/versions/{id}` | GET | `capability_versions(id)` | Path: `id` | `RegistryObservationEnvelope<Vec<CapabilityVersionRecord>>` |
| `GET /registry/observe/dependencies/{id}` | GET | `capability_dependencies(id)` | Path: `id` | `RegistryObservationEnvelope<Vec<CapabilityDependency>>` |
| `GET /registry/observe/types` | GET | `capability_types()` | None | `RegistryObservationEnvelope<Vec<CapabilityTypeDefinition>>` |
| `GET /registry/observe/overview` | GET | `registry_overview()` | None | `RegistryObservationEnvelope<RegistryOverview>` |

### 3.2 Route Prefix

`/registry/observe/*` — communicates observation projection, not registry administration.

**Forbidden route prefixes:**
- `/api/registry/*` — implies broader API surface
- `/registry/*` — ambiguous, could imply administration
- `/admin/registry/*` — implies administrative authority

### 3.3 Request Contract

- Method: GET only (no POST/PUT/DELETE — observation is read-only)
- Path parameters: `id` (string, capability identifier)
- Query parameters: none (M1-C)
- Content-Type: none for GET requests

### 3.4 Response Contract

- Response: Full `RegistryObservationEnvelope<T>` JSON (matching projection module output exactly)
- Content-Type: `application/json`
- Status: 200 OK on success, 4xx/5xx on error

### 3.5 Error Response

Same structure as MCP errors (§2.5):

```json
{
  "code": "REGISTRY_OBSERVATION_FAILED",
  "message": "human readable explanation"
}
```

HTTP status mapping:

| Error Code | HTTP Status |
|------------|-------------|
| `REGISTRY_IDENTITY_UNAVAILABLE` | 503 Service Unavailable |
| `PROJECTION_SNAPSHOT_FAILED` | 500 Internal Server Error |
| `REGISTRY_NOT_INITIALIZED` | 503 Service Unavailable |
| `UNSUPPORTED_OBSERVATION` | 404 Not Found |
| `CAPABILITY_NOT_FOUND` | 404 Not Found |
| `INVARIANT_VIOLATION` | 500 Internal Server Error |

---

## 4. Transport Identity Separation

### 4.1 Identity Layers

| Identity Layer | Source | Semantics |
|----------------|--------|-----------|
| `registry_identity` | `capability_registry_meta` (SHA-256) | Content-derived; tracks governed registry state |
| `node_id` | `RegistryObservationState::new()` | Sealed at construction; identifies the observing node |
| `projection_observed_at` | `SystemTime::now()` | RFC 3339 UTC; when the projection was read |
| MCP session identity | MCP transport layer | Transient; identifies the MCP session |
| Network connection identity | HTTP/TCP layer | Transient; identifies the network connection |

### 4.2 Separation Invariants

| Invariant | Statement |
|-----------|-----------|
| Transport Identity ≠ Registry Identity | MCP session ID / network connection ID must not enter registry semantics |
| Session Identity ≠ Capability Identity | MCP session is transient; capability identity is governed |
| Connection Identity ≠ Authority | Network connection does not grant registry authority |

### 4.3 Forbidden Identity Injections

Adapters MUST NOT:
- Include MCP session IDs in `registry_identity` or `node_id`
- Include network connection IDs in projection fields
- Include HTTP request context in error metadata
- Use transport identity to influence projection output
- Cache projection output keyed by transport identity

---

## 5. Adapter Constraints

### 5.1 Adapter MUST

- Consume projection module output (`RegistryObservationState` methods)
- Preserve projection semantics (field names, types, ordering)
- Preserve failure behavior (fail-closed error codes and messages)
- Preserve provenance fields (`registry_identity`, `projection_observed_at`, `node_id`)
- Serialize projection output as JSON matching `serde_json::to_value(&envelope)`
- Return full `RegistryObservationEnvelope<T>` (no truncation, no subsetting)

### 5.2 Adapter MUST NOT

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

### 5.3 Evidence Requirement

**Adapter output ≡ projection module output.**

For every adapter tool/route:
1. The projection module produces an `RegistryObservationEnvelope<T>`
2. The adapter serializes it as JSON
3. The serialized JSON MUST be byte-identical to `serde_json::to_value(&envelope)` (modulo transport framing)

No semantic drift between local module and transport consumer. The adapter is a passthrough, not a transformer.

---

## 6. Explicit Exclusions

### 6.1 JSON-File Service Migration

Existing JSON-backed services (`RegistryCandidateService`, `RegistryOwnerService`, `RegistryEnforcementService`, etc.) remain legacy parallel consumers until an explicit migration contract exists. M1-C establishes governed projection consumption, not service replacement.

**M1-C non-goal:** JSON-file services are not removed, rewritten, or redirected during M1-C. Migration of those services requires a separate ownership and equivalence decision.

### 6.2 Pagination

Pagination is deferred. The current projection contract defines deterministic bounded observations. Pagination introduces additional semantics (ordering guarantees, cursor identity, snapshot continuation, partial observation behavior) that belong to a later scale/concurrency phase.

Pagination will be reconsidered when registry cardinality requirements demonstrate the need for partitioned observation.

### 6.3 New Projection Methods

No new projection methods beyond the five locked in M1-B. Additional observations require a separate contract extension through the established migration discipline.

---

## 7. Error Semantics

### 7.1 Error Code Stability

Error codes are stable machine-readable identifiers. They MUST NOT change between versions. New error codes may be added; existing error codes MUST NOT be renamed or removed.

### 7.2 Error Message Semantics

Error messages are human-readable explanations. They SHOULD be descriptive enough to diagnose the failure without consulting source code. They MUST NOT contain:
- Internal storage paths
- SQL query text
- Stack traces
- Transport-specific metadata (session IDs, connection IDs)

### 7.3 Error Metadata Semantics

Error metadata (`registry_identity`, `observed_at`) is optional and MAY be included when the projection module can provide it. Error metadata MUST NOT contain:
- Transport identity (MCP session ID, network connection ID)
- Request context (HTTP method, path, headers)
- Internal service state

---

## 8. Manifest and Drift Guard

### 8.1 Contract Surface Manifest

The adapter contract surface is pinned in `contracts/capability/contract-surface-manifest.json` with:
- 5 MCP tool definitions
- 5 HTTP route definitions
- 6 error codes
- Transport identity separation rules

### 8.2 Drift Guard

The drift guard checks:
- MCP tool names match the locked definitions
- HTTP routes match the locked definitions
- Error codes match the locked definitions
- No unauthorized projection methods are exposed

---

## 9. Authorization

This contract is the adapter boundary for registry observation. It must be locked before any adapter implementation begins.

M1-C0 lock establishes this contract surface. M1-C1 (MCP adapter) and M1-C2 (HTTP adapter) may proceed after this lock.
